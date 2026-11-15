//! Tests for the Standby Buckets section.

use crossterm::event::KeyCode;

use bloatdroid::control::ControlPanel;
use bloatdroid::control::buckets::StandbyBucket;
use bloatdroid::log::LogLevel;

use crate::common::MockAdb;
use crate::common::MockResponse;

use super::harness::ANDROID_10;
use super::harness::ANDROID_13;
use super::harness::SERIAL;
use super::harness::entry_texts;
use super::harness::focused_text;
use super::harness::header_expansion;
use super::harness::log_has;
use super::harness::panel_for;
use super::harness::status_texts;
use super::harness::wait_for;
use super::harness::wait_for_commands;

/// Expects the package listing of the given applications.
fn expect_listing(mock: &MockAdb, applications: &[(&str, &str)]) {
    let mut listing = String::new();
    for &(package, _bucket) in applications {
        listing.push_str("package:");
        listing.push_str(package);
        listing.push('\n');
    }
    mock.expect(
        &["-s", SERIAL, "shell", "pm", "list", "packages", "-3"],
        MockResponse::output(listing),
    );
}

/// Expects the bucket read of every given application.
fn expect_buckets(mock: &MockAdb, applications: &[(&str, &str)]) {
    for &(package, bucket) in applications {
        mock.expect(
            &["-s", SERIAL, "shell", "am", "get-standby-bucket", package],
            MockResponse::output(bucket),
        );
    }
}

/// Expects the package listing and the bucket of every application.
fn expect_load(mock: &MockAdb, applications: &[(&str, &str)]) {
    expect_listing(mock, applications);
    expect_buckets(mock, applications);
}

/// Expects the write of one bucket and the read that verifies it.
fn expect_bucket_write(mock: &MockAdb, package: &str, bucket: &str) {
    mock.expect(
        &["-s", SERIAL, "shell", "am", "set-standby-bucket", package, bucket],
        MockResponse::output(""),
    );
    mock.expect(
        &["-s", SERIAL, "shell", "am", "get-standby-bucket", package],
        MockResponse::output(bucket),
    );
}

/// Expands the Standby Buckets section, which holds the cursor at the start.
fn expand_buckets(panel: &mut ControlPanel) {
    assert!(panel.handle_key(KeyCode::Char(' ')));
}

/// Polls until the expanded section shows the expected number of rows.
fn wait_for_rows(panel: &mut ControlPanel, count: usize) {
    wait_for(panel, |panel| {
        return entry_texts(panel).len() == count;
    });
}

/// Waits until the device confirmed the change to the given bucket.
///
/// The mock records the commands before the section applies their result, so
/// the confirmation is what makes the wait deterministic.
fn wait_for_bucket(panel: &mut ControlPanel, mock: &MockAdb, bucket: &str) {
    let message = format!(
        "Standby bucket successfully changed to {}.",
        bucket.to_ascii_uppercase()
    );
    wait_for_commands(panel, mock, |panel| {
        return log_has(panel, LogLevel::Success, message.as_str());
    });
}

/// Moves the cursor from the section header onto the first application.
fn focus_first_application(panel: &mut ControlPanel) {
    assert!(panel.handle_key(KeyCode::Down));
}

/// Returns the text of the row that holds the cursor.
fn focused_row(panel: &ControlPanel) -> String {
    match focused_text(panel) {
        Some(text) => return text,
        None => panic!("no row holds the cursor"),
    }
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

#[test]
fn loads_applications_sorted_by_package_name() {
    let mock = MockAdb::new();
    expect_load(
        &mock,
        &[
            ("com.example.zulu", "active"),
            ("com.example.alpha", "rare"),
            ("com.example.mike", "working_set"),
        ],
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_buckets(&mut panel);
    wait_for_rows(&mut panel, 3);

    let expected = vec![
        String::from("com.example.alpha  rare"),
        String::from("com.example.mike  working_set"),
        String::from("com.example.zulu  active"),
    ];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn changes_a_bucket_and_verifies_it() {
    let mock = MockAdb::new();
    expect_load(&mock, &[("com.example.app", "working_set")]);
    expect_bucket_write(&mock, "com.example.app", "frequent");

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_buckets(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Right));
    wait_for_bucket(&mut panel, &mock, "frequent");

    let expected = vec![String::from("com.example.app  frequent")];
    assert_eq!(entry_texts(&panel), expected);
    assert!(status_texts(&panel).is_empty());
    mock.assert_complete();
}

#[test]
fn moves_back_towards_the_more_active_buckets() {
    let mock = MockAdb::new();
    expect_load(&mock, &[("com.example.app", "working_set")]);
    expect_bucket_write(&mock, "com.example.app", "active");

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_buckets(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Left));
    wait_for_bucket(&mut panel, &mock, "active");

    let expected = vec![String::from("com.example.app  active")];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn stops_at_the_most_active_bucket() {
    let mock = MockAdb::new();
    expect_load(&mock, &[("com.example.app", "active")]);

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_buckets(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(!panel.handle_key(KeyCode::Left));

    let expected = vec![String::from("com.example.app  active")];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn offers_restricted_from_android_11() {
    let mock = MockAdb::new();
    expect_load(&mock, &[("com.example.app", "rare")]);
    expect_bucket_write(&mock, "com.example.app", "restricted");

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_buckets(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Right));
    wait_for_bucket(&mut panel, &mock, "restricted");

    let expected = vec![String::from("com.example.app  restricted")];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn does_not_offer_restricted_before_android_11() {
    let mock = MockAdb::new();
    expect_load(&mock, &[("com.example.app", "rare")]);

    let mut panel = panel_for(&mock, ANDROID_10);
    expand_buckets(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    // Android 10 has no restricted bucket, so the value stays and no command
    // is sent.
    assert!(!panel.handle_key(KeyCode::Right));

    let expected = vec![String::from("com.example.app  rare")];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn reverts_changes_that_the_device_did_not_apply() {
    let mock = MockAdb::new();
    expect_load(&mock, &[("com.example.app", "working_set")]);
    mock.expect(
        &[
            "-s",
            SERIAL,
            "shell",
            "am",
            "set-standby-bucket",
            "com.example.app",
            "frequent",
        ],
        MockResponse::output(""),
    );
    // The device keeps the old value when the bucket is read back.
    mock.expect(
        &["-s", SERIAL, "shell", "am", "get-standby-bucket", "com.example.app"],
        MockResponse::output("working_set"),
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_buckets(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Right));
    wait_for(&mut panel, |panel| {
        return status_contains(panel, "kept");
    });

    let expected = vec![String::from("com.example.app  working_set")];
    assert_eq!(entry_texts(&panel), expected);
    assert!(only_status(&panel).contains("kept"));
    mock.assert_complete();
}

#[test]
fn reverts_changes_that_fail() {
    let mock = MockAdb::new();
    expect_load(&mock, &[("com.example.app", "working_set")]);
    mock.expect(
        &[
            "-s",
            SERIAL,
            "shell",
            "am",
            "set-standby-bucket",
            "com.example.app",
            "frequent",
        ],
        MockResponse::failure("java.lang.SecurityException"),
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_buckets(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Right));
    wait_for(&mut panel, |panel| {
        return status_contains(panel, "rejected");
    });

    let expected = vec![String::from("com.example.app  working_set")];
    assert_eq!(entry_texts(&panel), expected);
    assert!(only_status(&panel).contains("rejected"));
    mock.assert_complete();
}

#[test]
fn reports_unreadable_buckets_and_leaves_them_alone() {
    let mock = MockAdb::new();
    expect_load(&mock, &[("com.example.app", "")]);

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_buckets(&mut panel);
    wait_for_rows(&mut panel, 1);

    let expected = vec![String::from("com.example.app  unknown")];
    assert_eq!(entry_texts(&panel), expected);
    assert!(only_status(&panel).contains("could not be read"));

    // An unknown bucket has no neighbour to move to, and it is never rounded
    // to a valid one.
    focus_first_application(&mut panel);
    assert!(!panel.handle_key(KeyCode::Right));
    assert!(!panel.handle_key(KeyCode::Left));

    assert_eq!(entry_texts(&panel), expected);
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
    expand_buckets(&mut panel);
    wait_for(&mut panel, |panel| {
        return status_contains(panel, "could not list applications");
    });

    assert!(only_status(&panel).contains("could not list applications"));
    mock.assert_complete();
}

#[test]
fn keeps_the_selected_application_after_an_inventory_refresh() {
    let mock = MockAdb::new();
    expect_load(
        &mock,
        &[
            ("com.example.alpha", "active"),
            ("com.example.bravo", "active"),
            ("com.example.charlie", "active"),
        ],
    );
    // The refreshed inventory adds an application in front of the selected
    // one, and every bucket is read again for it.
    expect_load(
        &mock,
        &[
            ("com.example.alfa", "active"),
            ("com.example.alpha", "active"),
            ("com.example.bravo", "active"),
            ("com.example.charlie", "active"),
        ],
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_buckets(&mut panel);
    wait_for_rows(&mut panel, 3);

    assert!(panel.handle_key(KeyCode::Down));
    assert!(panel.handle_key(KeyCode::Down));
    assert!(panel.handle_key(KeyCode::Down));
    assert_eq!(focused_row(&panel), "com.example.charlie  active");

    // The refresh happens while an application stays selected.
    assert!(panel.refresh_inventory());
    wait_for_rows(&mut panel, 4);

    assert_eq!(focused_row(&panel), "com.example.charlie  active");
    mock.assert_complete();
}

#[test]
fn parses_textual_and_numeric_buckets() {
    let expected = [
        ("exempted", StandbyBucket::Exempted, 5),
        ("active", StandbyBucket::Active, 10),
        ("working_set", StandbyBucket::WorkingSet, 20),
        ("frequent", StandbyBucket::Frequent, 30),
        ("rare", StandbyBucket::Rare, 40),
        ("restricted", StandbyBucket::Restricted, 45),
    ];

    for (name, bucket, id) in expected {
        let number = id.to_string();

        assert_eq!(StandbyBucket::from_name(name), Some(bucket));
        assert_eq!(StandbyBucket::from_id(id), Some(bucket));
        assert_eq!(StandbyBucket::parse(name), Some(bucket));
        assert_eq!(StandbyBucket::parse(number.as_str()), Some(bucket));
        assert_eq!(bucket.name(), name);
        assert_eq!(bucket.id(), id);
    }
}

#[test]
fn keeps_values_that_are_not_buckets_unknown() {
    let values = ["", "unknown", "ALLOW", "10x", "-10", "50", "0", "10 "];
    for value in values {
        assert_eq!(StandbyBucket::parse(value), None, "parsed {value:?}");
    }

    assert_eq!(StandbyBucket::from_id(15), None);
    assert_eq!(StandbyBucket::from_name("exempt"), None);
}

#[test]
fn treats_the_exempted_bucket_as_read_only() {
    assert!(!StandbyBucket::Exempted.is_modifiable());
    assert!(StandbyBucket::Active.is_modifiable());
    assert!(StandbyBucket::Restricted.is_modifiable());

    // The buckets a change can target never include the read-only one.
    for bucket in StandbyBucket::MODIFIABLE {
        assert_ne!(bucket, StandbyBucket::Exempted);
    }
}

#[test]
fn shows_numeric_buckets_as_their_names() {
    let mock = MockAdb::new();
    expect_load(
        &mock,
        &[
            ("com.example.zulu", "45"),
            ("com.example.alpha", "10"),
            ("com.example.mike", "20"),
        ],
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_buckets(&mut panel);
    wait_for_rows(&mut panel, 3);

    let expected = vec![
        String::from("com.example.alpha  active"),
        String::from("com.example.mike  working_set"),
        String::from("com.example.zulu  restricted"),
    ];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn shows_the_exempted_bucket_and_leaves_it_alone() {
    let mock = MockAdb::new();
    expect_load(&mock, &[("com.example.app", "5")]);

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_buckets(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    let expected = vec![String::from("com.example.app  exempted")];
    assert_eq!(entry_texts(&panel), expected);

    // The bucket is read-only, so neither arrow asks the device for a change.
    assert!(!panel.handle_key(KeyCode::Right));
    assert!(!panel.handle_key(KeyCode::Left));

    assert_eq!(entry_texts(&panel), expected);
    assert!(status_texts(&panel).is_empty());
    mock.assert_complete();
}

#[test]
fn never_changes_a_bucket_it_does_not_understand() {
    let mock = MockAdb::new();
    expect_load(&mock, &[("com.example.app", "77")]);

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_buckets(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    let expected = vec![String::from("com.example.app  unknown")];
    assert_eq!(entry_texts(&panel), expected);

    assert!(!panel.handle_key(KeyCode::Right));
    assert!(!panel.handle_key(KeyCode::Left));

    assert_eq!(entry_texts(&panel), expected);
    assert!(only_status(&panel).contains("1 applications could not be read"));
    mock.assert_complete();
}

#[test]
fn cycles_from_a_numeric_bucket() {
    let mock = MockAdb::new();
    expect_load(&mock, &[("com.example.app", "20")]);
    mock.expect(
        &[
            "-s",
            SERIAL,
            "shell",
            "am",
            "set-standby-bucket",
            "com.example.app",
            "frequent",
        ],
        MockResponse::output(""),
    );
    // The device answers with the new bucket as a number.
    mock.expect(
        &["-s", SERIAL, "shell", "am", "get-standby-bucket", "com.example.app"],
        MockResponse::output("30"),
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_buckets(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Right));
    wait_for_bucket(&mut panel, &mock, "frequent");

    let expected = vec![String::from("com.example.app  frequent")];
    assert_eq!(entry_texts(&panel), expected);
    assert!(status_texts(&panel).is_empty());
    mock.assert_complete();
}

#[test]
fn reverts_when_the_device_reports_another_numeric_bucket() {
    let mock = MockAdb::new();
    expect_load(&mock, &[("com.example.app", "20")]);
    mock.expect(
        &[
            "-s",
            SERIAL,
            "shell",
            "am",
            "set-standby-bucket",
            "com.example.app",
            "frequent",
        ],
        MockResponse::output(""),
    );
    // The device kept the active bucket instead of the requested one.
    mock.expect(
        &["-s", SERIAL, "shell", "am", "get-standby-bucket", "com.example.app"],
        MockResponse::output("10"),
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_buckets(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Right));
    wait_for(&mut panel, |panel| {
        return status_contains(panel, "kept");
    });

    let expected = vec![String::from("com.example.app  working_set")];
    assert_eq!(entry_texts(&panel), expected);
    assert!(only_status(&panel).contains("kept `active`"));
    mock.assert_complete();
}

#[test]
fn reverts_when_the_device_reports_no_bucket() {
    let mock = MockAdb::new();
    expect_load(&mock, &[("com.example.app", "10")]);
    mock.expect(
        &[
            "-s",
            SERIAL,
            "shell",
            "am",
            "set-standby-bucket",
            "com.example.app",
            "working_set",
        ],
        MockResponse::output(""),
    );
    mock.expect(
        &["-s", SERIAL, "shell", "am", "get-standby-bucket", "com.example.app"],
        MockResponse::output(""),
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_buckets(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);

    assert!(panel.handle_key(KeyCode::Right));
    wait_for(&mut panel, |panel| {
        return status_contains(panel, "could not be read");
    });

    let expected = vec![String::from("com.example.app  active")];
    assert_eq!(entry_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn collapsing_returns_the_cursor_to_the_header() {
    let mock = MockAdb::new();
    expect_load(&mock, &[("com.example.app", "active")]);

    let mut panel = panel_for(&mock, ANDROID_13);
    expand_buckets(&mut panel);
    wait_for_rows(&mut panel, 1);
    focus_first_application(&mut panel);
    assert_eq!(focused_row(&panel), "com.example.app  active");

    assert!(panel.handle_key(KeyCode::Char(' ')));

    assert_eq!(focused_text(&panel), Some(String::from("Standby Buckets")));
    assert_eq!(header_expansion(&panel), vec![false, false, false, false]);
    mock.assert_complete();
}
