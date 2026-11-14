//! Tests for the Doze Whitelist section and the states it manages.

use crossterm::event::KeyCode;

use bloatdroid::control::ControlPanel;
use bloatdroid::control::ControlTarget;
use bloatdroid::control::doze::WhitelistState;

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

/// Expects the package listing and the complete whitelist of a device.
fn expect_load(
    mock: &MockAdb,
    serial: &str,
    packages: &[&str],
    whitelist: &str,
) {
    let mut listing = String::new();
    for package in packages {
        listing.push_str("package:");
        listing.push_str(package);
        listing.push('\n');
    }
    mock.expect(
        &["-s", serial, "shell", "pm", "list", "packages"],
        MockResponse::output(listing),
    );
    mock.expect(
        &["-s", serial, "shell", "cmd", "deviceidle", "whitelist"],
        MockResponse::output(whitelist),
    );
}

/// Expects a whitelist command and the complete whitelist read after it.
fn expect_change(
    mock: &MockAdb,
    serial: &str,
    argument: &str,
    whitelist: &str,
) {
    mock.expect(
        &["-s", serial, "shell", "cmd", "deviceidle", "whitelist", argument],
        MockResponse::output(""),
    );
    mock.expect(
        &["-s", serial, "shell", "cmd", "deviceidle", "whitelist"],
        MockResponse::output(whitelist),
    );
}

/// Expands the Doze Whitelist section, the last one in the panel.
fn expand_doze(panel: &mut ControlPanel) {
    assert!(panel.handle_key(KeyCode::Down));
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

/// Returns how often the whitelist of a device was read.
fn whitelist_reads(mock: &MockAdb, serial: &str) -> usize {
    let wanted = vec![
        String::from("-s"),
        String::from(serial),
        String::from("shell"),
        String::from("cmd"),
        String::from("deviceidle"),
        String::from("whitelist"),
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
fn maps_states_to_and_from_their_names() {
    assert_eq!(WhitelistState::User.name(), "user");
    assert_eq!(WhitelistState::System.name(), "system");
    assert_eq!(WhitelistState::NotWhitelisted.name(), "none");

    assert_eq!(WhitelistState::from_name("user"), Some(WhitelistState::User));
    assert_eq!(
        WhitelistState::from_name("system"),
        Some(WhitelistState::System)
    );
    assert_eq!(
        WhitelistState::from_name("none"),
        Some(WhitelistState::NotWhitelisted)
    );
    assert_eq!(WhitelistState::from_name("unknown"), None);
}

#[test]
fn toggles_only_modifiable_states() {
    assert_eq!(
        WhitelistState::NotWhitelisted.toggled(),
        Some(WhitelistState::User)
    );
    assert_eq!(
        WhitelistState::User.toggled(),
        Some(WhitelistState::NotWhitelisted)
    );
    assert_eq!(WhitelistState::System.toggled(), None);
}

#[test]
fn classifies_user_system_and_excidle_entries() {
    let mock = MockAdb::new();
    expect_load(
        &mock,
        SERIAL,
        &[
            "com.example.alpha",
            "com.example.bravo",
            "com.example.charlie",
            "com.example.delta",
        ],
        "user,com.example.alpha,10123\nsystem,com.example.bravo,1000\nsystem-excidle,com.example.charlie,1001\n",
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_doze(&mut panel);
    wait_for_rows(&mut panel, 4);

    let expected = vec![
        String::from("com.example.alpha  (user)"),
        String::from("com.example.bravo  (system)"),
        String::from("com.example.charlie  (system)"),
        String::from("com.example.delta  (none)"),
    ];
    assert_eq!(entry_texts(&panel), expected);
    assert!(status_texts(&panel).is_empty());
    mock.assert_complete();
}

#[test]
fn gives_system_entries_precedence_over_user_entries() {
    let mock = MockAdb::new();
    expect_load(
        &mock,
        SERIAL,
        &["com.example.app"],
        "user,com.example.app,10123\nsystem-excidle,com.example.app,1000\n",
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_doze(&mut panel);
    wait_for_rows(&mut panel, 1);

    let expected = vec![String::from("com.example.app  (system)")];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn ignores_temporary_whitelist_entries() {
    let mock = MockAdb::new();
    expect_load(
        &mock,
        SERIAL,
        &["com.example.app"],
        "temp,com.example.app,10123\n",
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_doze(&mut panel);
    wait_for_rows(&mut panel, 1);

    let expected = vec![String::from("com.example.app  (none)")];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn treats_an_empty_whitelist_as_no_entries() {
    let mock = MockAdb::new();
    expect_load(&mock, SERIAL, &["com.example.app"], "");

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_doze(&mut panel);
    wait_for_rows(&mut panel, 1);

    let expected = vec![String::from("com.example.app  (none)")];
    assert_eq!(entry_texts(&panel), expected);
    assert!(status_texts(&panel).is_empty());
    mock.assert_complete();
}

#[test]
fn reports_malformed_whitelist_output() {
    let mock = MockAdb::new();
    expect_load(
        &mock,
        SERIAL,
        &["com.example.app"],
        "this is not a whitelist entry\n",
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_doze(&mut panel);
    wait_for(&mut panel, |panel| {
        return status_contains(panel, "could not read the whitelist");
    });

    assert!(entry_texts(&panel).is_empty());
    assert!(only_status(&panel).contains("could not read the whitelist"));
    mock.assert_complete();
}

#[test]
fn reports_unknown_whitelist_categories() {
    let mock = MockAdb::new();
    expect_load(
        &mock,
        SERIAL,
        &["com.example.app"],
        "mystery,com.example.app,10123\n",
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_doze(&mut panel);
    wait_for(&mut panel, |panel| {
        return status_contains(panel, "could not read the whitelist");
    });

    assert!(entry_texts(&panel).is_empty());
    mock.assert_complete();
}

#[test]
fn loads_applications_sorted_by_package_name() {
    let mock = MockAdb::new();
    expect_load(
        &mock,
        SERIAL,
        &["com.example.zulu", "com.example.alpha", "com.example.mike"],
        "",
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_doze(&mut panel);
    wait_for_rows(&mut panel, 3);

    let expected = vec![
        String::from("com.example.alpha  (none)"),
        String::from("com.example.mike  (none)"),
        String::from("com.example.zulu  (none)"),
    ];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn reports_package_listing_failures() {
    let mock = MockAdb::new();
    mock.expect(
        &["-s", SERIAL, "shell", "pm", "list", "packages"],
        MockResponse::failure("error: device offline"),
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_doze(&mut panel);
    wait_for(&mut panel, |panel| {
        return status_contains(panel, "could not list applications");
    });

    assert!(entry_texts(&panel).is_empty());
    assert!(only_status(&panel).contains("could not list applications"));
    mock.assert_complete();
}

#[test]
fn reports_whitelist_failures() {
    let mock = MockAdb::new();
    mock.expect(
        &["-s", SERIAL, "shell", "pm", "list", "packages"],
        MockResponse::output("package:com.example.app\n"),
    );
    mock.expect(
        &["-s", SERIAL, "shell", "cmd", "deviceidle", "whitelist"],
        MockResponse::failure("Unknown command: deviceidle"),
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_doze(&mut panel);
    wait_for(&mut panel, |panel| {
        return status_contains(panel, "could not read the whitelist");
    });

    assert!(entry_texts(&panel).is_empty());
    assert!(only_status(&panel).contains("could not read the whitelist"));
    mock.assert_complete();
}

#[test]
fn adds_a_package_to_the_user_whitelist() {
    let mock = MockAdb::new();
    expect_load(&mock, SERIAL, &["com.example.app"], "");
    expect_change(
        &mock,
        SERIAL,
        "+com.example.app",
        "user,com.example.app,10123\n",
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_doze(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Right));
    wait_for_commands(&mut panel, &mock);

    let expected = vec![String::from("com.example.app  (user)")];
    assert_eq!(entry_texts(&panel), expected);
    assert!(status_texts(&panel).is_empty());
    mock.assert_complete();
}

#[test]
fn removes_a_package_from_the_user_whitelist() {
    let mock = MockAdb::new();
    expect_load(
        &mock,
        SERIAL,
        &["com.example.app"],
        "user,com.example.app,10123\n",
    );
    expect_change(&mock, SERIAL, "-com.example.app", "");

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_doze(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Left));
    wait_for_commands(&mut panel, &mock);

    let expected = vec![String::from("com.example.app  (none)")];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn does_not_change_system_whitelist_entries() {
    let mock = MockAdb::new();
    expect_load(
        &mock,
        SERIAL,
        &["com.example.app"],
        "system,com.example.app,1000\n",
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_doze(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(!panel.handle_key(KeyCode::Right));
    assert!(!panel.handle_key(KeyCode::Left));

    let expected = vec![String::from("com.example.app  (system)")];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn ignores_changes_while_another_is_pending() {
    let mock = MockAdb::new();
    expect_load(&mock, SERIAL, &["com.example.app"], "");
    expect_change(
        &mock,
        SERIAL,
        "+com.example.app",
        "user,com.example.app,10123\n",
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_doze(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Right));
    // The first change is still in flight, so this one is ignored.
    assert!(!panel.handle_key(KeyCode::Left));
    wait_for_commands(&mut panel, &mock);

    let expected = vec![String::from("com.example.app  (user)")];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn reverts_changes_that_the_device_did_not_apply() {
    let mock = MockAdb::new();
    expect_load(&mock, SERIAL, &["com.example.app"], "");
    expect_change(&mock, SERIAL, "+com.example.app", "");

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_doze(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Right));
    wait_for(&mut panel, |panel| {
        return status_contains(panel, "kept");
    });

    let expected = vec![String::from("com.example.app  (none)")];
    assert_eq!(entry_texts(&panel), expected);
    assert!(only_status(&panel).contains("kept"));
    mock.assert_complete();
}

#[test]
fn reverts_changes_that_fail() {
    let mock = MockAdb::new();
    expect_load(&mock, SERIAL, &["com.example.app"], "");
    mock.expect(
        &[
            "-s",
            SERIAL,
            "shell",
            "cmd",
            "deviceidle",
            "whitelist",
            "+com.example.app",
        ],
        MockResponse::failure("java.lang.SecurityException"),
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_doze(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Right));
    wait_for(&mut panel, |panel| {
        return status_contains(panel, "rejected");
    });

    let expected = vec![String::from("com.example.app  (none)")];
    assert_eq!(entry_texts(&panel), expected);
    assert!(only_status(&panel).contains("rejected"));
    mock.assert_complete();
}

#[test]
fn reverts_changes_that_cannot_be_verified() {
    let mock = MockAdb::new();
    expect_load(&mock, SERIAL, &["com.example.app"], "");
    expect_change(&mock, SERIAL, "+com.example.app", "not a whitelist entry\n");

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_doze(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Right));
    wait_for(&mut panel, |panel| {
        return status_contains(panel, "could not verify");
    });

    let expected = vec![String::from("com.example.app  (none)")];
    assert_eq!(entry_texts(&panel), expected);
    assert!(only_status(&panel).contains("could not verify"));
    mock.assert_complete();
}

#[test]
fn loads_only_when_expanded() {
    let mock = MockAdb::new();
    expect_load(&mock, SERIAL, &["com.example.app"], "");

    let mut panel = panel_for(&mock, ANDROID_13);
    panel.poll();
    assert!(mock.calls().is_empty());

    expand_doze(&mut panel);
    wait_for_rows(&mut panel, 1);

    for _ in 0..20 {
        panel.poll();
    }
    assert_eq!(whitelist_reads(&mock, SERIAL), 1);
    mock.assert_complete();
}

#[test]
fn reloads_after_collapsing_and_expanding() {
    let mock = MockAdb::new();
    expect_load(&mock, SERIAL, &["com.example.alpha"], "");
    expect_load(&mock, SERIAL, &["com.example.alpha", "com.example.bravo"], "");

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_doze(&mut panel);
    wait_for_rows(&mut panel, 1);
    assert_eq!(whitelist_reads(&mock, SERIAL), 1);

    assert!(panel.handle_key(KeyCode::Char(' ')));
    panel.poll();
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for_rows(&mut panel, 2);

    assert_eq!(whitelist_reads(&mock, SERIAL), 2);
    mock.assert_complete();
}

#[test]
fn discards_a_pending_change_when_the_device_changes() {
    let mock = MockAdb::new();
    expect_load(&mock, SERIAL, &["com.example.app"], "");
    expect_change(
        &mock,
        SERIAL,
        "+com.example.app",
        "user,com.example.app,10123\n",
    );
    expect_load(
        &mock,
        OTHER_SERIAL,
        &["com.example.other"],
        "system,com.example.other,1000\n",
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_doze(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);
    assert!(panel.handle_key(KeyCode::Right));

    let target = ControlTarget::new(
        String::from(OTHER_SERIAL),
        String::from(ANDROID_13),
    );
    assert!(panel.set_target(Some(target)));

    wait_for(&mut panel, |panel| {
        let expected = vec![String::from("com.example.other  (system)")];
        return entry_texts(panel) == expected;
    });
    assert!(status_texts(&panel).is_empty());
    mock.assert_complete();
}

#[test]
fn ignores_results_from_a_previous_device() {
    let mock = MockAdb::new();
    expect_load(&mock, SERIAL, &["com.example.first"], "");
    expect_load(&mock, OTHER_SERIAL, &["com.example.second"], "");

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_doze(&mut panel);
    // The load for the first device is requested, then the device changes.
    panel.poll();

    let target = ControlTarget::new(
        String::from(OTHER_SERIAL),
        String::from(ANDROID_13),
    );
    assert!(panel.set_target(Some(target)));

    wait_for(&mut panel, |panel| {
        let expected = vec![String::from("com.example.second  (none)")];
        return entry_texts(panel) == expected;
    });
    assert_eq!(whitelist_reads(&mock, SERIAL), 1);
    assert_eq!(whitelist_reads(&mock, OTHER_SERIAL), 1);
    mock.assert_complete();
}
