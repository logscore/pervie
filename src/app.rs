use std::sync::Arc;

use crate::core::{AppState, Device, FileSystemType, Iso};
use crate::core::disk_ops::DiskManager;
use crate::core::flasher::Flasher;

/// Main application state
pub struct App {
    pub devices: Vec<Device>,
    pub selected_index: usize,
    pub state: AppState,
    pub input_buffer: String,
    pub disk_manager: Arc<dyn DiskManager>,
    pub flasher: Arc<Flasher>,
    pub fs_options: Vec<FileSystemType>,
    pub selected_fs_index: usize,
    pub isos: Vec<Iso>,
    pub selected_iso_index: usize,
    pub should_quit: bool,
    pub tick: u64,
    pub operation_tx: tokio::sync::mpsc::UnboundedSender<AppState>,
    pub operation_rx: tokio::sync::mpsc::UnboundedReceiver<AppState>,
}

impl App {
    pub fn new(disk_manager: Arc<dyn DiskManager>) -> Self {
        let (operation_tx, operation_rx) = tokio::sync::mpsc::unbounded_channel();
        Self {
            devices: Vec::new(),
            selected_index: 0,
            state: AppState::Idle,
            input_buffer: String::new(),
            disk_manager,
            flasher: Arc::new(Flasher::new()),
            fs_options: FileSystemType::macos_options(),
            selected_fs_index: 0,
            isos: vec![
                Iso {
                    name: "Debian".to_string(),
                    version: "13".to_string(),
                    arch: "amd64".to_string(),
                    variety: "Netinst".to_string(),
                    url: "https://cdimage.debian.org/debian-cd/current/amd64/iso-cd/debian-13.2.0-amd64-netinst.iso".to_string(),
                },
                Iso {
                    name: "Debian".to_string(),
                    version: "13".to_string(),
                    arch: "arm64".to_string(),
                    variety: "Netinst".to_string(),
                    url: "https://cdimage.debian.org/debian-cd/current/arm64/iso-cd/debian-13.2.0-arm64-netinst.iso".to_string(),
                },
                Iso {
                    name: "Ubuntu".to_string(),
                    version: "24.04.3".to_string(),
                    arch: "amd64".to_string(),
                    variety: "Live Server".to_string(),
                    url: "https://releases.ubuntu.com/24.04.3/ubuntu-24.04.3-live-server-amd64.iso".to_string(),
                },
                Iso {
                    name: "Ubuntu".to_string(),
                    version: "24.04.3".to_string(),
                    arch: "arm64".to_string(),
                    variety: "Live Server".to_string(),
                    url: "https://cdimage.ubuntu.com/releases/24.04.3/release/ubuntu-24.04.3-live-server-arm64.iso".to_string(),
                },
                Iso {
                    name: "Alpine".to_string(),
                    version: "3.23.2".to_string(),
                    arch: "x86_64".to_string(),
                    variety: "Standard".to_string(),
                    url: "https://dl-cdn.alpinelinux.org/alpine/v3.23/releases/x86_64/alpine-standard-3.23.2-x86_64.iso".to_string(),
                },
                Iso {
                    name: "Alpine".to_string(),
                    version: "3.23.2".to_string(),
                    arch: "aarch64".to_string(),
                    variety: "Standard".to_string(),
                    url: "https://dl-cdn.alpinelinux.org/alpine/v3.23/releases/aarch64/alpine-standard-3.23.2-aarch64.iso".to_string(),
                },
                Iso {
                    name: "Arch Linux".to_string(),
                    version: "2025.12.01".to_string(),
                    arch: "x86_64".to_string(),
                    variety: "Standard".to_string(),
                    url: "https://geo.mirror.pkgbuild.com/iso/2025.12.01/archlinux-2025.12.01-x86_64.iso".to_string(),
                },
                // Windows 11 - Reserved for future S3 bucket implementation
                /*
                Iso {
                    name: "Windows 11".to_string(),
                    version: "23H2".to_string(),
                    arch: "x64".to_string(),
                    variety: "English Intl".to_string(),
                    url: "https://www.microsoft.com/software-download/windows11".to_string(),
                },
                Iso {
                    name: "Windows 11".to_string(),
                    version: "23H2 (ARM)".to_string(),
                    arch: "arm64".to_string(),
                    variety: "Insider VHDX".to_string(),
                    url: "https://www.microsoft.com/en-us/software-download/windowsinsiderpreviewARM64".to_string(),
                },
                */
            ],
            selected_iso_index: 0,
            should_quit: false,
            tick: 0,
            operation_tx,
            operation_rx,
        }
    }

    pub async fn refresh_devices(&mut self) -> Result<(), String> {
        match self.disk_manager.list_devices().await {
            Ok(devices) => {
                self.devices = devices;
                if self.selected_index >= self.devices.len() && !self.devices.is_empty() {
                    self.selected_index = self.devices.len() - 1;
                }
                Ok(())
            }
            Err(e) => {
                self.state = AppState::Error(e.to_string());
                Err(e.to_string())
            }
        }
    }

    pub fn select_next(&mut self) {
        if !self.devices.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.devices.len();
        }
    }

    pub fn select_previous(&mut self) {
        if !self.devices.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.devices.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    pub fn select_next_fs(&mut self) {
        if !self.fs_options.is_empty() {
            self.selected_fs_index = (self.selected_fs_index + 1) % self.fs_options.len();
        }
    }

    pub fn select_previous_fs(&mut self) {
        if !self.fs_options.is_empty() {
            if self.selected_fs_index == 0 {
                self.selected_fs_index = self.fs_options.len() - 1;
            } else {
                self.selected_fs_index -= 1;
            }
        }
    }

    pub fn selected_device(&self) -> Option<&Device> {
        self.devices.get(self.selected_index)
    }

    pub fn selected_fs(&self) -> Option<FileSystemType> {
        self.fs_options.get(self.selected_fs_index).copied()
    }

    pub fn enter_select_mode(&mut self) {
        if !self.devices.is_empty() {
            self.state = AppState::DeviceSelected(self.selected_index);
        }
    }

    pub fn enter_format_menu(&mut self) {
        self.state = AppState::FormattingMenu;
        self.selected_fs_index = 0;
        self.input_buffer.clear();
    }

    pub fn enter_iso_selection(&mut self) {
        self.state = AppState::IsoSelection;
        self.selected_iso_index = 0;
    }

    pub fn select_next_iso(&mut self) {
        if !self.isos.is_empty() {
            self.selected_iso_index = (self.selected_iso_index + 1) % self.isos.len();
        }
    }

    pub fn select_previous_iso(&mut self) {
        if !self.isos.is_empty() {
            if self.selected_iso_index == 0 {
                self.selected_iso_index = self.isos.len() - 1;
            } else {
                self.selected_iso_index -= 1;
            }
        }
    }

    pub fn selected_iso(&self) -> Option<&Iso> {
        self.isos.get(self.selected_iso_index)
    }

    pub fn flash_selected_iso(&mut self) {
        if let Some(device) = self.selected_device().cloned() {
            self.state = AppState::ConfirmFlash(device.path);
            self.input_buffer.clear();
        }
    }

    pub fn start_flashing(&mut self) {
        let device = match self.selected_device().cloned() {
            Some(d) => d,
            None => return,
        };
        
        // Verify confirmation
        if self.input_buffer != device.path {
            self.state = AppState::Error(format!(
                "Confirmation mismatch. Expected '{}', got '{}'",
                device.path, self.input_buffer
            ));
            return;
        }

        let iso = match self.selected_iso().cloned() {
            Some(i) => i,
            None => return,
        };

        self.state = AppState::InProgress(format!("Starting flash of {}...", iso.name));

        let tx = self.operation_tx.clone();
        let disk_manager = self.disk_manager.clone();
        let flasher = self.flasher.clone();
        let path = device.path.clone();
        let url = iso.url.clone();

        tokio::spawn(async move {
            // 1. Unmount device first
            let _ = tx.send(AppState::InProgress(format!("Unmounting {}...", path)));
            if let Err(e) = disk_manager.unmount(&path).await {
                let _ = tx.send(AppState::Error(format!("Failed to unmount: {}", e)));
                return;
            }

            // 2. Prepare for flashing
            let _ = tx.send(AppState::InProgress(format!("Flashing {}...", iso.name)));
            
            // On macOS, use raw disk device for performance and correct access
            #[cfg(target_os = "macos")]
            let flash_path = path.replace("/dev/disk", "/dev/rdisk");
            
            #[cfg(not(target_os = "macos"))]
            let flash_path = path.clone();

            // 3. Execute Flash
            match flasher.flash(url, flash_path.clone(), tx.clone()).await {
                Ok(()) => {
                    // 4. Auto-eject on success
                    let _ = tx.send(AppState::InProgress("Ejecting device...".to_string()));
                    if let Err(e) = disk_manager.eject(&path).await {
                         // Warning instead of error? For now, let's just warn but consider it success
                         let _ = tx.send(AppState::Success(format!("Flash complete, but eject failed: {}", e)));
                    } else {
                        let _ = tx.send(AppState::Success("Flash complete! Device ejected safely.".to_string()));
                    }
                }
                Err(e) => {
                    let _ = tx.send(AppState::Error(format!("{:#}", e)));
                }
            }
        });
    }

    pub fn enter_confirm_mode(&mut self) {
        if let Some(device) = self.selected_device() {
            self.state = AppState::ConfirmDestructive(device.path.clone());
            self.input_buffer.clear();
        }
    }

    pub fn cancel(&mut self) {
        self.state = AppState::Idle;
        self.input_buffer.clear();
    }

    pub fn unmount_selected(&mut self) {
        if let Some(device) = self.selected_device().cloned() {
            if device.is_protected {
                self.state = AppState::Error("Cannot unmount protected system drive".to_string());
                return;
            }

            self.state = AppState::InProgress("Unmounting...".to_string());

            let tx = self.operation_tx.clone();
            let disk_manager = self.disk_manager.clone();
            let path = device.path.clone();

            tokio::spawn(async move {
                match disk_manager.unmount(&path).await {
                    Ok(()) => {
                        let _ = tx.send(AppState::Success(format!("Unmounted {}", path)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppState::Error(e.to_string()));
                    }
                }
            });
        }
    }

    pub fn format_selected(&mut self) {
        let device = match self.selected_device().cloned() {
            Some(d) => d,
            None => return,
        };

        if device.is_protected {
            self.state = AppState::Error("Cannot format protected system drive".to_string());
            return;
        }

        // Verify confirmation input matches device path
        if self.input_buffer != device.path {
            self.state = AppState::Error(format!(
                "Confirmation mismatch. Expected '{}', got '{}'",
                device.path, self.input_buffer
            ));
            return;
        }

        let fs_type = match self.selected_fs() {
            Some(fs) => fs,
            None => return,
        };

        self.state = AppState::InProgress(format!("Formatting {} as {}...", device.path, fs_type.display_name()));

        let tx = self.operation_tx.clone();
        let disk_manager = self.disk_manager.clone();
        let path = device.path.clone();
        let display_name = fs_type.display_name();

        tokio::spawn(async move {
            match disk_manager.format(&path, fs_type, "UNTITLED").await {
                Ok(()) => {
                    let _ = tx.send(AppState::Success(format!(
                        "Formatted {} as {}",
                        path, display_name
                    )));
                }
                Err(e) => {
                    let _ = tx.send(AppState::Error(e.to_string()));
                }
            }
        });
    }
}
