#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;

use std::sync::Arc;

use crate::core::disk_ops::DiskManager;

/// Get the appropriate DiskManager for the current platform
pub fn get_disk_manager() -> Arc<dyn DiskManager> {
    #[cfg(target_os = "macos")]
    {
        Arc::new(macos::MacOSDiskManager::new())
    }

    #[cfg(target_os = "linux")]
    {
        Arc::new(linux::LinuxDiskManager::new())
    }
}
