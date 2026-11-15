//! Tests for the global focus order and the two Standby Crates focus targets.
//!
//! The focus targets are a plain enumeration, so their order is tested
//! directly. The two targets of the Standby Crates panel are then driven
//! through the panel, which is where their input is separated.

use crossterm::event::KeyCode;

use bloatdroid::control::ControlPanel;
use bloatdroid::tui::Focus;
use bloatdroid::tui::Panel;

use crate::common::MockAdb;
use crate::common::MockResponse;

use super::harness::ANDROID_13;
use super::harness::SERIAL;
use super::harness::entry_texts;
use super::harness::focused_overlay_lines;
use super::harness::focused_text;
use super::harness::header_expansion;
use super::harness::overlay_lines;
use super::harness::panel_for;
use super::harness::wait_for;

/// Expects the package listing of one device.
fn expect_listing(mock: &MockAdb, packages: &[&str], include_system: bool) {
    let mut listing = String::new();
    for package in packages {
        listing.push_str("package:");
        listing.push_str(package);
        listing.push('\n');
    }

    let arguments = if include_system {
        vec!["-s", SERIAL, "shell", "pm", "list", "packages"]
    } else {
        vec!["-s", SERIAL, "shell", "pm", "list", "packages", "-3"]
    };
    mock.expect(&arguments, MockResponse::output(listing));
}

/// Expects the standby bucket read of one application.
fn expect_bucket(mock: &MockAdb, package: &str, bucket: &str) {
    mock.expect(
        &["-s", SERIAL, "shell", "am", "get-standby-bucket", package],
        MockResponse::output(bucket),
    );
}

/// Returns whether the checkbox of the strip is ticked.
fn checkbox_checked(panel: &ControlPanel) -> bool {
    for line in overlay_lines(panel) {
        if line.is_checkbox() {
            return line.is_checked();
        }
    }
    return false;
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
        Focus::RemovePackages,
        Focus::StandbyCratesOverlay,
        Focus::StandbyCratesContent,
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
        Focus::StandbyCratesContent,
        Focus::StandbyCratesOverlay,
        Focus::RemovePackages,
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
fn keeps_both_standby_targets_in_one_panel() {
    assert_eq!(Focus::StandbyCratesOverlay.panel(), Panel::StandbyCrates);
    assert_eq!(Focus::StandbyCratesContent.panel(), Panel::StandbyCrates);
    assert_ne!(Focus::StandbyCratesOverlay, Focus::StandbyCratesContent);

    assert_eq!(Focus::DeviceSelection.panel(), Panel::DeviceSelection);
    assert_eq!(Focus::RemovePackages.panel(), Panel::RemovePackages);
    assert_eq!(Focus::Operations.panel(), Panel::Operations);
}

#[test]
fn marks_the_area_that_holds_the_focus() {
    let mock = MockAdb::new();
    let panel = panel_for(&mock, ANDROID_13);

    for line in focused_overlay_lines(&panel) {
        assert!(line.is_focused(), "the focused strip has no focus mark");
    }
    for line in overlay_lines(&panel) {
        assert!(!line.is_focused(), "the unfocused strip looks focused");
    }

    // The cursor of the sections is shown only while the sections have focus.
    let mut marked = 0;
    for line in panel.lines(false) {
        if line.is_focused() {
            marked += 1;
        }
    }
    assert_eq!(marked, 0, "the sections showed a cursor without focus");

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
fn toggles_the_checkbox_only_from_the_overlay() {
    let mock = MockAdb::new();
    expect_listing(&mock, &["com.example.alpha"], false);
    expect_bucket(&mock, "com.example.alpha", "active");
    // Ticking the checkbox reads the inventory again, this time unfiltered.
    expect_listing(&mock, &["com.android.systemui", "com.example.alpha"], true);
    expect_bucket(&mock, "com.android.systemui", "active");
    expect_bucket(&mock, "com.example.alpha", "active");

    let mut panel = panel_for(&mock, ANDROID_13);

    // SPACE from the section content expands the section and leaves the
    // checkbox alone.
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for_rows(&mut panel, 1);
    assert_eq!(header_expansion(&panel), vec![true, false, false, false]);
    assert!(!checkbox_checked(&panel));

    // SPACE from the overlay ticks the checkbox and refreshes the inventory.
    assert!(panel.handle_overlay_key(KeyCode::Char(' ')));
    wait_for_rows(&mut panel, 2);
    assert!(checkbox_checked(&panel));
    mock.assert_complete();
}

#[test]
fn does_not_change_applications_from_the_overlay() {
    let mock = MockAdb::new();
    expect_listing(&mock, &["com.example.alpha"], false);
    expect_bucket(&mock, "com.example.alpha", "active");

    let mut panel = panel_for(&mock, ANDROID_13);
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for_rows(&mut panel, 1);
    // The cursor moves from the header onto the application.
    assert!(panel.handle_key(KeyCode::Down));
    assert_eq!(
        focused_text(&panel),
        Some(String::from("com.example.alpha  active"))
    );

    let commands = mock.calls().len();

    // The arrows belong to the sections, so the overlay ignores them and the
    // application keeps its value.
    assert!(!panel.handle_overlay_key(KeyCode::Left));
    assert!(!panel.handle_overlay_key(KeyCode::Right));
    assert!(!panel.handle_overlay_key(KeyCode::Up));
    assert!(!panel.handle_overlay_key(KeyCode::Down));

    assert_eq!(mock.calls().len(), commands, "the overlay sent a command");
    assert_eq!(
        entry_texts(&panel),
        vec![String::from("com.example.alpha  active")]
    );
    mock.assert_complete();
}

#[test]
fn keeps_the_expansion_and_the_selection_across_the_focus_targets() {
    let mock = MockAdb::new();
    expect_listing(&mock, &["com.example.alpha", "com.example.mike"], false);
    expect_bucket(&mock, "com.example.alpha", "active");
    expect_bucket(&mock, "com.example.mike", "working_set");
    expect_listing(&mock, &["com.example.alpha", "com.example.mike"], true);
    expect_bucket(&mock, "com.example.alpha", "active");
    expect_bucket(&mock, "com.example.mike", "working_set");

    let mut panel = panel_for(&mock, ANDROID_13);
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for_rows(&mut panel, 2);
    assert!(panel.handle_key(KeyCode::Down));

    // Keys the overlay does not use leave the sections as they are.
    assert!(!panel.handle_overlay_key(KeyCode::Up));
    assert_eq!(header_expansion(&panel), vec![true, false, false, false]);
    assert_eq!(
        focused_text(&panel),
        Some(String::from("com.example.alpha  active"))
    );

    // Changing the filter from the overlay keeps the expansion and puts the
    // cursor back on the same application.
    assert!(panel.handle_overlay_key(KeyCode::Char(' ')));
    wait_for(&mut panel, |_| {
        return mock.calls().len() == 6;
    });

    assert_eq!(header_expansion(&panel), vec![true, false, false, false]);
    assert_eq!(
        focused_text(&panel),
        Some(String::from("com.example.alpha  active"))
    );
    mock.assert_complete();
}
