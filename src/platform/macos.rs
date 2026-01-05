use async_trait::async_trait;
use std::process::Command;

use crate::core::{Device, DiskError, FileSystemType};
use crate::core::disk_ops::DiskManager;

pub struct MacOSDiskManager;

impl MacOSDiskManager {
    pub fn new() -> Self {
        Self
    }

    /// Parse diskutil list -plist output to get devices
    fn parse_diskutil_output(&self, output: &str) -> Result<Vec<Device>, DiskError> {
        let plist: plist::Value = plist::from_bytes(output.as_bytes())
            .map_err(|e| DiskError::ParseError(e.to_string()))?;

        let mut devices = Vec::new();

        // Get AllDisksAndPartitions array
        let all_disks = plist
            .as_dictionary()
            .and_then(|d| d.get("AllDisksAndPartitions"))
            .and_then(|v| v.as_array())
            .ok_or_else(|| DiskError::ParseError("Missing AllDisksAndPartitions".to_string()))?;

        for disk in all_disks {
            let disk_dict = match disk.as_dictionary() {
                Some(d) => d,
                None => continue,
            };

            // Get disk identifier (e.g., disk0)
            let device_identifier = disk_dict
                .get("DeviceIdentifier")
                .and_then(|v| v.as_string())
                .unwrap_or("unknown");

            // Get disk size
            let size_bytes = disk_dict
                .get("Size")
                .and_then(|v| v.as_unsigned_integer())
                .unwrap_or(0);

            // Get content type
            let content = disk_dict
                .get("Content")
                .and_then(|v| v.as_string())
                .unwrap_or("Unknown");

            // Heuristic for system disk: disk0 or contains the root mount point
            let mut is_system = device_identifier == "disk0";

            // Check partitions to see if any are the root mount point
            if let Some(partitions) = disk_dict.get("Partitions").and_then(|v| v.as_array()) {
                for partition in partitions {
                    if let Some(part_dict) = partition.as_dictionary() {
                        let mount_point = part_dict
                            .get("MountPoint")
                            .and_then(|v| v.as_string());

                        if mount_point == Some("/") {
                            is_system = true;
                        }
                    }
                }
            }

            // Only add the whole disk entry if it's not a partition (e.g., skip disk0s1, disk1s2)
            // macOS partition identifiers usually end with 's' and a number
            let is_partition = device_identifier.contains('s') && 
                device_identifier.split('s').last().map_or(false, |s| s.chars().all(|c| c.is_ascii_digit()));

            if size_bytes > 0 && !is_partition {
                devices.push(Device {
                    path: format!("/dev/{}", device_identifier),
                    name: format!("Disk {}", device_identifier),
                    size_bytes,
                    filesystem: content.to_string(),
                    label: device_identifier.to_string(),
                    mount_point: None,
                    is_protected: is_system,
                    is_removable: !is_system,
                });
            }
        }

        Ok(devices)
    }
}

#[async_trait]
impl DiskManager for MacOSDiskManager {
    async fn list_devices(&self) -> Result<Vec<Device>, DiskError> {
        let output = Command::new("diskutil")
            .args(["list", "-plist"])
            .output()?;

        if !output.status.success() {
            return Err(DiskError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        self.parse_diskutil_output(&stdout)
    }

    async fn unmount(&self, path: &str) -> Result<(), DiskError> {
        if !self.has_privileges() {
            return Err(DiskError::InsufficientPrivileges);
        }

        let output = Command::new("diskutil")
            .args(["unmountDisk", path])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("busy") {
                return Err(DiskError::DeviceBusy);
            }
            return Err(DiskError::CommandFailed(stderr.to_string()));
        }

        Ok(())
    }

    async fn format(
        &self,
        path: &str,
        fs_type: FileSystemType,
        label: &str,
    ) -> Result<(), DiskError> {
        if !self.has_privileges() {
            return Err(DiskError::InsufficientPrivileges);
        }

        // Extract disk identifier from path (e.g., /dev/disk2 -> disk2, /dev/disk2s1 -> disk2s1)
        let identifier = path
            .strip_prefix("/dev/")
            .ok_or_else(|| DiskError::DeviceNotFound(path.to_string()))?;

        // Always extract parent disk - eraseDisk requires whole disk identifier
        // disk4s1 -> disk4, disk4 -> disk4 (unchanged if already whole disk)
        let target_disk = extract_parent_disk(identifier);

        let output = Command::new("diskutil")
            .args([
                "eraseDisk",
                fs_type.as_diskutil_format(),
                label,
                &target_disk,
            ])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("busy") {
                return Err(DiskError::DeviceBusy);
            }
            return Err(DiskError::CommandFailed(stderr.to_string()));
        }

        Ok(())
    }

    fn has_privileges(&self) -> bool {
        unsafe { libc::getuid() == 0 }
    }
}

/// Extract parent disk from partition identifier
/// e.g., disk4s1 -> disk4, disk4s2 -> disk4, disk0s1 -> disk0
fn extract_parent_disk(identifier: &str) -> String {
    // Find the position of 's' that follows a digit (partition separator)
    let bytes = identifier.as_bytes();
    for i in (1..bytes.len()).rev() {
        if bytes[i] == b's' && bytes[i - 1].is_ascii_digit() {
            return identifier[..i].to_string();
        }
    }
    // No partition separator found, return as-is
    identifier.to_string()
}
