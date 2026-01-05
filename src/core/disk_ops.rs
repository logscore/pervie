use async_trait::async_trait;

use super::{Device, DiskError, FileSystemType};

/// Trait for platform-specific disk operations
#[async_trait]
pub trait DiskManager: Send + Sync {
    /// Scans system for block devices
    async fn list_devices(&self) -> Result<Vec<Device>, DiskError>;

    /// Unmounts the device at the specified path
    async fn unmount(&self, path: &str) -> Result<(), DiskError>;

    /// Formats the device with the specified filesystem and label
    async fn format(
        &self,
        path: &str,
        fs_type: FileSystemType,
        label: &str,
    ) -> Result<(), DiskError>;

    /// Checks if running with elevated privileges (root/admin)
    fn has_privileges(&self) -> bool;
}
