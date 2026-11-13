//! Drawing of the panels and the device information overlay.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};

use super::app::App;
use super::app::Panel;

/// Draws the four panels and, when it is open, the overlay.
pub(crate) fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let rows = Layout::vertical([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(area);

    let [top, bottom] = match rows.as_ref() {
        [top, bottom] => [*top, *bottom],
        _ => return,
    };

    let top_columns = match split_columns(top) {
        Some(columns) => columns,
        None => return,
    };
    let bottom_columns = match split_columns(bottom) {
        Some(columns) => columns,
        None => return,
    };

    let [top_left, top_right] = top_columns;
    let [bottom_left, bottom_right] = bottom_columns;

    let panels = [
        (top_left, Panel::DeviceSelection),
        (top_right, Panel::RemovePackages),
        (bottom_left, Panel::Operations),
        (bottom_right, Panel::StandbyCrates),
    ];

    for (panel_area, panel) in panels {
        render_panel(frame, app, panel_area, panel);
    }

    if app.overlay_is_open() {
        render_overlay(frame, app);
    }
}

/// Draws a single panel, styled by whether it has focus.
fn render_panel(frame: &mut Frame, app: &App, area: Rect, panel: Panel) {
    let focused = app.focused() == panel;
    let block = Block::bordered()
        .title(panel.title())
        .title_alignment(Alignment::Center)
        .border_style(panel_style(focused));

    if panel == Panel::DeviceSelection {
        let body = Paragraph::new(device_lines(app)).block(block);
        frame.render_widget(body, area);
        return;
    }

    frame.render_widget(block, area);
}

/// Returns the device list as rendered lines.
fn device_lines(app: &App) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    if let Some(error) = app.discovery_error() {
        lines.push(Line::from(format!("adb: {error}")));
    }

    let devices = app.devices();
    if devices.is_empty() {
        lines.push(Line::from("No devices found"));
        return lines;
    }

    for (index, device) in devices.iter().enumerate() {
        let marker = if Some(index) == app.selected_index() {
            "> "
        } else {
            "  "
        };
        let line = format!(
            "{marker}{}  {}  Android {}",
            device.identifier, device.model, device.android_version
        );
        lines.push(Line::from(line));
    }

    return lines;
}

/// Draws the focused device information as a centered overlay.
fn render_overlay(frame: &mut Frame, app: &App) {
    let device = match app.selected_device() {
        Some(device) => device,
        None => return,
    };

    let area = centered_rect(frame.area());
    frame.render_widget(Clear, area);

    let fields = [
        ("Identifier", device.identifier.as_str()),
        ("Manufacturer", device.manufacturer.as_str()),
        ("Model", device.model.as_str()),
        ("Device", device.device.as_str()),
        ("Android version", device.android_version.as_str()),
        ("SDK version", device.sdk_version.as_str()),
        ("Build fingerprint", device.build_fingerprint.as_str()),
        ("ADB state", device.state.as_str()),
    ];

    let mut lines = Vec::new();
    for (label, value) in fields {
        lines.push(Line::from(format!("{label}: {value}")));
    }

    let block = Block::bordered()
        .title("Device Information")
        .title_alignment(Alignment::Center)
        .border_style(Style::new().fg(Color::Yellow));
    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap {
        trim: true,
    });
    frame.render_widget(paragraph, area);
}

/// Splits an area into a left and a right column.
fn split_columns(area: Rect) -> Option<[Rect; 2]> {
    let columns = Layout::horizontal([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(area);

    let left = *columns.first()?;
    let right = *columns.get(1)?;
    return Some([left, right]);
}

/// Returns a rectangle of the given share of the area, centered in it.
fn centered_rect(area: Rect) -> Rect {
    let width = (area.width as u32 * 70 / 100) as u16;
    let height = (area.height as u32 * 60 / 100) as u16;
    let x = area.x + (area.width - width) / 2;
    let y = area.y + (area.height - height) / 2;
    return Rect::new(x, y, width, height);
}

/// Returns the border style for a focused or an unfocused panel.
fn panel_style(focused: bool) -> Style {
    let style = if focused {
        Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(Color::DarkGray)
    };
    return style;
}
