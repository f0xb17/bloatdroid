//! Tests for the global system application filter.
//!
//! The filter belongs to the application, so these tests hand it to the panel
//! the way the interface does and check that every application list follows it.
//! Nothing here needs a device: every ADB command goes through the mock.

use crossterm::event::KeyCode;

use bloatdroid::control::ControlPanel;
use bloatdroid::log::LogLevel;

use crate::common::MockAdb;
use crate::common::MockResponse;

use super::harness::ANDROID_13;
use super::harness::SERIAL;
use super::harness::entry_texts;
use super::harness::log_has;
use super::harness::panel_for;
use super::harness::set_filter;
use super::harness::status_texts;
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

/// Returns how often the package list of a device was read with a filter.
fn listings(mock: &MockAdb, include_system: bool) -> usize {
    let mut wanted = vec![
        String::from("-s"),
        String::from(SERIAL),
        String::from("shell"),
        String::from("pm"),
        String::from("list"),
        String::from("packages"),
    ];
    if !include_system {
        wanted.push(String::from("-3"));
    }

    let mut count = 0;
    for call in mock.calls() {
        if call == wanted {
            count += 1;
        }
    }
    return count;
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

/// Polls until the expanded section shows the expected applications.
fn wait_for_entries(panel: &mut ControlPanel, count: usize) {
    wait_for(panel, |panel| {
        return entry_texts(panel).len() == count;
    });
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
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for_entries(&mut panel, THIRD_PARTY.len());

    let expected = vec![
        String::from("com.example.alpha  active"),
        String::from("com.example.bravo  active"),
    ];
    assert_eq!(entry_texts(&panel), expected);
    assert_eq!(listings(&mock, false), 1);
    assert_eq!(listings(&mock, true), 0);
    mock.assert_complete();
}

#[test]
fn includes_system_applications_when_the_filter_is_enabled() {
    let mock = MockAdb::new();
    expect_listing(&mock, &THIRD_PARTY, false);
    expect_buckets(&mock, &THIRD_PARTY);
    expect_listing(&mock, &ALL_APPLICATIONS, true);
    expect_buckets(&mock, &ALL_APPLICATIONS);

    let mut panel = panel_for(&mock, ANDROID_13);
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for_entries(&mut panel, THIRD_PARTY.len());

    assert!(set_filter(&mut panel, true));
    wait_for_entries(&mut panel, ALL_APPLICATIONS.len());

    let expected = vec![
        String::from("com.android.systemui  active"),
        String::from("com.example.alpha  active"),
        String::from("com.example.bravo  active"),
    ];
    assert_eq!(entry_texts(&panel), expected);
    assert_eq!(listings(&mock, true), 1);
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
    wait_for_entries(&mut panel, THIRD_PARTY.len());
    assert!(panel.handle_key(KeyCode::Char(' ')));

    assert!(set_filter(&mut panel, true));
    wait_for(&mut panel, |_| {
        return listings(&mock, true) == 1;
    });

    // Walk the sections from the bottom up: Doze Whitelist, App Inactive,
    // AppOps and Standby Buckets. Every one of them works from the same
    // filtered inventory.
    for _ in 0..3 {
        assert!(panel.handle_key(KeyCode::Down));
    }
    for _ in 0..3 {
        check_focused_section(&mut panel);
        assert!(panel.handle_key(KeyCode::Up));
    }
    check_focused_section(&mut panel);
    mock.assert_complete();
}

#[test]
fn reads_the_inventory_once_per_filter_change() {
    let mock = MockAdb::new();
    expect_listing(&mock, &THIRD_PARTY, false);
    expect_listing(&mock, &ALL_APPLICATIONS, true);

    let mut panel = panel_for(&mock, ANDROID_13);
    wait_for(&mut panel, |panel| {
        return log_has(panel, LogLevel::Success, "Found 2 applications.");
    });

    assert!(set_filter(&mut panel, true));
    wait_for(&mut panel, |panel| {
        return log_has(panel, LogLevel::Success, "Found 3 applications.");
    });

    // Polling again neither repeats the read nor starts another one, and the
    // setting itself cannot be changed twice.
    for _ in 0..20 {
        panel.poll();
    }
    assert!(!set_filter(&mut panel, true));
    assert_eq!(listings(&mock, false), 1);
    assert_eq!(listings(&mock, true), 1);
    mock.assert_complete();
}

#[test]
fn keeps_the_inventory_when_a_refresh_fails() {
    let mock = MockAdb::new();
    expect_listing(&mock, &THIRD_PARTY, false);
    expect_buckets(&mock, &THIRD_PARTY);
    // The refresh cannot read the list, so the applications that are already
    // there have to stay.
    mock.expect(
        &["-s", SERIAL, "shell", "pm", "list", "packages", "-3"],
        MockResponse::failure("error: device offline"),
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for_entries(&mut panel, THIRD_PARTY.len());

    assert!(panel.refresh_inventory());
    wait_for(&mut panel, |panel| {
        for status in status_texts(panel) {
            if status.contains("could not list applications") {
                return true;
            }
        }
        return false;
    });

    let expected = vec![
        String::from("com.example.alpha  active"),
        String::from("com.example.bravo  active"),
    ];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}
