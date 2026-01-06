use async_trait::async_trait;
use serde::Deserialize;
use std::process::Command;

use crate::core::disk_ops::DiskManager;
use crate::core::{Device, DiskError, FileSystemType};

/// Linux-specific disk manager using lsblk and standard Linux tools
pub struct LinuxDiskManager;

impl LinuxDiskManager {
    pub fn new() -> Self {
        Self
    }

    /// Parse lsblk JSON output to get devices
    fn parse_lsblk_output(&self, output: &str) -> Result<Vec<Device>, DiskError> {
        let lsblk: LsblkOutput =
            serde_json::from_str(output).map_err(|e| DiskError::ParseError(e.to_string()))?;

        let mut devices = Vec::new();

        // Get root mount device to mark as protected
        let root_device = self.get_root_device();

        for block in lsblk.blockdevices {
            // Skip loop devices and other non-physical devices
            if block.name.starts_with("loop") || block.name.starts_with("ram") {
                continue;
            }

            let is_disk = block.device_type == "disk" || block.device_type == "rom";
            let path = block
                .path
                .clone()
                .unwrap_or_else(|| format!("/dev/{}", block.name));

            // Check if this is the root device or contains the root partition
            let is_root_device = root_device
                .as_ref()
                .map(|rd| path.contains(rd) || rd.contains(&block.name))
                .unwrap_or(false);

            // Skip anything that looks like a partition (e.g. sda1, nvme0n1p1) if it's not a whole disk
            // Note: lsblk --json usually shows partitions as children.
            // If it's a 'part' type, we skip it.
            if block.device_type == "part" {
                continue;
            }

            // Only add the whole disk entry if it's a disk/rom and has a size
            if is_disk {
                let size = parse_size(&block.size);
                if size > 0 {
                    devices.push(Device {
                        path: path.clone(),
                        name: format!("Disk {}", block.name),
                        size_bytes: size,
                        filesystem: block
                            .fstype
                            .clone()
                            .unwrap_or_else(|| "Unknown".to_string()),
                        label: block.label.clone().unwrap_or_else(|| block.name.clone()),
                        mount_point: block.mountpoint.clone(),
                        is_protected: is_root_device,
                        is_removable: block.rm.unwrap_or(false),
                    });
                }
            }
        }

        Ok(devices)
    }

    /// Get the device containing the root filesystem
    fn get_root_device(&self) -> Option<String> {
        // Try to read from /proc/cmdline or use findmnt
        let output = Command::new("findmnt")
            .args(["-n", "-o", "SOURCE", "/"])
            .output()
            .ok()?;

        if output.status.success() {
            let source = String::from_utf8_lossy(&output.stdout).trim().to_string();
            // Extract base device (e.g., /dev/sda1 -> sda, /dev/nvme0n1p1 -> nvme0n1)
            Some(source)
        } else {
            None
        }
    }
}

/// Structures for parsing lsblk JSON output
#[derive(Debug, Deserialize)]
struct LsblkOutput {
    blockdevices: Vec<BlockDevice>,
}

#[derive(Debug, Deserialize)]
struct BlockDevice {
    name: String,
    size: String,
    #[serde(rename = "type")]
    device_type: String,
    fstype: Option<String>,
    label: Option<String>,
    mountpoint: Option<String>,
    path: Option<String>,
    rm: Option<bool>,
    children: Option<Vec<BlockDevice>>,
}

/// Parse size string from lsblk (e.g., "500G", "1T", "256M") to bytes
fn parse_size(size_str: &str) -> u64 {
    let size_str = size_str.trim();
    if size_str.is_empty() {
        return 0;
    }

    let (num_str, suffix) = size_str.split_at(size_str.len().saturating_sub(1));
    let multiplier: u64 = match suffix.to_uppercase().as_str() {
        "B" => 1,
        "K" => 1024,
        "M" => 1024 * 1024,
        "G" => 1024 * 1024 * 1024,
        "T" => 1024 * 1024 * 1024 * 1024,
        "P" => 1024 * 1024 * 1024 * 1024 * 1024,
        _ => {
            // No suffix, try to parse as bytes
            return size_str.parse().unwrap_or(0);
        }
    };

    num_str
        .trim()
        .parse::<f64>()
        .map(|n| (n * multiplier as f64) as u64)
        .unwrap_or(0)
}

#[async_trait]
impl DiskManager for LinuxDiskManager {
    async fn list_devices(&self) -> Result<Vec<Device>, DiskError> {
        let output = Command::new("lsblk")
            .args([
                "--json",
                "-o",
                "NAME,SIZE,TYPE,FSTYPE,LABEL,MOUNTPOINT,PATH,RM",
            ])
            .output()?;

        if !output.status.success() {
            return Err(DiskError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        self.parse_lsblk_output(&stdout)
    }

    async fn unmount(&self, path: &str) -> Result<(), DiskError> {
        if !self.has_privileges() {
            return Err(DiskError::InsufficientPrivileges);
        }

        let output = Command::new("umount").arg(path).output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("busy") || stderr.contains("target is busy") {
                return Err(DiskError::DeviceBusy);
            }
            if stderr.contains("not mounted") {
                // Already unmounted, treat as success
                return Ok(());
            }
            return Err(DiskError::CommandFailed(stderr.to_string()));
        }

        Ok(())
    }

    async fn eject(&self, path: &str) -> Result<(), DiskError> {
        if !self.has_privileges() {
            return Err(DiskError::InsufficientPrivileges);
        }

        let output = Command::new("eject").arg(path).output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
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

        let (cmd, args) = match fs_type {
            FileSystemType::Fat32 => ("mkfs.vfat", vec!["-F", "32", "-n", label, path]),
            FileSystemType::ExFat => ("mkfs.exfat", vec!["-n", label, path]),
            FileSystemType::Ntfs => ("mkfs.ntfs", vec!["-f", "-L", label, path]),
            FileSystemType::Ext4 => ("mkfs.ext4", vec!["-L", label, path]),
            FileSystemType::Apfs => {
                return Err(DiskError::UnsupportedFilesystem(
                    "APFS is not supported on Linux".to_string(),
                ));
            }
        };

        let output = Command::new(cmd).args(&args).output()?;

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
        Command::new("id")
            .arg("-u")
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).trim() == "0")
            .unwrap_or(false)
    }
}
