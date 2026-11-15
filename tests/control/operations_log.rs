//! Tests for what the control operations report to the Operations Log.
//!
//! Every test drives the panel through the mock ADB runner, so nothing here
//! needs a device.

use crossterm::event::KeyCode;

use bloatdroid::log::LogLevel;

use crate::common::MockAdb;
use crate::common::MockResponse;

use super::harness::ANDROID_13;
use super::harness::SERIAL;
use super::harness::entry_texts;
use super::harness::log_has;
use super::harness::panel_for;
use super::harness::set_filter;
use super::harness::wait_for;
use super::harness::wait_for_commands;

/// Expects the package listing of one application.
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

/// Polls until the expanded section shows the expected number of rows.
fn wait_for_rows(panel: &mut bloatdroid::control::ControlPanel, count: usize) {
    wait_for(panel, |panel| {
        return entry_texts(panel).len() == count;
    });
}

#[test]
fn logs_the_initial_application_inventory() {
    let mock = MockAdb::new();
    expect_listing(&mock, &["com.example.alpha", "com.example.bravo"]);

    let mut panel = panel_for(&mock, ANDROID_13);
    wait_for(&mut panel, |panel| {
        return log_has(panel, LogLevel::Success, "Found 2 applications.");
    });

    assert!(log_has(
        &panel,
        LogLevel::Info,
        "Loading the application inventory."
    ));
    mock.assert_complete();
}

#[test]
fn logs_a_verified_standby_bucket_change() {
    let mock = MockAdb::new();
    expect_listing(&mock, &["com.example.app"]);
    mock.expect(
        &["-s", SERIAL, "shell", "am", "get-standby-bucket", "com.example.app"],
        MockResponse::output("working_set"),
    );
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
    mock.expect(
        &["-s", SERIAL, "shell", "am", "get-standby-bucket", "com.example.app"],
        MockResponse::output("frequent"),
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for_rows(&mut panel, 1);
    assert!(panel.handle_key(KeyCode::Down));
    assert!(panel.handle_key(KeyCode::Right));
    wait_for_commands(&mut panel, &mock);

    assert!(log_has(
        &panel,
        LogLevel::Info,
        "Setting com.example.app to FREQUENT."
    ));
    assert!(log_has(
        &panel,
        LogLevel::Success,
        "Standby bucket successfully changed to FREQUENT."
    ));
    mock.assert_complete();
}

#[test]
fn logs_a_rejected_standby_bucket_change() {
    let mock = MockAdb::new();
    expect_listing(&mock, &["com.example.app"]);
    mock.expect(
        &["-s", SERIAL, "shell", "am", "get-standby-bucket", "com.example.app"],
        MockResponse::output("working_set"),
    );
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
    // The device keeps the previous bucket, so the change is not verified.
    mock.expect(
        &["-s", SERIAL, "shell", "am", "get-standby-bucket", "com.example.app"],
        MockResponse::output("working_set"),
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for_rows(&mut panel, 1);
    assert!(panel.handle_key(KeyCode::Down));
    assert!(panel.handle_key(KeyCode::Right));
    wait_for_commands(&mut panel, &mock);

    assert!(log_has(
        &panel,
        LogLevel::Error,
        "Failed to change the standby bucket: the device kept `working_set`."
    ));
    mock.assert_complete();
}

#[test]
fn logs_a_rejected_appops_change() {
    let mock = MockAdb::new();
    expect_listing(&mock, &["com.example.app"]);
    mock.expect(
        &[
            "-s",
            SERIAL,
            "shell",
            "cmd",
            "appops",
            "get",
            "com.example.app",
            "RUN_ANY_IN_BACKGROUND",
        ],
        MockResponse::output("RUN_ANY_IN_BACKGROUND: allow\n"),
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
            "RUN_ANY_IN_BACKGROUND",
            "default",
        ],
        MockResponse::output(""),
    );
    mock.expect(
        &[
            "-s",
            SERIAL,
            "shell",
            "cmd",
            "appops",
            "get",
            "com.example.app",
            "RUN_ANY_IN_BACKGROUND",
        ],
        MockResponse::output("RUN_ANY_IN_BACKGROUND: allow\n"),
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    assert!(panel.handle_key(KeyCode::Down));
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for_rows(&mut panel, 1);
    assert!(panel.handle_key(KeyCode::Down));
    assert!(panel.handle_key(KeyCode::Right));
    wait_for_commands(&mut panel, &mock);

    assert!(log_has(
        &panel,
        LogLevel::Info,
        "Setting com.example.app to DEFAULT."
    ));
    assert!(log_has(
        &panel,
        LogLevel::Error,
        "Failed to change the AppOps mode: the device kept `allow`."
    ));
    mock.assert_complete();
}

#[test]
fn logs_a_rejected_inactive_change() {
    let mock = MockAdb::new();
    expect_listing(&mock, &["com.example.app"]);
    mock.expect(
        &["-s", SERIAL, "shell", "am", "get-inactive", "com.example.app"],
        MockResponse::output("Idle=false\n"),
    );
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
        MockResponse::output(""),
    );
    mock.expect(
        &["-s", SERIAL, "shell", "am", "get-inactive", "com.example.app"],
        MockResponse::output("Idle=false\n"),
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    assert!(panel.handle_key(KeyCode::Down));
    assert!(panel.handle_key(KeyCode::Down));
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for_rows(&mut panel, 1);
    assert!(panel.handle_key(KeyCode::Down));
    assert!(panel.handle_key(KeyCode::Right));
    wait_for_commands(&mut panel, &mock);

    assert!(log_has(
        &panel,
        LogLevel::Info,
        "Setting com.example.app to INACTIVE."
    ));
    assert!(log_has(
        &panel,
        LogLevel::Error,
        "Failed to change the App Inactive state: the device kept `active`."
    ));
    mock.assert_complete();
}

#[test]
fn logs_a_verified_doze_whitelist_change() {
    let mock = MockAdb::new();
    expect_listing(&mock, &["com.example.app"]);
    mock.expect(
        &["-s", SERIAL, "shell", "cmd", "deviceidle", "whitelist"],
        MockResponse::output(""),
    );
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
        MockResponse::output(""),
    );
    mock.expect(
        &["-s", SERIAL, "shell", "cmd", "deviceidle", "whitelist"],
        MockResponse::output("user,com.example.app,10123\n"),
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    for _ in 0..3 {
        assert!(panel.handle_key(KeyCode::Down));
    }
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for_rows(&mut panel, 1);
    assert!(panel.handle_key(KeyCode::Down));
    assert!(panel.handle_key(KeyCode::Right));
    wait_for_commands(&mut panel, &mock);

    assert!(log_has(
        &panel,
        LogLevel::Info,
        "Adding com.example.app to the Doze whitelist."
    ));
    assert!(log_has(
        &panel,
        LogLevel::Success,
        "com.example.app was added to the Doze whitelist."
    ));
    mock.assert_complete();
}

#[test]
fn logs_the_inventory_of_a_filter_change() {
    let mock = MockAdb::new();
    expect_listing(&mock, &["com.example.alpha"]);
    mock.expect(
        &["-s", SERIAL, "shell", "pm", "list", "packages"],
        MockResponse::output(
            "package:com.android.systemui\npackage:com.example.alpha\n",
        ),
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    wait_for(&mut panel, |panel| {
        return log_has(panel, LogLevel::Success, "Found 1 application.");
    });

    assert!(set_filter(&mut panel, true));

    wait_for(&mut panel, |panel| {
        return log_has(panel, LogLevel::Success, "Found 2 applications.");
    });
    mock.assert_complete();
}
