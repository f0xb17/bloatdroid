//! Tests for the control strip and the system application filter.

use crossterm::event::KeyCode;

use bloatdroid::control::ControlPanel;

use crate::common::MockAdb;
use crate::common::MockResponse;

use super::harness::ANDROID_13;
use super::harness::SERIAL;
use super::harness::entry_texts;
use super::harness::focused_text;
use super::harness::overlay_is_focused;
use super::harness::overlay_lines;
use super::harness::panel_for;
use super::harness::wait_for;

/// The applications that the device reports as installed by the user.
const THIRD_PARTY: [&str; 2] = ["com.example.alpha", "com.example.bravo"];

/// The application that the device reports as a system application.
const SYSTEM: &str = "com.android.systemui";

/// Every application, with the system application sorted first.
const ALL_APPLICATIONS: [&str; 3] =
    [SYSTEM, "com.example.alpha", "com.example.bravo"];

/// The AppOps operation that the sections read.
const OPERATION: &str = "RUN_ANY_IN_BACKGROUND";

/// Expects the package listing of one device.
fn expect_listing(mock: &MockAdb, applications: &[&str], include_system: bool) {
    let mut listing = String::new();
    for package in applications {
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

/// Expects the standby bucket read of every given application.
fn expect_buckets(mock: &MockAdb, applications: &[&str]) {
    for package in applications {
        mock.expect(
            &["-s", SERIAL, "shell", "am", "get-standby-bucket", package],
            MockResponse::output("active"),
        );
    }
}

/// Expects the AppOps read of every given application.
fn expect_appops(mock: &MockAdb, applications: &[&str]) {
    for package in applications {
        mock.expect(
            &[
                "-s", SERIAL, "shell", "cmd", "appops", "get", package,
                OPERATION,
            ],
            MockResponse::output("RUN_ANY_IN_BACKGROUND: allow\n"),
        );
    }
}

/// Expects the inactive state read of every given application.
fn expect_inactive(mock: &MockAdb, applications: &[&str]) {
    for package in applications {
        mock.expect(
            &["-s", SERIAL, "shell", "am", "get-inactive", package],
            MockResponse::output("Idle=false\n"),
        );
    }
}

/// Expects one read of the complete whitelist.
fn expect_whitelist(mock: &MockAdb) {
    mock.expect(
        &["-s", SERIAL, "shell", "cmd", "deviceidle", "whitelist"],
        MockResponse::output(""),
    );
}

/// Returns the text of every line of the control strip.
fn overlay_texts(panel: &ControlPanel) -> Vec<String> {
    let mut texts = Vec::new();
    for line in overlay_lines(panel) {
        texts.push(String::from(line.text()));
    }
    return texts;
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

/// Returns whether the expanded section lists the system application.
fn lists_system_application(panel: &ControlPanel) -> bool {
    for row in entry_texts(panel) {
        if row.starts_with(SYSTEM) {
            return true;
        }
    }
    return false;
}

/// Moves the cursor down until the control strip holds it.
fn focus_overlay(panel: &mut ControlPanel) {
    for _ in 0..20 {
        if overlay_is_focused(panel) {
            return;
        }
        assert!(panel.handle_key(KeyCode::Down));
    }
    panic!("the control strip was never reached");
}

/// Ticks the checkbox that includes the system applications.
fn show_system_apps(panel: &mut ControlPanel) {
    focus_overlay(panel);
    assert!(panel.handle_key(KeyCode::Char(' ')));
    assert!(checkbox_checked(panel));
}

/// Expands the focused section, waits for its rows, and collapses it again.
fn check_focused_section(panel: &mut ControlPanel) {
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for(panel, |panel| {
        return entry_texts(panel).len() == ALL_APPLICATIONS.len()
            && lists_system_application(panel);
    });
    assert!(panel.handle_key(KeyCode::Char(' ')));
}

#[test]
fn lists_only_third_party_applications_by_default() {
    let mock = MockAdb::new();
    expect_listing(&mock, &THIRD_PARTY, false);
    expect_buckets(&mock, &THIRD_PARTY);

    let mut panel = panel_for(&mock, ANDROID_13);
    assert!(!checkbox_checked(&panel));
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for(&mut panel, |panel| {
        return entry_texts(panel).len() == THIRD_PARTY.len();
    });

    let expected = vec![
        String::from("com.example.alpha  (active)"),
        String::from("com.example.bravo  (active)"),
    ];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn reaches_the_checkbox_and_shows_system_applications() {
    let mock = MockAdb::new();
    expect_listing(&mock, &THIRD_PARTY, false);
    expect_buckets(&mock, &THIRD_PARTY);
    // Ticking the checkbox reads the inventory again, this time unfiltered.
    expect_listing(&mock, &ALL_APPLICATIONS, true);
    expect_buckets(&mock, &ALL_APPLICATIONS);

    let mut panel = panel_for(&mock, ANDROID_13);
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for(&mut panel, |panel| {
        return entry_texts(panel).len() == THIRD_PARTY.len();
    });

    focus_overlay(&mut panel);
    // The strip is the only row the cursor can reach, so no content row is
    // focused while it holds the cursor.
    assert_eq!(focused_text(&panel), None);
    assert!(!checkbox_checked(&panel));

    assert!(panel.handle_key(KeyCode::Char(' ')));
    assert!(checkbox_checked(&panel));

    wait_for(&mut panel, |panel| {
        return entry_texts(panel).len() == ALL_APPLICATIONS.len();
    });
    let expected = vec![
        String::from("com.android.systemui  (active)"),
        String::from("com.example.alpha  (active)"),
        String::from("com.example.bravo  (active)"),
    ];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn applies_the_filter_to_every_section() {
    let mock = MockAdb::new();
    expect_listing(&mock, &THIRD_PARTY, false);
    expect_buckets(&mock, &THIRD_PARTY);
    expect_listing(&mock, &ALL_APPLICATIONS, true);
    expect_buckets(&mock, &ALL_APPLICATIONS);
    expect_appops(&mock, &ALL_APPLICATIONS);
    expect_inactive(&mock, &ALL_APPLICATIONS);
    expect_whitelist(&mock);

    let mut panel = panel_for(&mock, ANDROID_13);
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for(&mut panel, |panel| {
        return entry_texts(panel).len() == THIRD_PARTY.len();
    });
    assert!(panel.handle_key(KeyCode::Char(' ')));

    show_system_apps(&mut panel);

    // Doze Whitelist, App Inactive, AppOps and Standby Buckets, bottom up.
    for _ in 0..4 {
        assert!(panel.handle_key(KeyCode::Up));
        check_focused_section(&mut panel);
    }
    mock.assert_complete();
}

#[test]
fn leaves_the_applications_alone_while_the_checkbox_has_focus() {
    let mock = MockAdb::new();
    expect_listing(&mock, &THIRD_PARTY, false);
    expect_buckets(&mock, &THIRD_PARTY);

    let mut panel = panel_for(&mock, ANDROID_13);
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for(&mut panel, |panel| {
        return entry_texts(panel).len() == THIRD_PARTY.len();
    });
    focus_overlay(&mut panel);

    // The arrows belong to the strip while it holds the cursor, so no bucket
    // of an application is changed and no command is sent.
    assert!(!panel.handle_key(KeyCode::Left));
    assert!(!panel.handle_key(KeyCode::Right));

    let expected = vec![
        String::from("com.example.alpha  (active)"),
        String::from("com.example.bravo  (active)"),
    ];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn keeps_the_strip_visible_while_the_content_scrolls() {
    let mock = MockAdb::new();
    expect_listing(&mock, &THIRD_PARTY, false);
    expect_buckets(&mock, &THIRD_PARTY);

    let mut panel = panel_for(&mock, ANDROID_13);
    let strip = overlay_texts(&panel);
    assert_eq!(strip, vec![String::from("Show System Apps")]);

    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for(&mut panel, |panel| {
        return entry_texts(panel).len() == THIRD_PARTY.len();
    });
    assert!(panel.set_viewport(2));

    for _ in 0..5 {
        assert!(panel.handle_key(KeyCode::Down));
    }

    // Scrolling never moves the strip, and the strip is never part of the
    // scrollable content.
    assert_eq!(overlay_texts(&panel), strip);
    assert!(!overlay_is_focused(&panel));
    for line in panel.lines() {
        assert!(!line.is_checkbox(), "the strip leaked into the content");
    }
    mock.assert_complete();
}

#[test]
fn refreshes_the_sections_when_the_filter_changes() {
    let mock = MockAdb::new();
    expect_listing(&mock, &THIRD_PARTY, false);
    expect_buckets(&mock, &THIRD_PARTY);
    expect_listing(&mock, &ALL_APPLICATIONS, true);
    expect_buckets(&mock, &ALL_APPLICATIONS);

    let mut panel = panel_for(&mock, ANDROID_13);
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for(&mut panel, |panel| {
        return entry_texts(panel).len() == THIRD_PARTY.len();
    });

    show_system_apps(&mut panel);
    wait_for(&mut panel, |panel| {
        return entry_texts(panel).len() == ALL_APPLICATIONS.len();
    });

    // The cursor stays on the strip, which is a valid position for the new
    // inventory, and the section shows the refreshed list.
    assert!(overlay_is_focused(&panel));
    assert!(lists_system_application(&panel));
    mock.assert_complete();
}
