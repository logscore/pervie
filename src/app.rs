use std::sync::Arc;

use crate::core::{AppState, Device, FileSystemType};
use crate::core::disk_ops::DiskManager;

/// Main application state
pub struct App {
    pub devices: Vec<Device>,
    pub selected_index: usize,
    pub state: AppState,
    pub input_buffer: String,
    pub disk_manager: Arc<dyn DiskManager>,
    pub fs_options: Vec<FileSystemType>,
    pub selected_fs_index: usize,
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
            fs_options: FileSystemType::macos_options(),
            selected_fs_index: 0,
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
