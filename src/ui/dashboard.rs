use ratatui::{
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Padding, Paragraph, Row, Table},
    Frame,
};

use crate::app::App;
use crate::core::AppState;
use crate::utils::bytes_to_human;

// Design tokens for consistent styling
const COLOR_PRIMARY: Color = Color::Rgb(99, 179, 237);    // Soft blue
const COLOR_SUCCESS: Color = Color::Rgb(104, 211, 145);   // Soft green
const COLOR_WARNING: Color = Color::Rgb(246, 173, 85);    // Soft orange
const COLOR_DANGER: Color = Color::Rgb(252, 129, 129);    // Soft red
const COLOR_MUTED: Color = Color::Rgb(113, 128, 150);     // Gray
const COLOR_BORDER: Color = Color::Rgb(74, 85, 104);      // Dark gray

/// Draw the main dashboard with device list
pub fn draw_dashboard(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Outer margin for breathing room
    let inner_area = area.inner(Margin::new(2, 1));

    let chunks = Layout::vertical([
        Constraint::Length(5),  // Header
        Constraint::Min(8),     // Device table
        Constraint::Length(3),  // Help bar
    ])
    .split(inner_area);

    draw_header(frame, chunks[0], app);
    draw_device_table(frame, chunks[1], app);
    draw_help_bar(frame, chunks[2], app);
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    // Privilege badge
    let (badge_text, badge_style) = if app.disk_manager.has_privileges() {
        (
            " ● ROOT ",
            Style::default()
                .fg(Color::Black)
                .bg(COLOR_SUCCESS)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        (
            " ○ USER ",
            Style::default()
                .fg(Color::Black)
                .bg(COLOR_WARNING)
                .add_modifier(Modifier::BOLD),
        )
    };

    let title_line = Line::from(vec![
        Span::styled(
            "Pervie",
            Style::default()
                .fg(COLOR_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(badge_text, badge_style),
    ]);

    let subtitle = Line::from(vec![Span::styled(
        format!("{} devices detected", app.devices.len()),
        Style::default().fg(COLOR_MUTED),
    )]);

    let header = Paragraph::new(vec![Line::default(), title_line, Line::default(), subtitle])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_set(border::ROUNDED)
                .border_style(Style::default().fg(COLOR_BORDER))
                .padding(Padding::horizontal(2)),
        );

    frame.render_widget(header, area);
}

fn draw_device_table(frame: &mut Frame, area: Rect, app: &App) {
    // Header row
    let header_cells = ["NAME", "SIZE", "TYPE", "MOUNT POINT", "STATUS"]
        .iter()
        .map(|h| {
            Cell::from(format!(" {} ", h)).style(
                Style::default()
                    .fg(COLOR_MUTED)
                    .add_modifier(Modifier::BOLD),
            )
        });

    let header = Row::new(header_cells).height(1).bottom_margin(1);

    // Data rows
    let rows: Vec<Row> = app
        .devices
        .iter()
        .enumerate()
        .map(|(i, device)| {
            let is_selected = i == app.selected_index;

            // Color based on device type
            let base_color = if device.is_protected {
                COLOR_DANGER
            } else if device.is_removable {
                COLOR_SUCCESS
            } else {
                Color::White
            };

            // Selection styling
            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(base_color)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(base_color)
            };

            // Status indicator
            let status_icon = if device.is_protected {
                "🔒"
            } else if device.mount_point.is_some() {
                "●"
            } else {
                "○"
            };

            let mount = device.mount_point.as_deref().unwrap_or("—");
            let status_text = if device.is_protected {
                "Protected"
            } else if device.mount_point.is_some() {
                "Mounted"
            } else {
                "Unmounted"
            };

            // Clean up filesystem name
            let fs_clean = device
                .filesystem
                .replace("Apple_", "")
                .replace("_Container", "")
                .replace("_Recovery", " (R)")
                .replace("_ISC", " (ISC)");

            Row::new(vec![
                Cell::from(format!(" {} ", device.name)),
                Cell::from(format!(" {} ", bytes_to_human(device.size_bytes))),
                Cell::from(format!(" {} ", fs_clean)),
                Cell::from(format!(" {} ", mount)),
                Cell::from(format!(" {} {} ", status_icon, status_text)),
            ])
            .style(style)
        })
        .collect();

    // Column widths
    let widths = [
        Constraint::Min(18),
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Percentage(28),
        Constraint::Length(14),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_set(border::ROUNDED)
                .border_style(Style::default().fg(COLOR_BORDER))
                .title(" Devices ")
                .title_style(Style::default().fg(Color::White).bold())
                .padding(Padding::horizontal(1)),
        )
        .column_spacing(1)
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    frame.render_widget(table, area);
}

fn draw_help_bar(frame: &mut Frame, area: Rect, app: &App) {
    let bindings = match &app.state {
        AppState::Idle => vec![
            ("↑↓", "Navigate"),
            ("Enter", "Select"),
            ("r", "Refresh"),
            ("i", "Flash ISO"),
            ("q", "Quit"),
        ],
        AppState::DeviceSelected(_) => vec![
            ("u", "Unmount"),
            ("f", "Format"),
            ("Esc", "Back"),
            ("q", "Quit"),
        ],
        _ => vec![("Esc", "Back"), ("q", "Quit")],
    };

    let mut spans = Vec::new();
    for (i, (key, action)) in bindings.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  │  ", Style::default().fg(COLOR_BORDER)));
        }
        spans.push(Span::styled(
            format!(" {} ", key),
            Style::default()
                .fg(Color::White)
                .bg(COLOR_BORDER)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {}", action),
            Style::default().fg(COLOR_MUTED),
        ));
    }

    let help = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_set(border::ROUNDED)
                .border_style(Style::default().fg(COLOR_BORDER)),
        );

    frame.render_widget(help, area);
}
