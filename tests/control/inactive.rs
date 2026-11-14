//! Tests for the App Inactive section and the states it toggles.

use crossterm::event::KeyCode;

use bloatdroid::control::ControlPanel;
use bloatdroid::control::ControlTarget;
use bloatdroid::control::inactive::InactiveState;

use crate::common::MockAdb;
use crate::common::MockResponse;

use super::harness::ANDROID_13;
use super::harness::SERIAL;
use super::harness::entry_texts;
use super::harness::panel_for;
use super::harness::status_texts;
use super::harness::wait_for;
use super::harness::wait_for_commands;

/// The serial of the second device used by the device switching tests.
const OTHER_SERIAL: &str = "other-serial";

/// Expects the package listing of the given applications.
fn expect_listing(mock: &MockAdb, serial: &str, applications: &[(&str, &str)]) {
    let mut listing = String::new();
    for &(package, _output) in applications {
        listing.push_str("package:");
        listing.push_str(package);
        listing.push('\n');
    }
    mock.expect(
        &["-s", serial, "shell", "pm", "list", "packages", "-3"],
        MockResponse::output(listing),
    );
}

/// Expects the state read of every given application.
fn expect_states(mock: &MockAdb, serial: &str, applications: &[(&str, &str)]) {
    for &(package, output) in applications {
        mock.expect(
            &["-s", serial, "shell", "am", "get-inactive", package],
            MockResponse::output(output),
        );
    }
}

/// Expects the package listing and the state of every application.
fn expect_load(mock: &MockAdb, serial: &str, applications: &[(&str, &str)]) {
    expect_listing(mock, serial, applications);
    expect_states(mock, serial, applications);
}

/// Expects a state change and the read that verifies it.
fn expect_change(
    mock: &MockAdb,
    serial: &str,
    package: &str,
    flag: &str,
    reported: &str,
) {
    mock.expect(
        &["-s", serial, "shell", "am", "set-inactive", package, flag],
        MockResponse::output(""),
    );
    mock.expect(
        &["-s", serial, "shell", "am", "get-inactive", package],
        MockResponse::output(reported),
    );
}

/// Expands the App Inactive section, which follows the AppOps header.
fn expand_inactive(panel: &mut ControlPanel) {
    assert!(panel.handle_key(KeyCode::Down));
    assert!(panel.handle_key(KeyCode::Down));
    assert!(panel.handle_key(KeyCode::Char(' ')));
}

/// Moves the cursor from the section header onto the first application.
fn focus_first_application(panel: &mut ControlPanel) {
    assert!(panel.handle_key(KeyCode::Down));
}

/// Polls until the expanded section shows the expected number of rows.
fn wait_for_rows(panel: &mut ControlPanel, count: usize) {
    wait_for(panel, |panel| {
        return entry_texts(panel).len() == count;
    });
}

/// Returns `true` when a status row contains the given text.
fn status_contains(panel: &ControlPanel, text: &str) -> bool {
    for status in status_texts(panel) {
        if status.contains(text) {
            return true;
        }
    }
    return false;
}

/// Returns the only status row of the panel.
fn only_status(panel: &ControlPanel) -> String {
    let statuses = status_texts(panel);
    assert_eq!(statuses.len(), 1, "expected exactly one status row");
    match statuses.first() {
        Some(status) => return String::from(status.as_str()),
        None => panic!("the status row disappeared"),
    }
}

/// Returns how often the package list of a device was read.
fn package_listings(mock: &MockAdb, serial: &str) -> usize {
    let wanted = vec![
        String::from("-s"),
        String::from(serial),
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

#[test]
fn parses_the_reported_boolean() {
    assert_eq!(InactiveState::from_flag("true"), Some(InactiveState::Inactive));
    assert_eq!(InactiveState::from_flag("false"), Some(InactiveState::Active));

    assert_eq!(InactiveState::Inactive.name(), "inactive");
    assert_eq!(InactiveState::Active.name(), "active");
    assert_eq!(
        InactiveState::from_name("inactive"),
        Some(InactiveState::Inactive)
    );
    assert_eq!(InactiveState::from_name("active"), Some(InactiveState::Active));
}

#[test]
fn maps_states_to_the_boolean_flag() {
    assert_eq!(InactiveState::Inactive.flag(), "true");
    assert_eq!(InactiveState::Active.flag(), "false");
}

#[test]
fn rejects_unknown_and_malformed_states() {
    assert_eq!(InactiveState::from_flag("maybe"), None);
    assert_eq!(InactiveState::from_flag(""), None);
    assert_eq!(InactiveState::from_flag("1"), None);
    assert_eq!(InactiveState::from_name("unknown"), None);
}

#[test]
fn toggles_between_the_two_states() {
    assert_eq!(InactiveState::Active.toggled(), InactiveState::Inactive);
    assert_eq!(InactiveState::Inactive.toggled(), InactiveState::Active);
}

#[test]
fn loads_states_sorted_by_package_name() {
    let mock = MockAdb::new();
    expect_load(
        &mock,
        SERIAL,
        &[
            ("com.example.zulu", "Idle=true\n"),
            ("com.example.alpha", "Idle=false\n"),
            ("com.example.mike", "Idle=true\n"),
        ],
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_inactive(&mut panel);
    wait_for_rows(&mut panel, 3);

    let expected = vec![
        String::from("com.example.alpha  (active)"),
        String::from("com.example.mike  (inactive)"),
        String::from("com.example.zulu  (inactive)"),
    ];
    assert_eq!(entry_texts(&panel), expected);
    assert!(status_texts(&panel).is_empty());
    mock.assert_complete();
}

#[test]
fn skips_lines_that_do_not_report_the_state() {
    let mock = MockAdb::new();
    expect_load(
        &mock,
        SERIAL,
        &[("com.example.app", "Warning: activity not started\nIdle=true\n")],
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_inactive(&mut panel);
    wait_for_rows(&mut panel, 1);

    let expected = vec![String::from("com.example.app  (inactive)")];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn treats_empty_and_unreadable_states_as_unknown() {
    let mock = MockAdb::new();
    expect_load(
        &mock,
        SERIAL,
        &[("com.example.empty", ""), ("com.example.odd", "Idle=maybe\n")],
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_inactive(&mut panel);
    wait_for_rows(&mut panel, 2);

    let expected = vec![
        String::from("com.example.empty  (unknown)"),
        String::from("com.example.odd  (unknown)"),
    ];
    assert_eq!(entry_texts(&panel), expected);
    assert!(only_status(&panel).contains("2 applications could not be read"));
    mock.assert_complete();
}

#[test]
fn reports_listing_failures() {
    let mock = MockAdb::new();
    mock.expect(
        &["-s", SERIAL, "shell", "pm", "list", "packages", "-3"],
        MockResponse::failure("error: device offline"),
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_inactive(&mut panel);
    wait_for(&mut panel, |panel| {
        return status_contains(panel, "could not list applications");
    });

    assert!(entry_texts(&panel).is_empty());
    assert!(only_status(&panel).contains("could not list applications"));
    mock.assert_complete();
}

#[test]
fn toggles_to_inactive_and_verifies_it() {
    let mock = MockAdb::new();
    expect_load(&mock, SERIAL, &[("com.example.app", "Idle=false\n")]);
    expect_change(&mock, SERIAL, "com.example.app", "true", "Idle=true\n");

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_inactive(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Right));
    wait_for_commands(&mut panel, &mock);

    let expected = vec![String::from("com.example.app  (inactive)")];
    assert_eq!(entry_texts(&panel), expected);
    assert!(status_texts(&panel).is_empty());
    mock.assert_complete();
}

#[test]
fn toggles_to_active_and_verifies_it() {
    let mock = MockAdb::new();
    expect_load(&mock, SERIAL, &[("com.example.app", "Idle=true\n")]);
    expect_change(&mock, SERIAL, "com.example.app", "false", "Idle=false\n");

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_inactive(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Left));
    wait_for_commands(&mut panel, &mock);

    let expected = vec![String::from("com.example.app  (active)")];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn ignores_changes_for_unknown_states() {
    let mock = MockAdb::new();
    expect_load(&mock, SERIAL, &[("com.example.app", "Idle=maybe\n")]);

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_inactive(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    // The opposite of an unknown state is unknown, so no command is sent.
    assert!(!panel.handle_key(KeyCode::Right));
    assert!(!panel.handle_key(KeyCode::Left));

    let expected = vec![String::from("com.example.app  (unknown)")];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn ignores_changes_while_another_is_pending() {
    let mock = MockAdb::new();
    expect_load(&mock, SERIAL, &[("com.example.app", "Idle=false\n")]);
    expect_change(&mock, SERIAL, "com.example.app", "true", "Idle=true\n");

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_inactive(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Right));
    // The first change is still in flight, so this one is ignored.
    assert!(!panel.handle_key(KeyCode::Left));
    wait_for_commands(&mut panel, &mock);

    let expected = vec![String::from("com.example.app  (inactive)")];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn reverts_changes_that_the_device_did_not_apply() {
    let mock = MockAdb::new();
    expect_load(&mock, SERIAL, &[("com.example.app", "Idle=false\n")]);
    expect_change(&mock, SERIAL, "com.example.app", "true", "Idle=false\n");

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_inactive(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Right));
    wait_for(&mut panel, |panel| {
        return status_contains(panel, "kept");
    });

    let expected = vec![String::from("com.example.app  (active)")];
    assert_eq!(entry_texts(&panel), expected);
    assert!(only_status(&panel).contains("kept"));
    mock.assert_complete();
}

#[test]
fn reverts_changes_that_fail() {
    let mock = MockAdb::new();
    expect_load(&mock, SERIAL, &[("com.example.app", "Idle=false\n")]);
    mock.expect(
        &[
            "-s",
            SERIAL,
            "shell",
            "am",
            "set-inactive",
            "com.example.app",
            "true",
        ],
        MockResponse::failure("java.lang.SecurityException"),
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_inactive(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Right));
    wait_for(&mut panel, |panel| {
        return status_contains(panel, "rejected");
    });

    let expected = vec![String::from("com.example.app  (active)")];
    assert_eq!(entry_texts(&panel), expected);
    assert!(only_status(&panel).contains("rejected"));
    mock.assert_complete();
}

#[test]
fn reverts_changes_that_cannot_be_verified() {
    let mock = MockAdb::new();
    expect_load(&mock, SERIAL, &[("com.example.app", "Idle=false\n")]);
    expect_change(&mock, SERIAL, "com.example.app", "true", "Idle=maybe\n");

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_inactive(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Right));
    wait_for(&mut panel, |panel| {
        return status_contains(panel, "could not be read");
    });

    let expected = vec![String::from("com.example.app  (active)")];
    assert_eq!(entry_texts(&panel), expected);
    assert!(only_status(&panel).contains("could not be read"));
    mock.assert_complete();
}

#[test]
fn loads_states_only_when_expanded() {
    let mock = MockAdb::new();
    expect_load(&mock, SERIAL, &[("com.example.app", "Idle=false\n")]);

    let mut panel = panel_for(&mock, ANDROID_13);

    // The package inventory is read as soon as the panel polls, but no state is
    // read while the section stays collapsed.
    wait_for(&mut panel, |_| {
        return package_listings(&mock, SERIAL) == 1;
    });
    for _ in 0..20 {
        panel.poll();
    }
    assert_eq!(mock.calls().len(), 1);

    expand_inactive(&mut panel);
    wait_for_rows(&mut panel, 1);

    for _ in 0..20 {
        panel.poll();
    }
    assert_eq!(package_listings(&mock, SERIAL), 1);
    assert_eq!(mock.calls().len(), 2);
    mock.assert_complete();
}

#[test]
fn reloads_states_after_collapsing_and_expanding() {
    let mock = MockAdb::new();
    expect_load(&mock, SERIAL, &[("com.example.alpha", "Idle=false\n")]);
    // Expanding again reads the states once more from the shared inventory, so
    // the package list is not read a second time.
    expect_states(&mock, SERIAL, &[("com.example.alpha", "Idle=true\n")]);

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_inactive(&mut panel);
    wait_for_rows(&mut panel, 1);
    assert_eq!(package_listings(&mock, SERIAL), 1);

    assert!(panel.handle_key(KeyCode::Char(' ')));
    panel.poll();
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for_commands(&mut panel, &mock);

    assert_eq!(package_listings(&mock, SERIAL), 1);
    let expected = vec![String::from("com.example.alpha  (inactive)")];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn discards_a_pending_change_when_the_device_changes() {
    let mock = MockAdb::new();
    expect_load(&mock, SERIAL, &[("com.example.app", "Idle=false\n")]);
    expect_change(&mock, SERIAL, "com.example.app", "true", "Idle=true\n");
    expect_load(&mock, OTHER_SERIAL, &[("com.example.other", "Idle=true\n")]);

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_inactive(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);
    assert!(panel.handle_key(KeyCode::Right));

    let target = ControlTarget::new(
        String::from(OTHER_SERIAL),
        String::from(ANDROID_13),
    );
    assert!(panel.set_target(Some(target)));

    wait_for(&mut panel, |panel| {
        let expected = vec![String::from("com.example.other  (inactive)")];
        return entry_texts(panel) == expected;
    });
    assert!(status_texts(&panel).is_empty());
    mock.assert_complete();
}

#[test]
fn ignores_results_from_a_previous_device() {
    let mock = MockAdb::new();
    // The inventory of the first device is read, but the device changes before
    // any of its states is, so no state is ever read for it.
    expect_listing(&mock, SERIAL, &[("com.example.first", "Idle=false\n")]);
    expect_load(&mock, OTHER_SERIAL, &[("com.example.second", "Idle=true\n")]);

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_inactive(&mut panel);
    // The inventory for the first device is requested, then the device changes.
    panel.poll();

    let target = ControlTarget::new(
        String::from(OTHER_SERIAL),
        String::from(ANDROID_13),
    );
    assert!(panel.set_target(Some(target)));

    wait_for(&mut panel, |panel| {
        let expected = vec![String::from("com.example.second  (inactive)")];
        return entry_texts(panel) == expected;
    });
    assert_eq!(package_listings(&mock, SERIAL), 1);
    assert_eq!(package_listings(&mock, OTHER_SERIAL), 1);
    // Both listings and the one state read of the new device.
    assert_eq!(mock.calls().len(), 3);
    mock.assert_complete();
}
