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

/// The fill of the system application checkbox that does not hold the cursor.
const FILTER_BACKGROUND: Color = Color::Rgb(60, 60, 60);

/// How many rows the system application checkbox takes in its panel.
const FILTER_HEIGHT: u16 = 1;

/// The label of the checkbox that decides whether system applications show.
const FILTER_LABEL: &str = "Show System Apps";

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
    if app.removal_confirmation_package().is_some() {
        render_removal_confirmation(frame, app);
    }
}

/// Draws a single panel, styled by whether it has focus.
fn render_panel(frame: &mut Frame, app: &mut App, area: Rect, panel: Panel) {
    let focused = app.focused().panel() == panel;
    let block = Block::bordered()
        .title(panel.title())
        .title_alignment(Alignment::Center)
        .border_style(panel_style(focused));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    match panel {
        Panel::DeviceSelection => {
            render_device_selection(frame, app, inner);
        }
        Panel::StandbyCrates => {
            render_crates(frame, app, inner);
        }
        Panel::Operations => {
            render_operations(frame, app, inner);
        }
        Panel::RemovePackages => {
            render_remove_packages(frame, app, inner);
        }
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

/// Draws the device list and the persistent system application checkbox.
///
/// The checkbox keeps the bottom row of the panel for itself, so it stays
/// visible whatever the device list looks like and while the list is navigated.
/// It is a control of the panel, not a panel of its own: everything is drawn
/// inside the one border the two share.
fn render_device_selection(frame: &mut Frame, app: &App, area: Rect) {
    let list_height = area.height.saturating_sub(FILTER_HEIGHT);
    let list = Rect::new(area.x, area.y, area.width, list_height);
    let body = Paragraph::new(device_lines(app));
    frame.render_widget(body, list);

    let top = area.y.saturating_add(list_height);
    let row = Rect::new(area.x, top, area.width, FILTER_HEIGHT);
    render_filter_checkbox(frame, row, app);
}

/// Draws the checkbox that decides whether system applications show.
///
/// The checkbox marks the focus with a cursor mark and the focus tone, so it is
/// recognizable as the focused control while the device list above it keeps its
/// selection mark.
fn render_filter_checkbox(frame: &mut Frame, row: Rect, app: &App) {
    let focused = app.filter_is_focused();
    let marker = if focused {
        "> "
    } else {
        "  "
    };
    let checkbox = if app.show_system() {
        "[x]"
    } else {
        "[ ]"
    };
    let text = format!("{marker}{checkbox} {FILTER_LABEL}");
    let body = Paragraph::new(text).style(filter_style(focused));
    frame.render_widget(body, row);
}

/// How many rows the removal status takes in its panel.
const REMOVAL_STATUS_HEIGHT: u16 = 1;

/// Draws the packages that can be removed and the status of the removal.
///
/// The list scrolls just enough to keep the selection in view, and the status
/// row keeps the last result or the removal in progress visible below it.
fn render_remove_packages(frame: &mut Frame, app: &App, area: Rect) {
    let list_height = area.height.saturating_sub(REMOVAL_STATUS_HEIGHT);
    let list = Rect::new(area.x, area.y, area.width, list_height);
    let status = Rect::new(
        area.x,
        area.y.saturating_add(list_height),
        area.width,
        REMOVAL_STATUS_HEIGHT,
    );

    render_removal_list(frame, app, list);
    render_removal_status(frame, app, status);
}

/// Draws the package list of the Remove Packages panel.
fn render_removal_list(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(message) = app.removal_placeholder() {
        let body =
            Paragraph::new(message).style(Style::new().fg(Color::DarkGray));
        frame.render_widget(body, area);
        return;
    }

    let packages = app.removal_packages();
    let height = usize::from(area.height);
    if height == 0 || packages.is_empty() {
        return;
    }

    let selected = app.removal_selected_index();
    let start = selected.saturating_add(1).saturating_sub(height);
    let end = packages.len().min(start.saturating_add(height));

    let mut y = area.y;
    for index in start..end {
        let package = match packages.get(index) {
            Some(package) => package,
            None => continue,
        };
        let marker = if index == selected {
            "> "
        } else {
            "  "
        };
        let style = removal_package_style(index == selected, app);
        let line = Line::from(format!("{marker}{package}"));
        let row = Rect::new(area.x, y, area.width, 1);
        frame.render_widget(Paragraph::new(line).style(style), row);
        y = y.saturating_add(1);
    }
}

/// Returns the style of one package row.
///
/// The selected package is emphasized only while the panel has the focus, so
/// the mark of the selection stays recognizable without competing with the
/// focused panel.
fn removal_package_style(selected: bool, app: &App) -> Style {
    if selected && app.remove_packages_are_focused() {
        return Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    }
    return Style::new();
}

/// Draws the status line of the Remove Packages panel.
fn render_removal_status(frame: &mut Frame, app: &App, area: Rect) {
    let status = match app.removal_status() {
        Some(status) => status,
        None => return,
    };
    let body = Paragraph::new(status).style(Style::new().fg(Color::DarkGray));
    frame.render_widget(body, area);
}

/// Draws the confirmation that asks before a package is removed.
///
/// The confirmation names the package, states that the removal only affects the
/// current Android user, and keeps the answer to one key press.
fn render_removal_confirmation(frame: &mut Frame, app: &App) {
    let package = match app.removal_confirmation_package() {
        Some(package) => package,
        None => return,
    };

    let area = centered_rect(frame.area());
    frame.render_widget(Clear, area);

    let lines = vec![
        Line::from(format!("Uninstall {package}?")),
        Line::from(""),
        Line::from("The package is removed only for the current Android user."),
        Line::from("If it is preinstalled, its APK stays in the system image."),
        Line::from(""),
        Line::from("[y] Confirm    [n] Cancel"),
    ];
    let block = Block::bordered()
        .title("User-Level Uninstall")
        .title_alignment(Alignment::Center)
        .border_style(Style::new().fg(Color::Yellow));
    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap {
        trim: true,
    });
    frame.render_widget(paragraph, area);
}

/// Draws the expandable sections inside the Standby Crates panel.
///
/// The panel is told how many rows it has room for, so it scrolls to keep its
/// cursor visible. The cursor mark is shown only while the sections hold the
/// focus.
fn render_crates(frame: &mut Frame, app: &mut App, area: Rect) {
    app.set_control_viewport(usize::from(area.height));
    let focused = app.crates_are_focused();
    render_control_content(frame, app.control(), area, focused);
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
    content_focused: bool,
) {
    let bottom = area.y.saturating_add(area.height);

    let mut y = area.y;
    for line in control.lines(content_focused) {
        if y >= bottom {
            return;
        }
        let row = Rect::new(area.x, y, area.width, 1);
        render_control_line(frame, row, &line);
        y = y.saturating_add(1);
    }
}

/// Returns the style of the system application checkbox.
///
/// The checkbox uses a fill of its own so it reads as a control rather than as
/// another list row, and it switches to the focus tone while the cursor is on
/// it, so the focused control is recognizable at a glance.
fn filter_style(focused: bool) -> Style {
    let style = if focused {
        Style::new()
            .fg(Color::Black)
            .bg(HEADER_FOCUSED_BACKGROUND)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(Color::Gray).bg(FILTER_BACKGROUND)
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
