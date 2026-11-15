//! Tests for the Remove Packages panel.
//!
//! The application is driven through its public interface with a mocked ADB
//! runner, so no terminal and no Android device are involved.

use std::thread;
use std::time::Duration;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

use bloatdroid::discover::parser::PROP_ANDROID_VERSION;
use bloatdroid::discover::parser::PROP_BUILD_FINGERPRINT;
use bloatdroid::discover::parser::PROP_DEVICE;
use bloatdroid::discover::parser::PROP_MANUFACTURER;
use bloatdroid::discover::parser::PROP_MODEL;
use bloatdroid::discover::parser::PROP_QEMU;
use bloatdroid::discover::parser::PROP_SDK_VERSION;
use bloatdroid::log::LogLevel;
use bloatdroid::tui::App;
use bloatdroid::tui::Focus;

use crate::common::MockAdb;
use crate::common::MockResponse;

/// The serial of the device the tests discover.
const SERIAL: &str = "R58M12345";

/// The build fingerprint of a physical device.
const FINGERPRINT: &str =
    "google/flame/flame:13/TQ3A.230805.001/1:user/release-keys";

/// The application installed by the user.
const PACKAGE: &str = "com.example.alpha";

/// A second application installed by the user.
const SECOND_PACKAGE: &str = "com.example.beta";

/// The application that the device reports as a system application.
const SYSTEM_PACKAGE: &str = "com.android.systemui";

/// How long a test waits before it looks at the application again.
const WAIT_STEP: Duration = Duration::from_millis(5);

/// How many times a test looks at the application before it gives up.
const WAIT_LIMIT: usize = 400;

/// Builds a key press without modifiers.
fn key(code: KeyCode) -> KeyEvent {
    return KeyEvent::new(code, KeyModifiers::NONE);
}

/// Expects one `getprop` read of the discovered device.
fn expect_property(mock: &MockAdb, name: &str, value: &str) {
    mock.expect(
        &["-s", SERIAL, "shell", "getprop", name],
        MockResponse::output(value),
    );
}

/// Expects the discovery of the device.
fn expect_discovery(mock: &MockAdb) {
    let listing = format!(
        "List of devices attached\n{SERIAL}\tdevice product:flame model:Pixel_4 device:flame\n"
    );
    mock.expect(&["devices", "-l"], MockResponse::output(listing));

    expect_property(mock, PROP_QEMU, "0");
    expect_property(mock, PROP_MANUFACTURER, "Google");
    expect_property(mock, PROP_MODEL, "Pixel 4");
    expect_property(mock, PROP_DEVICE, "flame");
    expect_property(mock, PROP_ANDROID_VERSION, "13");
    expect_property(mock, PROP_SDK_VERSION, "33");
    expect_property(mock, PROP_BUILD_FINGERPRINT, FINGERPRINT);
    mock.expect(&["-s", SERIAL, "get-state"], MockResponse::output("device"));
}

/// Expects the package listing of the device with the given filter.
fn expect_listing(mock: &MockAdb, include_system: bool, packages: &[&str]) {
    let mut listing = String::new();
    for package in packages {
        listing.push_str("package:");
        listing.push_str(package);
        listing.push('\n');
    }

    if include_system {
        mock.expect(
            &["-s", SERIAL, "shell", "pm", "list", "packages"],
            MockResponse::output(listing),
        );
        return;
    }
    mock.expect(
        &["-s", SERIAL, "shell", "pm", "list", "packages", "-3"],
        MockResponse::output(listing),
    );
}

/// Expects the read of the active Android user.
fn expect_user(mock: &MockAdb, user: &str) {
    mock.expect(
        &["-s", SERIAL, "shell", "am", "get-current-user"],
        MockResponse::output(user),
    );
}

/// Expects one installation check for the user.
fn expect_installation(
    mock: &MockAdb,
    user: &str,
    package: &str,
    installed: bool,
) {
    let listing = if installed {
        format!("package:{package}\n")
    } else {
        String::new()
    };
    mock.expect(
        &[
            "-s", SERIAL, "shell", "pm", "list", "packages", "--user", user,
            package,
        ],
        MockResponse::output(listing),
    );
}

/// Expects the uninstall of the package for the user.
fn expect_uninstall(
    mock: &MockAdb,
    user: &str,
    package: &str,
    answer: MockResponse,
) {
    mock.expect(
        &["-s", SERIAL, "shell", "pm", "uninstall", "--user", user, package],
        answer,
    );
}

/// Starts the application on top of the mock.
fn start_app(mock: &MockAdb) -> App {
    match App::new(mock.client()) {
        Ok(app) => return app,
        Err(error) => panic!("could not start the application: {error}"),
    }
}

/// Applies the background work that finished since the last check.
fn pump(app: &mut App) {
    app.poll_discovery();
    app.poll_control();
    app.poll_operations();
    app.poll_removal();
}

/// Polls until the application reached the expected state.
fn wait_until(app: &mut App, done: impl Fn(&App) -> bool) {
    for _ in 0..WAIT_LIMIT {
        pump(app);
        if done(app) {
            return;
        }
        thread::sleep(WAIT_STEP);
    }
    panic!("the application did not reach the expected state in time");
}

/// Starts the application and waits for the remove package list.
fn ready_app(mock: &MockAdb) -> App {
    let mut app = start_app(mock);
    wait_until(&mut app, |app| {
        return !app.removal_packages().is_empty();
    });
    return app;
}

/// Moves the focus from the device list to the Remove Packages panel.
fn focus_remove_packages(app: &mut App) {
    app.handle_key(key(KeyCode::Tab));
    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.focused(), Focus::RemovePackages);
}

/// Returns the number of commands that used the given argument.
fn count_arguments(mock: &MockAdb, wanted: &str) -> usize {
    let mut count = 0;
    for call in mock.calls() {
        for argument in &call {
            if argument == wanted {
                count += 1;
                break;
            }
        }
    }
    return count;
}

/// Returns every message of the Operations Log.
fn log_messages(app: &App) -> Vec<String> {
    let mut messages = Vec::new();
    for entry in app.control().log().snapshot() {
        messages.push(String::from(entry.message()));
    }
    return messages;
}

/// Returns whether the Operations Log holds the message at the level.
fn log_has(app: &App, level: LogLevel, message: &str) -> bool {
    for entry in app.control().log().snapshot() {
        if entry.level() == level && entry.message() == message {
            return true;
        }
    }
    return false;
}

#[test]
fn lists_the_shared_application_inventory() {
    let mock = MockAdb::new();
    expect_discovery(&mock);
    expect_listing(&mock, false, &[PACKAGE, SECOND_PACKAGE]);

    let mut app = ready_app(&mock);
    focus_remove_packages(&mut app);

    assert_eq!(app.removal_selected_package(), Some(PACKAGE));
    assert_eq!(app.removal_packages(), vec![PACKAGE, SECOND_PACKAGE]);
    assert_eq!(app.removal_selected_index(), 0);

    app.handle_key(key(KeyCode::Down));
    assert_eq!(app.removal_selected_package(), Some(SECOND_PACKAGE));
    app.handle_key(key(KeyCode::Up));
    assert_eq!(app.removal_selected_package(), Some(PACKAGE));

    mock.assert_complete();
}

#[test]
fn requires_confirmation_before_uninstalling() {
    let mock = MockAdb::new();
    expect_discovery(&mock);
    expect_listing(&mock, false, &[PACKAGE]);

    let mut app = ready_app(&mock);
    focus_remove_packages(&mut app);

    // Selecting and pressing ENTER only asks for confirmation.
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.removal_confirmation_package(), Some(PACKAGE));
    assert_eq!(count_arguments(&mock, "uninstall"), 0);
    assert_eq!(count_arguments(&mock, "am"), 0);

    // Cancelling leaves the package alone and the application running.
    app.handle_key(key(KeyCode::Char('n')));
    assert_eq!(app.removal_confirmation_package(), None);
    assert!(app.is_running());
    assert_eq!(count_arguments(&mock, "uninstall"), 0);

    app.handle_key(key(KeyCode::Enter));
    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.removal_confirmation_package(), None);
    assert!(app.is_running());

    mock.assert_complete();
}

#[test]
fn confirms_and_uninstalls_the_selected_package() {
    let mock = MockAdb::new();
    expect_discovery(&mock);
    expect_listing(&mock, false, &[PACKAGE, SECOND_PACKAGE]);
    expect_user(&mock, "0\n");
    expect_installation(&mock, "0", SECOND_PACKAGE, true);
    expect_uninstall(
        &mock,
        "0",
        SECOND_PACKAGE,
        MockResponse::output("Success\n"),
    );
    expect_installation(&mock, "0", SECOND_PACKAGE, false);
    // The verified removal refreshes the shared inventory.
    expect_listing(&mock, false, &[PACKAGE]);

    let mut app = ready_app(&mock);
    focus_remove_packages(&mut app);

    app.handle_key(key(KeyCode::Down));
    app.handle_key(key(KeyCode::Enter));
    app.handle_key(key(KeyCode::Char('y')));

    assert!(app.removal_is_running());
    wait_until(&mut app, |app| {
        return log_has(
            app,
            LogLevel::Success,
            "User-level uninstall of com.example.beta for user 0 succeeded and was verified.",
        );
    });
    wait_until(&mut app, |app| {
        return app.removal_packages() == vec![PACKAGE];
    });

    assert!(!app.removal_is_running());
    assert_eq!(count_arguments(&mock, "uninstall"), 1);
    assert_eq!(app.removal_status(), Some("com.example.beta: removed"));
    mock.assert_complete();
}

#[test]
fn prevents_a_second_uninstall_while_one_runs() {
    let mock = MockAdb::new();
    expect_discovery(&mock);
    expect_listing(&mock, false, &[PACKAGE]);
    expect_user(&mock, "0\n");
    expect_installation(&mock, "0", PACKAGE, true);
    expect_uninstall(&mock, "0", PACKAGE, MockResponse::output("Success\n"));
    expect_installation(&mock, "0", PACKAGE, false);
    expect_listing(&mock, false, &[]);

    let mut app = ready_app(&mock);
    focus_remove_packages(&mut app);

    app.handle_key(key(KeyCode::Enter));
    app.handle_key(key(KeyCode::Char('y')));
    assert!(app.removal_is_running());

    // A second submission is refused while the first one runs.
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.removal_confirmation_package(), None);
    assert_eq!(app.removal_status(), Some("An uninstall is already running."));

    // The verified removal refreshes the inventory, so the list empties and
    // every expected command is answered.
    wait_until(&mut app, |app| {
        return !app.removal_is_running()
            && app.removal_packages().is_empty()
            && mock.is_complete();
    });
    assert_eq!(count_arguments(&mock, "uninstall"), 1);
    mock.assert_complete();
}

#[test]
fn reports_a_rejected_uninstall() {
    let mock = MockAdb::new();
    expect_discovery(&mock);
    expect_listing(&mock, false, &[PACKAGE]);
    expect_user(&mock, "0\n");
    expect_installation(&mock, "0", PACKAGE, true);
    expect_uninstall(
        &mock,
        "0",
        PACKAGE,
        MockResponse::output("Failure [DELETE_FAILED_INTERNAL_ERROR]\n"),
    );

    let mut app = ready_app(&mock);
    focus_remove_packages(&mut app);

    app.handle_key(key(KeyCode::Enter));
    app.handle_key(key(KeyCode::Char('y')));
    wait_until(&mut app, |app| {
        return !app.removal_is_running();
    });

    assert!(log_has(
        &app,
        LogLevel::Error,
        "Package Manager rejected the user-level uninstall of com.example.alpha for user 0: DELETE_FAILED_INTERNAL_ERROR.",
    ));
    assert!(!log_has(
        &app,
        LogLevel::Success,
        "User-level uninstall of com.example.alpha for user 0 succeeded and was verified.",
    ));
    assert_eq!(app.removal_packages(), vec![PACKAGE]);
    mock.assert_complete();
}

#[test]
fn refuses_a_package_that_does_not_validate() {
    let mock = MockAdb::new();
    expect_discovery(&mock);
    expect_listing(&mock, false, &[PACKAGE, "not valid"]);

    let mut app = ready_app(&mock);
    focus_remove_packages(&mut app);

    app.handle_key(key(KeyCode::Down));
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.removal_confirmation_package(), Some("not valid"));
    app.handle_key(key(KeyCode::Char('y')));

    // The invalid package never reaches the device.
    assert!(!app.removal_is_running());
    assert_eq!(count_arguments(&mock, "am"), 0);
    assert_eq!(count_arguments(&mock, "uninstall"), 0);
    assert!(
        log_has(
            &app,
            LogLevel::Error,
            "Cannot uninstall not valid: `not valid` is not a valid Android package name",
        ),
        "the log does not hold the validation error: {:?}",
        log_messages(&app)
    );
    mock.assert_complete();
}

#[test]
fn follows_the_system_application_filter() {
    let mock = MockAdb::new();
    expect_discovery(&mock);
    expect_listing(&mock, false, &[PACKAGE]);
    expect_listing(&mock, true, &[SYSTEM_PACKAGE, PACKAGE]);

    let mut app = ready_app(&mock);
    assert_eq!(app.removal_packages(), vec![PACKAGE]);

    app.handle_key(key(KeyCode::Tab));
    app.handle_key(key(KeyCode::Char(' ')));
    wait_until(&mut app, |app| {
        return app.removal_packages().len() == 2;
    });
    wait_until(&mut app, |app| {
        return log_has(app, LogLevel::Success, "Found 2 applications.");
    });

    // The Remove Packages panel lists the same inventory as every other
    // application list, system application included.
    assert_eq!(app.removal_packages(), vec![SYSTEM_PACKAGE, PACKAGE]);
    mock.assert_complete();
}
