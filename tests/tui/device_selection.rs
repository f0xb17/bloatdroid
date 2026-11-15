//! Tests for the Device Selection panel and the global focus navigation.
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

/// The serial of the device most tests discover.
const SERIAL: &str = "R58M12345";

/// The serial of the second device used by the device switching tests.
const SECOND_SERIAL: &str = "R58M99999";

/// The build fingerprint of a physical device.
const FINGERPRINT: &str =
    "google/flame/flame:13/TQ3A.230805.001/1:user/release-keys";

/// The applications that the device reports as installed by the user.
const THIRD_PARTY: [&str; 1] = ["com.example.alpha"];

/// The application that the device reports as a system application.
const SYSTEM: &str = "com.android.systemui";

/// Every application, with the system application sorted first.
const ALL_APPLICATIONS: [&str; 2] = [SYSTEM, "com.example.alpha"];

/// How long a test waits before it looks at the application again.
const WAIT_STEP: Duration = Duration::from_millis(5);

/// How many times a test looks at the application before it gives up.
const WAIT_LIMIT: usize = 400;

/// Builds a key press without modifiers.
fn key(code: KeyCode) -> KeyEvent {
    return KeyEvent::new(code, KeyModifiers::NONE);
}

/// Expects one `getprop` read of a discovered device.
fn expect_property(mock: &MockAdb, serial: &str, name: &str, value: &str) {
    mock.expect(
        &["-s", serial, "shell", "getprop", name],
        MockResponse::output(value),
    );
}

/// Expects the discovery of every given online physical device.
fn expect_discovery(mock: &MockAdb, serials: &[&str]) {
    let mut listing = String::from("List of devices attached\n");
    for serial in serials {
        listing.push_str(serial);
        listing.push_str("\tdevice product:flame model:Pixel_4 device:flame\n");
    }
    mock.expect(&["devices", "-l"], MockResponse::output(listing));

    for serial in serials {
        expect_property(mock, serial, PROP_QEMU, "0");
        expect_property(mock, serial, PROP_MANUFACTURER, "Google");
        expect_property(mock, serial, PROP_MODEL, "Pixel 4");
        expect_property(mock, serial, PROP_DEVICE, "flame");
        expect_property(mock, serial, PROP_ANDROID_VERSION, "13");
        expect_property(mock, serial, PROP_SDK_VERSION, "33");
        expect_property(mock, serial, PROP_BUILD_FINGERPRINT, FINGERPRINT);
        mock.expect(
            &["-s", serial, "get-state"],
            MockResponse::output("device"),
        );
    }
}

/// Expects the package listing of the device with the given filter.
fn expect_listing(mock: &MockAdb, serial: &str, include_system: bool) {
    let applications = if include_system {
        ALL_APPLICATIONS.as_slice()
    } else {
        THIRD_PARTY.as_slice()
    };

    let mut listing = String::new();
    for package in applications {
        listing.push_str("package:");
        listing.push_str(package);
        listing.push('\n');
    }

    if include_system {
        mock.expect(
            &["-s", serial, "shell", "pm", "list", "packages"],
            MockResponse::output(listing),
        );
        return;
    }
    mock.expect(
        &["-s", serial, "shell", "pm", "list", "packages", "-3"],
        MockResponse::output(listing),
    );
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

/// Polls device discovery until the device list is filled.
fn wait_for_devices(app: &mut App) {
    for _ in 0..WAIT_LIMIT {
        app.poll_discovery();
        if !app.devices().is_empty() {
            return;
        }
        thread::sleep(WAIT_STEP);
    }
    panic!("device discovery did not find the device");
}

/// Returns the application rows of the Standby Crates panel.
fn entry_texts(app: &App) -> Vec<String> {
    let mut texts = Vec::new();
    for line in app.control().lines(true) {
        if line.is_header() || line.is_status() {
            continue;
        }
        let text = match line.state() {
            Some(state) => format!("{}  {state}", line.text()),
            None => String::from(line.text()),
        };
        texts.push(text);
    }
    return texts;
}

/// Returns whether the Operations Log holds the given message.
fn log_has(app: &App, level: LogLevel, message: &str) -> bool {
    for entry in app.control().log().snapshot() {
        if entry.level() == level && entry.message() == message {
            return true;
        }
    }
    return false;
}

#[test]
fn starts_with_the_device_list_focused() {
    let mock = MockAdb::new();
    expect_discovery(&mock, &[SERIAL]);

    let mut app = start_app(&mock);

    assert_eq!(app.focused(), Focus::DeviceSelection);
    assert!(app.device_list_is_focused());
    assert!(!app.filter_is_focused());

    wait_for_devices(&mut app);

    assert_eq!(app.focused(), Focus::DeviceSelection);
    assert!(app.device_list_is_focused());
    assert!(app.selected_device().is_some());
    mock.assert_complete();
}

#[test]
fn tabs_forward_through_every_focus_target() {
    let mock = MockAdb::new();
    expect_discovery(&mock, &[SERIAL]);

    let mut app = start_app(&mock);
    wait_for_devices(&mut app);

    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.focused(), Focus::DeviceSelectionFilter);
    assert!(app.filter_is_focused());

    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.focused(), Focus::RemovePackages);

    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.focused(), Focus::StandbyCrates);

    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.focused(), Focus::Operations);

    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.focused(), Focus::DeviceSelection);
    assert!(app.device_list_is_focused());
    mock.assert_complete();
}

#[test]
fn shift_tab_moves_backward_through_every_focus_target() {
    let mock = MockAdb::new();
    expect_discovery(&mock, &[SERIAL]);

    let mut app = start_app(&mock);
    wait_for_devices(&mut app);

    let back = key(KeyCode::BackTab);

    app.handle_key(back);
    assert_eq!(app.focused(), Focus::Operations);

    app.handle_key(back);
    assert_eq!(app.focused(), Focus::StandbyCrates);

    app.handle_key(back);
    assert_eq!(app.focused(), Focus::RemovePackages);

    app.handle_key(back);
    assert_eq!(app.focused(), Focus::DeviceSelectionFilter);

    app.handle_key(back);
    assert_eq!(app.focused(), Focus::DeviceSelection);
    mock.assert_complete();
}

#[test]
fn space_toggles_the_checkbox_only_while_it_has_focus() {
    let mock = MockAdb::new();
    expect_discovery(&mock, &[SERIAL]);
    // Turning the setting on reads the complete inventory once.
    expect_listing(&mock, SERIAL, true);

    let mut app = start_app(&mock);
    wait_for_devices(&mut app);

    app.handle_key(key(KeyCode::Tab));
    app.handle_key(key(KeyCode::Char(' ')));

    assert!(app.show_system());
    assert!(!app.overlay_is_open());
    assert!(log_has(&app, LogLevel::Info, "Showing system applications."));

    // SPACE in another panel leaves the setting alone.
    app.handle_key(key(KeyCode::Tab));
    app.handle_key(key(KeyCode::Char(' ')));
    assert!(app.show_system());

    wait_until(&mut app, |_| {
        return mock.is_complete();
    });
    mock.assert_complete();
}

#[test]
fn space_on_the_device_list_opens_the_device_information_overlay() {
    let mock = MockAdb::new();
    expect_discovery(&mock, &[SERIAL]);

    let mut app = start_app(&mock);
    wait_for_devices(&mut app);

    app.handle_key(key(KeyCode::Char(' ')));

    assert!(app.overlay_is_open());
    assert!(!app.show_system());
    mock.assert_complete();
}

#[test]
fn switching_focus_keeps_the_selected_device() {
    let mock = MockAdb::new();
    expect_discovery(&mock, &[SECOND_SERIAL, SERIAL]);

    let mut app = start_app(&mock);
    wait_for_devices(&mut app);

    // UP and DOWN keep selecting devices while the list has focus.
    assert_eq!(app.selected_index(), Some(0));
    app.handle_key(key(KeyCode::Down));
    assert_eq!(app.selected_index(), Some(1));
    app.handle_key(key(KeyCode::Up));
    assert_eq!(app.selected_index(), Some(0));

    // Changing the focus never changes the selected device, and the device keys
    // only work while the device list has focus.
    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.focused(), Focus::DeviceSelectionFilter);
    app.handle_key(key(KeyCode::Down));
    app.handle_key(key(KeyCode::Up));
    assert_eq!(app.selected_index(), Some(0));

    app.handle_key(key(KeyCode::BackTab));
    assert_eq!(app.focused(), Focus::DeviceSelection);
    app.handle_key(key(KeyCode::Down));
    assert_eq!(app.selected_index(), Some(1));
    mock.assert_complete();
}

#[test]
fn enabling_the_filter_updates_every_application_list() {
    let mock = MockAdb::new();
    expect_discovery(&mock, &[SERIAL]);
    expect_listing(&mock, SERIAL, false);
    expect_listing(&mock, SERIAL, true);
    expect_buckets(&mock, &ALL_APPLICATIONS);

    let mut app = start_app(&mock);
    wait_for_devices(&mut app);
    wait_until(&mut app, |app| {
        return log_has(app, LogLevel::Success, "Found 1 application.");
    });

    // The checkbox is reached from the device list, and enabling it refreshes
    // the inventory of the selected device.
    app.handle_key(key(KeyCode::Tab));
    app.handle_key(key(KeyCode::Char(' ')));
    wait_until(&mut app, |app| {
        return log_has(app, LogLevel::Success, "Found 2 applications.");
    });

    // The Standby Crates panel lists the same inventory, system application
    // included.
    app.handle_key(key(KeyCode::Tab));
    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.focused(), Focus::StandbyCrates);
    app.handle_key(key(KeyCode::Char(' ')));

    wait_until(&mut app, |app| {
        return entry_texts(app).len() == ALL_APPLICATIONS.len();
    });

    let expected = vec![
        String::from("com.android.systemui  active"),
        String::from("com.example.alpha  active"),
    ];
    assert_eq!(entry_texts(&app), expected);
    assert!(app.show_system());
    mock.assert_complete();
}
