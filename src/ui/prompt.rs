use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

use crate::app::App;
use crate::core::flasher::FlashProgress;
use crate::utils::bytes_to_human;
use ratatui::widgets::Gauge;

pub enum MessageType {
    Info,
    Success,
    Error,
}

/// Draw the filesystem selection menu
pub fn draw_format_menu(frame: &mut Frame, app: &App) {
    let area = centered_rect(50, 50, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Select Filesystem ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<ListItem> = app
        .fs_options
        .iter()
        .enumerate()
        .map(|(i, fs)| {
            let style = if i == app.selected_fs_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                Style::default()
            };
            ListItem::new(fs.display_name()).style(style)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);
}

/// Draw the ISO selection menu
pub fn draw_iso_selection(frame: &mut Frame, app: &App) {
    let area = centered_rect(70, 70, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Select ISO Image ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Magenta));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([Constraint::Length(2), Constraint::Min(1)]).split(inner);

    let header_text = format!(
        "   {:<12} | {:<10} | {:<8} | {}",
        "DISTRO", "VERSION", "ARCH", "VARIETY"
    );
    let header = Paragraph::new(header_text)
        .style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::BOTTOM));

    frame.render_widget(header, chunks[0]);

    let items: Vec<ListItem> = app
        .isos
        .iter()
        .enumerate()
        .map(|(i, iso)| {
            let content = format!(
                "{:<12} | {:<10} | {:<8} | {}",
                iso.name, iso.version, iso.arch, iso.variety
            );

            let style = if i == app.selected_iso_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                Style::default()
            };
            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, chunks[1]);
}

/// Draw confirmation dialog for destructive operations
pub fn draw_confirm_dialog(frame: &mut Frame, device_path: &str, input: &str, is_flash: bool) {
    let area = centered_rect(60, 40, frame.area());

    frame.render_widget(Clear, area);

    let title = if is_flash {
        " ⚠️  CONFIRM FLASH "
    } else {
        " ⚠️  CONFIRM FORMAT "
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Red));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Length(3),
        Constraint::Min(1),
    ])
    .split(inner);

    let warning_text = if is_flash {
        "This will OVERWRITE the device with the ISO image!"
    } else {
        "This will PERMANENTLY ERASE all data!"
    };

    let warning = Paragraph::new(Line::from(vec![
        Span::styled(
            "WARNING: ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::raw(warning_text),
    ]));
    frame.render_widget(warning, chunks[0]);

    let instruction = Paragraph::new(format!("Type '{}' to confirm:", device_path))
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(instruction, chunks[1]);

    let input_display = Paragraph::new(input).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Input ")
            .style(Style::default().fg(Color::White)),
    );
    frame.render_widget(input_display, chunks[2]);
}

pub fn draw_flash_progress(frame: &mut Frame, progress: &FlashProgress) {
    let area = centered_rect(60, 25, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Flashing ISO... ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .split(inner);

    let info = Paragraph::new(format!(
        "{}/{} ({:.1} MB/s)",
        bytes_to_human(progress.bytes_written),
        bytes_to_human(progress.total_bytes),
        progress.speed_mbps
    ))
    .alignment(Alignment::Center);

    frame.render_widget(info, chunks[0]);

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::NONE))
        .gauge_style(Style::default().fg(Color::Green))
        .ratio(progress.percent / 100.0)
        .label(format!("{:.1}%", progress.percent));

    frame.render_widget(gauge, chunks[1]);
}

/// Draw status/info messages
pub fn draw_status_message(frame: &mut Frame, app: &App, message: &str, msg_type: MessageType) {
    let area = centered_rect(60, 40, frame.area());

    frame.render_widget(Clear, area);

    let (title, color) = match msg_type {
        MessageType::Info => (" Progress ", Color::Cyan),
        MessageType::Success => (" Success ", Color::Green),
        MessageType::Error => (" Error ", Color::Red),
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().fg(color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);

    // Spinner for in-progress operations
    if matches!(msg_type, MessageType::Info) {
        let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let frame_idx = app.tick as usize % spinner_frames.len();
        let spinner = spinner_frames[frame_idx];

        let text = Paragraph::new(format!("{} {}", spinner, message))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(color));
        frame.render_widget(text, chunks[0]);
    } else {
        let text = Paragraph::new(message)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(color));
        frame.render_widget(text, chunks[0]);
    }

    let footer = Paragraph::new("Press Esc/Enter to dismiss")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));

    // Only show footer for non-info messages (Info is usually InProgress)
    if !matches!(msg_type, MessageType::Info) {
        frame.render_widget(footer, chunks[1]);
    }
}

/// Helper to create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let [area] = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .areas(r);
    let [area] = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .areas(area);
    area
}
