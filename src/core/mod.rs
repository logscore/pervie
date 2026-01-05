pub mod disk_ops;
pub mod flasher;

use self::flasher::FlashProgress;

use thiserror::Error;

/// Represents a block storage device
#[derive(Debug, Clone)]
pub struct Device {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
    pub filesystem: String,
    pub label: String,
    pub mount_point: Option<String>,
    pub is_protected: bool,
    pub is_removable: bool,
}

/// Application state machine
#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Idle,
    DeviceSelected(usize),
    FormattingMenu,
    ConfirmDestructive(String),
    ConfirmFlash(String),
    IsoSelection,
    Flashing(FlashProgress),
    InProgress(String),
    Error(String),
    Success(String),
}

/// Represents an ISO image available for flashing
#[derive(Debug, Clone)]
pub struct Iso {
    pub name: String,
    pub version: String,
    pub arch: String,
    pub url: String,
    pub variety: String,
}

/// Supported filesystem types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileSystemType {
    Fat32,
    ExFat,
    Ntfs,
    Ext4,
    Apfs,
}

impl FileSystemType {
    /// Get the filesystem name as used by diskutil
    pub fn as_diskutil_format(&self) -> &'static str {
        match self {
            FileSystemType::Fat32 => "FAT32",
            FileSystemType::ExFat => "ExFAT",
            FileSystemType::Ntfs => "NTFS",
            FileSystemType::Ext4 => "ExFAT", // Not directly supported, fallback
            FileSystemType::Apfs => "APFS",
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            FileSystemType::Fat32 => "FAT32",
            FileSystemType::ExFat => "exFAT",
            FileSystemType::Ntfs => "NTFS",
            FileSystemType::Ext4 => "ext4",
            FileSystemType::Apfs => "APFS",
        }
    }

    /// Get available filesystems for macOS
    pub fn macos_options() -> Vec<FileSystemType> {
        vec![
            FileSystemType::Apfs,
            FileSystemType::ExFat,
            FileSystemType::Fat32,
        ]
    }

    /// Get available filesystems for Linux
    pub fn linux_options() -> Vec<FileSystemType> {
        vec![
            FileSystemType::Ext4,
            FileSystemType::ExFat,
            FileSystemType::Fat32,
            FileSystemType::Ntfs,
        ]
    }
}

/// Errors that can occur during disk operations
#[derive(Error, Debug)]
pub enum DiskError {
    #[error("Device is protected (system drive)")]
    ProtectedDevice,

    #[error("Device is busy or in use")]
    DeviceBusy,

    #[error("Insufficient privileges - run as root/admin")]
    InsufficientPrivileges,

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Unsupported filesystem: {0}")]
    UnsupportedFilesystem(String),

    #[error("Platform not supported")]
    PlatformNotSupported,

    #[error("Command failed: {0}")]
    CommandFailed(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
