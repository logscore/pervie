pub mod dashboard;
pub mod prompt;

use ratatui::Frame;

use crate::app::App;
use crate::core::AppState;

/// Main draw function that dispatches to appropriate view
pub fn draw(frame: &mut Frame, app: &App) {
    match &app.state {
        AppState::Idle | AppState::DeviceSelected(_) => {
            dashboard::draw_dashboard(frame, app);
        }
        AppState::FormattingMenu => {
            dashboard::draw_dashboard(frame, app);
            prompt::draw_format_menu(frame, app);
        }
        AppState::ConfirmDestructive(path) => {
            dashboard::draw_dashboard(frame, app);
            prompt::draw_confirm_dialog(frame, path, &app.input_buffer);
        }
        AppState::InProgress(msg) => {
            dashboard::draw_dashboard(frame, app);
            prompt::draw_status_message(frame, app, msg, prompt::MessageType::Info);
        }
        AppState::Error(msg) => {
            dashboard::draw_dashboard(frame, app);
            prompt::draw_status_message(frame, app, msg, prompt::MessageType::Error);
        }
        AppState::Success(msg) => {
            dashboard::draw_dashboard(frame, app);
            prompt::draw_status_message(frame, app, msg, prompt::MessageType::Success);
        }
    }
}
