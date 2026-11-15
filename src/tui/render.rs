//! Drawing of the panels and the device information overlay.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};

use crate::control::ControlLine;
use crate::control::ControlPanel;
use crate::control::common::application_row;

use super::app::App;
use super::app::Panel;
use super::operations::level_style;

/// How many columns a section header reserves for its expansion indicator.
const INDICATOR_WIDTH: u16 = 2;

/// How many columns an entry line reserves for its focus marker.
const MARKER_WIDTH: u16 = 2;

/// The indicator of a collapsed section.
const COLLAPSED_INDICATOR: &str = "▶";

/// The indicator of an expanded section.
const EXPANDED_INDICATOR: &str = "▼";

/// The fill of a section header that does not hold the cursor.
const HEADER_BACKGROUND: Color = Color::Yellow;

/// The fill of the control strip that does not hold the cursor.
const STRIP_BACKGROUND: Color = Color::Rgb(60, 60, 60);

/// The fill of a section header that holds the cursor.
///
/// A warm orange that contrasts with the yellow of every other header, so the
/// focused section is recognizable at a glance. The color depends only on the
/// focus, never on whether the section is expanded.
const HEADER_FOCUSED_BACKGROUND: Color = Color::Rgb(230, 126, 34);

/// Draws the four panels and, when it is open, the overlay.
///
/// The application is borrowed mutably because the control panel learns the
/// height of its area here, which decides how much of its content is visible.
pub(crate) fn render(frame: &mut Frame, app: &mut App) {
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
fn render_panel(frame: &mut Frame, app: &mut App, area: Rect, panel: Panel) {
    let focused = app.focused() == panel;
    let block = Block::bordered()
        .title(panel.title())
        .title_alignment(Alignment::Center)
        .border_style(panel_style(focused));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    match panel {
        Panel::DeviceSelection => {
            let body = Paragraph::new(device_lines(app));
            frame.render_widget(body, inner);
        }
        Panel::StandbyCrates => {
            render_control(frame, app, inner);
        }
        Panel::Operations => {
            render_operations(frame, app, inner);
        }
        _ => {}
    }
}

/// Draws the Operations Log inside the Operations panel.
///
/// The panel is told the size of its area first, because the log layout depends
/// on the width and the scrolling depends on the height. Every row carries its
/// own number of columns, so the panel draws one row per line.
fn render_operations(frame: &mut Frame, app: &mut App, area: Rect) {
    let width = usize::from(area.width);
    let height = usize::from(area.height);
    app.set_operations_viewport(width, height);

    let bottom = area.y.saturating_add(area.height);
    let mut y = area.y;
    for (level, row) in app.operations().visible() {
        if y >= bottom {
            return;
        }

        let line = Line::from(vec![
            Span::raw(String::from(row.prefix())),
            Span::styled(String::from(row.message()), level_style(level)),
        ]);
        let row_area = Rect::new(area.x, y, area.width, 1);
        frame.render_widget(Paragraph::new(line), row_area);
        y = y.saturating_add(1);
    }
}

/// Draws the control sections and the control strip inside the Standby Crates
/// panel.
///
/// The strip keeps the bottom rows of the panel for itself and is drawn last,
/// so the scrolling content above it can never cover it. The scrollable content
/// is told about the rows the strip takes, so no line of it is hidden behind the
/// strip.
fn render_control(frame: &mut Frame, app: &mut App, area: Rect) {
    let strip = app.control().overlay_lines();
    let strip_height = u16::try_from(strip.len()).unwrap_or(u16::MAX);
    let content_height = area.height.saturating_sub(strip_height);
    app.set_control_viewport(usize::from(content_height));

    render_control_content(frame, app.control(), area);
    render_control_strip(frame, area, &strip);
}

/// Draws the control sections inside the Standby Crates panel.
///
/// Every line occupies one row of the panel, so a section header can span the
/// full width on its own. The panel already limited the lines to the ones that
/// fit into the area, so no row is drawn outside it.
fn render_control_content(
    frame: &mut Frame,
    control: &ControlPanel,
    area: Rect,
) {
    let bottom = area.y.saturating_add(area.height);

    let mut y = area.y;
    for line in control.lines() {
        if y >= bottom {
            return;
        }
        let row = Rect::new(area.x, y, area.width, 1);
        render_control_line(frame, row, &line);
        y = y.saturating_add(1);
    }
}

/// Draws the control strip over the bottom rows of the area.
fn render_control_strip(frame: &mut Frame, area: Rect, lines: &[ControlLine]) {
    let height = u16::try_from(lines.len()).unwrap_or(u16::MAX);
    if height == 0 {
        return;
    }

    let top = area.y.saturating_add(area.height.saturating_sub(height));
    let strip = Rect::new(area.x, top, area.width, height);
    frame.render_widget(Block::new().style(strip_style(false)), strip);

    let mut y = top;
    for line in lines {
        let row = Rect::new(area.x, y, area.width, 1);
        render_control_strip_line(frame, row, line);
        y = y.saturating_add(1);
    }
}

/// Draws one line of the control strip.
fn render_control_strip_line(frame: &mut Frame, row: Rect, line: &ControlLine) {
    let marker = if line.is_focused() {
        "> "
    } else {
        "  "
    };
    let checkbox = if line.is_checked() {
        "[x]"
    } else {
        "[ ]"
    };
    let text = format!("{marker}{checkbox} {}", line.text());
    let body = Paragraph::new(text).style(strip_style(line.is_focused()));
    frame.render_widget(body, row);
}

/// Returns the style of the control strip.
///
/// The strip uses a fill of its own so it reads as a bar rather than as another
/// section, and it switches to the focus tone while the cursor is on it, so the
/// focused control is recognizable at a glance.
fn strip_style(focused: bool) -> Style {
    let style = if focused {
        Style::new()
            .fg(Color::Black)
            .bg(HEADER_FOCUSED_BACKGROUND)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(Color::Gray).bg(STRIP_BACKGROUND)
    };
    return style;
}

/// Draws one line of the control panel.
fn render_control_line(frame: &mut Frame, row: Rect, line: &ControlLine) {
    if line.is_header() {
        render_control_header(frame, row, line);
        return;
    }

    let marker = if line.is_focused() {
        "> "
    } else {
        "  "
    };
    let text = control_line_text(line, marker, row.width);
    let body = Paragraph::new(text).style(control_content_style(line));
    frame.render_widget(body, row);
}

/// Returns the text of an entry or status line.
///
/// An entry is laid out by the shared two-column helper, so its state label
/// ends at the right edge of the row in every section. A status line is shown
/// as it is.
fn control_line_text(line: &ControlLine, marker: &str, width: u16) -> String {
    let state = match line.state() {
        Some(state) => state,
        None => return format!("{marker}{}", line.text()),
    };

    let available = usize::from(width.saturating_sub(MARKER_WIDTH));
    let text = application_row(line.text(), state, available);
    return format!("{marker}{text}");
}

/// Draws a section header as a filled block that spans the whole row.
///
/// The fill is drawn first, then the title is centered over the full width,
/// and last the indicator is drawn in the columns reserved for it, so the
/// indicator never shifts the title.
fn render_control_header(frame: &mut Frame, row: Rect, line: &ControlLine) {
    let style = header_style(line.is_focused());
    frame.render_widget(Block::new().style(style), row);

    let title =
        Paragraph::new(line.text()).alignment(Alignment::Center).style(style);
    frame.render_widget(title, row);

    let indicator = if line.is_expanded() {
        EXPANDED_INDICATOR
    } else {
        COLLAPSED_INDICATOR
    };
    let indicator_area = match right_edge(row, INDICATOR_WIDTH) {
        Some(indicator_area) => indicator_area,
        None => return,
    };
    let indicator = Paragraph::new(format!("{indicator} "))
        .alignment(Alignment::Right)
        .style(style);
    frame.render_widget(indicator, indicator_area);
}

/// Returns the style of an entry or status line.
fn control_content_style(line: &ControlLine) -> Style {
    let style = if line.is_focused() {
        Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else if line.is_status() {
        Style::new().fg(Color::DarkGray)
    } else {
        Style::new()
    };
    return style;
}

/// Returns the style of a section header.
///
/// Every header uses the same yellow-toned fill, apart from the header that
/// holds the cursor, which uses the contrasting focus tone. The expansion
/// state plays no part in the choice, so a section looks the same expanded and
/// collapsed.
fn header_style(focused: bool) -> Style {
    let background = if focused {
        HEADER_FOCUSED_BACKGROUND
    } else {
        HEADER_BACKGROUND
    };
    let style = Style::new()
        .fg(Color::Black)
        .bg(background)
        .add_modifier(Modifier::BOLD);
    return style;
}

/// Returns the rightmost columns of a row, if the row is wide enough.
fn right_edge(row: Rect, width: u16) -> Option<Rect> {
    let start = row.width.checked_sub(width)?;
    let area = Rect::new(row.x.saturating_add(start), row.y, width, 1);
    return Some(area);
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
