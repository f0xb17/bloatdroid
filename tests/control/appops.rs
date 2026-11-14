//! Tests for the AppOps section and the modes it cycles.

use crossterm::event::KeyCode;

use bloatdroid::control::ControlPanel;
use bloatdroid::control::ControlTarget;
use bloatdroid::control::appops::AppOpMode;

use crate::common::MockAdb;
use crate::common::MockResponse;

use super::harness::ANDROID_13;
use super::harness::SERIAL;
use super::harness::entry_texts;
use super::harness::panel_for;
use super::harness::status_texts;
use super::harness::wait_for;
use super::harness::wait_for_commands;

/// The AppOps operation that every test reads and writes.
const OPERATION: &str = "RUN_ANY_IN_BACKGROUND";

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

/// Expects the mode read of every given application.
fn expect_states(mock: &MockAdb, serial: &str, applications: &[(&str, &str)]) {
    for &(package, output) in applications {
        mock.expect(
            &[
                "-s", serial, "shell", "cmd", "appops", "get", package,
                OPERATION,
            ],
            MockResponse::output(output),
        );
    }
}

/// Expects the package listing and the mode of every application.
fn expect_load(mock: &MockAdb, serial: &str, applications: &[(&str, &str)]) {
    expect_listing(mock, serial, applications);
    expect_states(mock, serial, applications);
}

/// Expects a mode change and the read that verifies it.
fn expect_change(
    mock: &MockAdb,
    serial: &str,
    package: &str,
    mode: &str,
    reported: &str,
) {
    mock.expect(
        &[
            "-s", serial, "shell", "cmd", "appops", "set", package, OPERATION,
            mode,
        ],
        MockResponse::output(""),
    );
    mock.expect(
        &["-s", serial, "shell", "cmd", "appops", "get", package, OPERATION],
        MockResponse::output(reported),
    );
}

/// Expands the AppOps section, which follows the Standby Buckets header.
fn expand_appops(panel: &mut ControlPanel) {
    assert!(panel.handle_key(KeyCode::Down));
    assert!(panel.handle_key(KeyCode::Char(' ')));
}

/// Moves the cursor from the AppOps header onto the first application.
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
fn parses_the_supported_modes() {
    assert_eq!(AppOpMode::from_name("allow"), Some(AppOpMode::Allow));
    assert_eq!(AppOpMode::from_name("default"), Some(AppOpMode::Default));
    assert_eq!(AppOpMode::from_name("ignore"), Some(AppOpMode::Ignore));

    assert_eq!(AppOpMode::Allow.name(), "allow");
    assert_eq!(AppOpMode::Default.name(), "default");
    assert_eq!(AppOpMode::Ignore.name(), "ignore");
}

#[test]
fn rejects_unknown_and_malformed_modes() {
    assert_eq!(AppOpMode::from_name("maybe"), None);
    assert_eq!(AppOpMode::from_name(""), None);
    assert_eq!(AppOpMode::from_name("ALLOW"), None);
    assert_eq!(AppOpMode::from_name("allow "), None);
}

#[test]
fn cycles_forward_through_every_mode() {
    assert_eq!(AppOpMode::Allow.next(), AppOpMode::Default);
    assert_eq!(AppOpMode::Default.next(), AppOpMode::Ignore);
    assert_eq!(AppOpMode::Ignore.next(), AppOpMode::Allow);
}

#[test]
fn cycles_backward_through_every_mode() {
    assert_eq!(AppOpMode::Allow.previous(), AppOpMode::Ignore);
    assert_eq!(AppOpMode::Ignore.previous(), AppOpMode::Default);
    assert_eq!(AppOpMode::Default.previous(), AppOpMode::Allow);
}

#[test]
fn loads_applications_sorted_by_package_name() {
    let mock = MockAdb::new();
    expect_load(
        &mock,
        SERIAL,
        &[
            ("com.example.zulu", "RUN_ANY_IN_BACKGROUND: allow\n"),
            ("com.example.alpha", "RUN_ANY_IN_BACKGROUND: ignore\n"),
            ("com.example.mike", "RUN_ANY_IN_BACKGROUND: default\n"),
        ],
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_appops(&mut panel);
    wait_for_rows(&mut panel, 3);

    let expected = vec![
        String::from("com.example.alpha  (ignore)"),
        String::from("com.example.mike  (default)"),
        String::from("com.example.zulu  (allow)"),
    ];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn treats_a_missing_operation_as_default() {
    let mock = MockAdb::new();
    expect_load(&mock, SERIAL, &[("com.example.app", "")]);

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_appops(&mut panel);
    wait_for_rows(&mut panel, 1);

    let expected = vec![String::from("com.example.app  (default)")];
    assert_eq!(entry_texts(&panel), expected);
    assert!(status_texts(&panel).is_empty());
    mock.assert_complete();
}

#[test]
fn reads_the_mode_of_a_verbose_operation_entry() {
    let mock = MockAdb::new();
    expect_load(
        &mock,
        SERIAL,
        &[(
            "com.example.app",
            "RUN_ANY_IN_BACKGROUND: ignore; time=+1h2m3s;\n",
        )],
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_appops(&mut panel);
    wait_for_rows(&mut panel, 1);

    let expected = vec![String::from("com.example.app  (ignore)")];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn reports_applications_whose_mode_cannot_be_read() {
    let mock = MockAdb::new();
    mock.expect(
        &["-s", SERIAL, "shell", "pm", "list", "packages", "-3"],
        MockResponse::output(
            "package:com.example.broken\npackage:com.example.odd\n",
        ),
    );
    mock.expect(
        &[
            "-s",
            SERIAL,
            "shell",
            "cmd",
            "appops",
            "get",
            "com.example.broken",
            OPERATION,
        ],
        MockResponse::failure("error: device offline"),
    );
    mock.expect(
        &[
            "-s",
            SERIAL,
            "shell",
            "cmd",
            "appops",
            "get",
            "com.example.odd",
            OPERATION,
        ],
        MockResponse::output("RUN_ANY_IN_BACKGROUND: maybe\n"),
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_appops(&mut panel);
    wait_for_rows(&mut panel, 2);

    let expected = vec![
        String::from("com.example.broken  (unknown)"),
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
    expand_appops(&mut panel);
    wait_for(&mut panel, |panel| {
        return status_contains(panel, "could not list applications");
    });

    assert!(entry_texts(&panel).is_empty());
    assert!(only_status(&panel).contains("could not list applications"));
    mock.assert_complete();
}

#[test]
fn changes_a_mode_and_verifies_it() {
    let mock = MockAdb::new();
    expect_load(
        &mock,
        SERIAL,
        &[("com.example.app", "RUN_ANY_IN_BACKGROUND: allow\n")],
    );
    expect_change(
        &mock,
        SERIAL,
        "com.example.app",
        "default",
        "RUN_ANY_IN_BACKGROUND: default\n",
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_appops(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Right));
    wait_for_commands(&mut panel, &mock);

    let expected = vec![String::from("com.example.app  (default)")];
    assert_eq!(entry_texts(&panel), expected);
    assert!(status_texts(&panel).is_empty());
    mock.assert_complete();
}

#[test]
fn moves_back_to_the_previous_mode() {
    let mock = MockAdb::new();
    expect_load(
        &mock,
        SERIAL,
        &[("com.example.app", "RUN_ANY_IN_BACKGROUND: default\n")],
    );
    expect_change(
        &mock,
        SERIAL,
        "com.example.app",
        "allow",
        "RUN_ANY_IN_BACKGROUND: allow\n",
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_appops(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Left));
    wait_for_commands(&mut panel, &mock);

    let expected = vec![String::from("com.example.app  (allow)")];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn wraps_from_the_restricted_mode_to_allow() {
    let mock = MockAdb::new();
    expect_load(
        &mock,
        SERIAL,
        &[("com.example.app", "RUN_ANY_IN_BACKGROUND: ignore\n")],
    );
    expect_change(
        &mock,
        SERIAL,
        "com.example.app",
        "allow",
        "RUN_ANY_IN_BACKGROUND: allow\n",
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_appops(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Right));
    wait_for_commands(&mut panel, &mock);

    let expected = vec![String::from("com.example.app  (allow)")];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn changes_an_unreadable_mode_to_default() {
    let mock = MockAdb::new();
    expect_load(&mock, SERIAL, &[("com.example.app", "Not an AppOps entry\n")]);
    expect_change(
        &mock,
        SERIAL,
        "com.example.app",
        "default",
        "RUN_ANY_IN_BACKGROUND: default\n",
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_appops(&mut panel);
    wait_for_rows(&mut panel, 1);
    assert_eq!(
        entry_texts(&panel),
        vec![String::from("com.example.app  (unknown)")]
    );

    focus_first_application(&mut panel);
    assert!(panel.handle_key(KeyCode::Right));
    wait_for_commands(&mut panel, &mock);

    let expected = vec![String::from("com.example.app  (default)")];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn ignores_changes_while_another_is_pending() {
    let mock = MockAdb::new();
    expect_load(
        &mock,
        SERIAL,
        &[("com.example.app", "RUN_ANY_IN_BACKGROUND: allow\n")],
    );
    expect_change(
        &mock,
        SERIAL,
        "com.example.app",
        "default",
        "RUN_ANY_IN_BACKGROUND: default\n",
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_appops(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Right));
    // The first change is still in flight, so this one is ignored.
    assert!(!panel.handle_key(KeyCode::Left));
    wait_for_commands(&mut panel, &mock);

    let expected = vec![String::from("com.example.app  (default)")];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn reverts_changes_that_the_device_did_not_apply() {
    let mock = MockAdb::new();
    expect_load(
        &mock,
        SERIAL,
        &[("com.example.app", "RUN_ANY_IN_BACKGROUND: allow\n")],
    );
    expect_change(
        &mock,
        SERIAL,
        "com.example.app",
        "default",
        "RUN_ANY_IN_BACKGROUND: allow\n",
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_appops(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Right));
    wait_for(&mut panel, |panel| {
        return status_contains(panel, "kept");
    });

    let expected = vec![String::from("com.example.app  (allow)")];
    assert_eq!(entry_texts(&panel), expected);
    assert!(only_status(&panel).contains("kept"));
    mock.assert_complete();
}

#[test]
fn reverts_changes_that_fail() {
    let mock = MockAdb::new();
    expect_load(
        &mock,
        SERIAL,
        &[("com.example.app", "RUN_ANY_IN_BACKGROUND: allow\n")],
    );
    mock.expect(
        &[
            "-s",
            SERIAL,
            "shell",
            "cmd",
            "appops",
            "set",
            "com.example.app",
            OPERATION,
            "default",
        ],
        MockResponse::failure("java.lang.SecurityException"),
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_appops(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Right));
    wait_for(&mut panel, |panel| {
        return status_contains(panel, "rejected");
    });

    let expected = vec![String::from("com.example.app  (allow)")];
    assert_eq!(entry_texts(&panel), expected);
    assert!(only_status(&panel).contains("rejected"));
    mock.assert_complete();
}

#[test]
fn reverts_changes_that_cannot_be_verified() {
    let mock = MockAdb::new();
    expect_load(
        &mock,
        SERIAL,
        &[("com.example.app", "RUN_ANY_IN_BACKGROUND: allow\n")],
    );
    expect_change(
        &mock,
        SERIAL,
        "com.example.app",
        "default",
        "RUN_ANY_IN_BACKGROUND: maybe\n",
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_appops(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Right));
    wait_for(&mut panel, |panel| {
        return status_contains(panel, "could not be read");
    });

    let expected = vec![String::from("com.example.app  (allow)")];
    assert_eq!(entry_texts(&panel), expected);
    assert!(only_status(&panel).contains("could not be read"));
    mock.assert_complete();
}

#[test]
fn loads_states_only_when_expanded() {
    let mock = MockAdb::new();
    expect_load(
        &mock,
        SERIAL,
        &[("com.example.app", "RUN_ANY_IN_BACKGROUND: allow\n")],
    );

    let mut panel = panel_for(&mock, ANDROID_13);

    // The package inventory is read as soon as the panel polls, but no mode is
    // read while the section stays collapsed.
    wait_for(&mut panel, |_| {
        return package_listings(&mock, SERIAL) == 1;
    });
    for _ in 0..20 {
        panel.poll();
    }
    assert_eq!(mock.calls().len(), 1);

    expand_appops(&mut panel);
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
    expect_load(
        &mock,
        SERIAL,
        &[("com.example.alpha", "RUN_ANY_IN_BACKGROUND: allow\n")],
    );
    // Expanding again reads the modes once more from the shared inventory, so
    // the package list is not read a second time.
    expect_states(
        &mock,
        SERIAL,
        &[("com.example.alpha", "RUN_ANY_IN_BACKGROUND: default\n")],
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_appops(&mut panel);
    wait_for_rows(&mut panel, 1);
    assert_eq!(package_listings(&mock, SERIAL), 1);

    assert!(panel.handle_key(KeyCode::Char(' ')));
    panel.poll();
    assert!(panel.handle_key(KeyCode::Char(' ')));
    // Expanding again rescans the modes, so wait for the refreshed row.
    wait_for(&mut panel, |panel| {
        let expected = vec![String::from("com.example.alpha  (default)")];
        return entry_texts(panel) == expected;
    });

    assert_eq!(package_listings(&mock, SERIAL), 1);
    mock.assert_complete();
}

#[test]
fn discards_a_pending_change_when_the_device_changes() {
    let mock = MockAdb::new();
    expect_load(
        &mock,
        SERIAL,
        &[("com.example.app", "RUN_ANY_IN_BACKGROUND: allow\n")],
    );
    expect_change(
        &mock,
        SERIAL,
        "com.example.app",
        "default",
        "RUN_ANY_IN_BACKGROUND: default\n",
    );
    expect_load(
        &mock,
        OTHER_SERIAL,
        &[("com.example.other", "RUN_ANY_IN_BACKGROUND: ignore\n")],
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_appops(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);
    assert!(panel.handle_key(KeyCode::Right));

    let target = ControlTarget::new(
        String::from(OTHER_SERIAL),
        String::from(ANDROID_13),
    );
    assert!(panel.set_target(Some(target)));

    wait_for(&mut panel, |panel| {
        let expected = vec![String::from("com.example.other  (ignore)")];
        return entry_texts(panel) == expected;
    });
    assert!(status_texts(&panel).is_empty());
    mock.assert_complete();
}

#[test]
fn ignores_results_from_a_previous_device() {
    let mock = MockAdb::new();
    // The inventory of the first device is read, but the device changes before
    // any of its modes is, so no mode is ever read for it.
    expect_listing(
        &mock,
        SERIAL,
        &[("com.example.first", "RUN_ANY_IN_BACKGROUND: allow\n")],
    );
    expect_load(
        &mock,
        OTHER_SERIAL,
        &[("com.example.second", "RUN_ANY_IN_BACKGROUND: ignore\n")],
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_appops(&mut panel);
    // The inventory for the first device is requested, then the device changes.
    panel.poll();

    let target = ControlTarget::new(
        String::from(OTHER_SERIAL),
        String::from(ANDROID_13),
    );
    assert!(panel.set_target(Some(target)));

    wait_for(&mut panel, |panel| {
        let expected = vec![String::from("com.example.second  (ignore)")];
        return entry_texts(panel) == expected;
    });
    assert_eq!(package_listings(&mock, SERIAL), 1);
    assert_eq!(package_listings(&mock, OTHER_SERIAL), 1);
    // Both listings and the one mode read of the new device.
    assert_eq!(mock.calls().len(), 3);
    mock.assert_complete();
}
