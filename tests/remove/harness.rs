//! Devices for the removal tests.
//!
//! A device is discovered through the mock ADB runner, so the tests use the same
//! representation the application does and no Android device is needed.

use bloatdroid::discover::AndroidDevice;
use bloatdroid::discover::DeviceScanner;
use bloatdroid::discover::parser::PROP_ANDROID_VERSION;
use bloatdroid::discover::parser::PROP_BUILD_FINGERPRINT;
use bloatdroid::discover::parser::PROP_DEVICE;
use bloatdroid::discover::parser::PROP_MANUFACTURER;
use bloatdroid::discover::parser::PROP_MODEL;
use bloatdroid::discover::parser::PROP_QEMU;
use bloatdroid::discover::parser::PROP_SDK_VERSION;

use crate::common::MockAdb;
use crate::common::MockResponse;

/// The serial of the device the removal tests target.
pub const SERIAL: &str = "R58M12345";

/// The build fingerprint of the physical device the tests discover.
const FINGERPRINT: &str =
    "google/flame/flame:13/TQ3A.230805.001/1:user/release-keys";

/// Discovers one online physical device through the mock.
pub fn device() -> AndroidDevice {
    return scanned_device("device");
}

/// Discovers a device that reports another ADB state.
pub fn device_in_state(state: &str) -> AndroidDevice {
    return scanned_device(state);
}

/// Discovers one online physical device and keeps its mock.
///
/// The returned mock still answers every command of the discovery, so a test
/// adds the commands of its operation and asserts that nothing else ran.
pub fn device_with_mock() -> (MockAdb, AndroidDevice) {
    let mock = MockAdb::new();
    expect_discovery(&mock, "device");
    let device = scan(&mock);
    mock.assert_complete();
    return (mock, device);
}

/// Expects the commands of one device discovery.
fn expect_discovery(mock: &MockAdb, state: &str) {
    let listing = format!(
        "List of devices attached\n{SERIAL}\tdevice product:flame model:Pixel_4 device:flame\n"
    );
    mock.expect(&["devices", "-l"], MockResponse::output(listing));

    let properties = [
        (PROP_QEMU, "0"),
        (PROP_MANUFACTURER, "Google"),
        (PROP_MODEL, "Pixel 4"),
        (PROP_DEVICE, "flame"),
        (PROP_ANDROID_VERSION, "13"),
        (PROP_SDK_VERSION, "33"),
        (PROP_BUILD_FINGERPRINT, FINGERPRINT),
    ];
    for (name, value) in properties {
        mock.expect(
            &["-s", SERIAL, "shell", "getprop", name],
            MockResponse::output(value),
        );
    }
    mock.expect(&["-s", SERIAL, "get-state"], MockResponse::output(state));
}

/// Runs one discovery with the mock and returns its device.
fn scan(mock: &MockAdb) -> AndroidDevice {
    let scanner = DeviceScanner::new(mock.client());
    let mut devices = match scanner.scan() {
        Ok(devices) => devices,
        Err(error) => panic!("the device discovery failed: {error}"),
    };
    let device = match devices.pop() {
        Some(device) => device,
        None => panic!("the device discovery found no device"),
    };
    return device;
}

/// Runs one discovery with a mock of its own.
fn scanned_device(state: &str) -> AndroidDevice {
    let mock = MockAdb::new();
    expect_discovery(&mock, state);
    let device = scan(&mock);
    mock.assert_complete();
    return device;
}
