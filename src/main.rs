mod app;
mod core;
mod platform;
mod ui;
mod utils;

use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;

use crate::app::App;
use crate::core::AppState;
use crate::platform::get_disk_manager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Safety check: Validate terminal size BEFORE entering raw mode or alternate screen.
    let (cols, rows) = crossterm::terminal::size()?;
    if cols == 0 || rows == 0 || cols > 1000 || rows > 1000 {
        anyhow::bail!(
            "Invalid terminal size detected ({}x{}). Please ensure you're running in a valid terminal.",
            cols,
            rows
        );
    }

    // Now safe to setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let disk_manager = get_disk_manager();
    let mut app = App::new(disk_manager);

    // Check privileges - warn but don't exit
    if !app.disk_manager.has_privileges() {
        app.state =
            AppState::Error("Warning: Not running as root. Some operations may fail.".to_string());
    }

    // Initial device scan
    let _ = app.refresh_devices().await;

    // Main loop
    let result = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {e}");
    }

    Ok(())
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> anyhow::Result<()> {
    loop {
        app.tick = app.tick.wrapping_add(1);

        // Check for operation results
        if let Ok(new_state) = app.operation_rx.try_recv() {
            app.state = new_state.clone();
            if let AppState::Success(_) = new_state {
                let _ = app.refresh_devices().await;
            }
        }

        terminal.draw(|f| ui::draw(f, app))?;

        // Poll for events with timeout for tick
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match &app.state {
                    AppState::Idle => {
                        handle_idle_input(app, key.code).await;
                    }
                    AppState::DeviceSelected(_) => {
                        handle_selected_input(app, key.code);
                    }
                    AppState::IsoSelection => {
                        handle_iso_selection_input(app, key.code);
                    }
                    AppState::FormattingMenu => {
                        handle_format_menu_input(app, key.code);
                    }
                    AppState::ConfirmDestructive(_) | AppState::ConfirmFlash(_) => {
                        handle_confirm_input(app, key.code);
                    }
                    AppState::Flashing(_) | AppState::InProgress(_) => {
                        // Block input during operations
                    }
                    AppState::Error(_) | AppState::Success(_) => {
                        handle_message_input(app, key.code);
                    }
                }

                if app.should_quit {
                    return Ok(());
                }
            }
        }
    }
}

async fn handle_idle_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Up => app.select_previous(),
        KeyCode::Down => app.select_next(),
        KeyCode::Enter => app.enter_select_mode(),
        KeyCode::Char('r') => {
            let _ = app.refresh_devices().await;
        }
        _ => {}
    }
}

fn handle_selected_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Esc => app.cancel(),
        KeyCode::Up => app.select_previous(),
        KeyCode::Down => app.select_next(),
        KeyCode::Char('u') => app.unmount_selected(),
        KeyCode::Char('f') => app.enter_format_menu(),
        KeyCode::Char('i') => app.enter_iso_selection(),
        _ => {}
    }
}

fn handle_format_menu_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Esc => app.cancel(),
        KeyCode::Up => app.select_previous_fs(),
        KeyCode::Down => app.select_next_fs(),
        KeyCode::Enter => app.enter_confirm_mode(),
        _ => {}
    }
}

fn handle_iso_selection_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Esc => app.cancel(),
        KeyCode::Up => app.select_previous_iso(),
        KeyCode::Down => app.select_next_iso(),
        KeyCode::Enter => app.flash_selected_iso(),
        _ => {}
    }
}

fn handle_confirm_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.cancel(),
        KeyCode::Enter => match app.state {
            AppState::ConfirmDestructive(_) => app.format_selected(),
            AppState::ConfirmFlash(_) => app.start_flashing(),
            _ => {}
        },
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        _ => {}
    }
}

fn handle_message_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Esc | KeyCode::Enter => app.cancel(),
        _ => {}
    }
}
