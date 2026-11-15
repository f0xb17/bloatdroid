//! Tests for the global focus order and the focus marks of the panels.
//!
//! The focus targets are a plain enumeration, so their order is tested
//! directly. The cursor marks of the sections are then driven through the
//! panel, which is where the panel decides whether it shows its cursor.

use crossterm::event::KeyCode;

use bloatdroid::control::ControlPanel;
use bloatdroid::tui::Focus;
use bloatdroid::tui::Panel;

use crate::common::MockAdb;
use crate::common::MockResponse;

use super::harness::ANDROID_13;
use super::harness::SERIAL;
use super::harness::entry_texts;
use super::harness::focused_text;
use super::harness::header_expansion;
use super::harness::panel_for;
use super::harness::set_filter;
use super::harness::wait_for;

/// Expects the filtered package listing of the device.
fn expect_listing(mock: &MockAdb, packages: &[&str]) {
    let mut listing = String::new();
    for package in packages {
        listing.push_str("package:");
        listing.push_str(package);
        listing.push('\n');
    }
    mock.expect(
        &["-s", SERIAL, "shell", "pm", "list", "packages", "-3"],
        MockResponse::output(listing),
    );
}

/// Expects the complete package listing of the device.
fn expect_full_listing(mock: &MockAdb, packages: &[&str]) {
    let mut listing = String::new();
    for package in packages {
        listing.push_str("package:");
        listing.push_str(package);
        listing.push('\n');
    }
    mock.expect(
        &["-s", SERIAL, "shell", "pm", "list", "packages"],
        MockResponse::output(listing),
    );
}

/// Expects the standby bucket read of one application.
fn expect_bucket(mock: &MockAdb, package: &str) {
    mock.expect(
        &["-s", SERIAL, "shell", "am", "get-standby-bucket", package],
        MockResponse::output("active"),
    );
}

/// Returns how often the filtered package list was read.
fn filtered_listings(mock: &MockAdb) -> usize {
    let wanted = vec![
        String::from("-s"),
        String::from(SERIAL),
        String::from("shell"),
        String::from("pm"),
        String::from("list"),
        String::from("packages"),
        String::from("-3"),
    ];

    let mut count = 0;
    for call in mock.calls() {
        if call == wanted {
            count += 1;
        }
    }
    return count;
}

/// Polls until the expanded section shows the expected number of rows.
fn wait_for_rows(panel: &mut ControlPanel, count: usize) {
    wait_for(panel, |panel| {
        return entry_texts(panel).len() == count;
    });
}

#[test]
fn tabs_forward_through_every_focus_target() {
    let expected = [
        Focus::DeviceSelection,
        Focus::DeviceSelectionFilter,
        Focus::RemovePackages,
        Focus::StandbyCrates,
        Focus::Operations,
        // The last target wraps around to the first one.
        Focus::DeviceSelection,
    ];

    let mut focus = Focus::DeviceSelection;
    for target in expected {
        assert_eq!(focus, target);
        focus = focus.next();
    }
}

#[test]
fn shift_tabs_backward_through_every_focus_target() {
    let expected = [
        Focus::DeviceSelection,
        Focus::Operations,
        Focus::StandbyCrates,
        Focus::RemovePackages,
        Focus::DeviceSelectionFilter,
        // The first target wraps around to the last one.
        Focus::DeviceSelection,
    ];

    let mut focus = Focus::DeviceSelection;
    for target in expected {
        assert_eq!(focus, target);
        focus = focus.previous();
    }
}

#[test]
fn keeps_the_checkbox_in_the_device_selection_panel() {
    assert_eq!(Focus::DeviceSelection.panel(), Panel::DeviceSelection);
    assert_eq!(Focus::DeviceSelectionFilter.panel(), Panel::DeviceSelection);
    // The checkbox is a control of the panel, not a panel of its own.
    assert_ne!(Focus::DeviceSelectionFilter, Focus::DeviceSelection);

    assert_eq!(Focus::RemovePackages.panel(), Panel::RemovePackages);
    assert_eq!(Focus::StandbyCrates.panel(), Panel::StandbyCrates);
    assert_eq!(Focus::Operations.panel(), Panel::Operations);
}

#[test]
fn marks_the_sections_only_while_they_have_the_focus() {
    let mock = MockAdb::new();
    let panel = panel_for(&mock, ANDROID_13);

    for line in panel.lines(false) {
        assert!(!line.is_focused(), "a cursor was shown without focus");
    }

    let mut marked = 0;
    for line in panel.lines(true) {
        if line.is_focused() {
            marked += 1;
        }
    }
    assert_eq!(marked, 1, "the focused sections showed no cursor");
    mock.assert_complete();
}

#[test]
fn the_sections_cannot_change_the_application_filter() {
    let mock = MockAdb::new();
    expect_listing(&mock, &["com.example.alpha"]);
    expect_bucket(&mock, "com.example.alpha");

    let mut panel = panel_for(&mock, ANDROID_13);
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for_rows(&mut panel, 1);
    assert!(panel.handle_key(KeyCode::Char(' ')));

    // Expanding and collapsing the section never asks for the complete
    // inventory: the setting belongs to the application, not to a section.
    assert_eq!(filtered_listings(&mock), 1);
    mock.assert_complete();
}

#[test]
fn keeps_the_expansion_and_the_selection_across_a_filter_change() {
    let mock = MockAdb::new();
    expect_listing(&mock, &["com.example.alpha", "com.example.mike"]);
    expect_bucket(&mock, "com.example.alpha");
    expect_bucket(&mock, "com.example.mike");
    expect_full_listing(
        &mock,
        &["com.android.systemui", "com.example.alpha", "com.example.mike"],
    );
    expect_bucket(&mock, "com.android.systemui");
    expect_bucket(&mock, "com.example.alpha");
    expect_bucket(&mock, "com.example.mike");

    let mut panel = panel_for(&mock, ANDROID_13);
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for_rows(&mut panel, 2);
    assert!(panel.handle_key(KeyCode::Down));
    assert_eq!(
        focused_text(&panel),
        Some(String::from("com.example.alpha  active"))
    );

    // The filter changes while a section is expanded and an application is
    // selected.
    assert!(set_filter(&mut panel, true));
    wait_for_rows(&mut panel, 3);

    assert_eq!(header_expansion(&panel), vec![true, false, false, false]);
    assert_eq!(
        focused_text(&panel),
        Some(String::from("com.example.alpha  active"))
    );
    mock.assert_complete();
}
