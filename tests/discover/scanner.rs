//! Tests for the device scanner.

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

/// The serial of the physical device most tests discover.
const SERIAL: &str = "R58M12345";

/// The build fingerprint of a physical device.
const FINGERPRINT: &str =
    "google/flame/flame:13/TQ3A.230805.001/1:user/release-keys";

/// The property values that one device reports.
#[derive(Clone, Copy)]
struct Properties<'a> {
    serial: &'a str,
    qemu: &'a str,
    model: &'a str,
    release: &'a str,
    sdk: &'a str,
    fingerprint: &'a str,
}

impl<'a> Properties<'a> {
    /// The properties of a physical Pixel 4 running Android 13.
    fn pixel(serial: &'a str) -> Self {
        return Self {
            serial,
            qemu: "0",
            model: "Pixel 4",
            release: "13",
            sdk: "33",
            fingerprint: FINGERPRINT,
        };
    }

    /// Adds one `getprop` expectation per property.
    fn expect(&self, mock: &MockAdb) {
        expect_property(mock, self.serial, PROP_QEMU, self.qemu);
        expect_property(mock, self.serial, PROP_MANUFACTURER, "Google");
        expect_property(mock, self.serial, PROP_MODEL, self.model);
        expect_property(mock, self.serial, PROP_DEVICE, "flame");
        expect_property(mock, self.serial, PROP_ANDROID_VERSION, self.release);
        expect_property(mock, self.serial, PROP_SDK_VERSION, self.sdk);
        expect_property(
            mock,
            self.serial,
            PROP_BUILD_FINGERPRINT,
            self.fingerprint,
        );
    }
}

/// Returns the `devices -l` line of an online device.
fn online_line(serial: &str) -> String {
    return format!(
        "{serial}\tdevice product:flame model:Pixel_4 device:flame"
    );
}

/// Expects the `devices -l` answer with the given device lines.
fn expect_listing(mock: &MockAdb, lines: &[String]) {
    let mut listing = String::from("List of devices attached\n");
    for line in lines {
        listing.push_str(line);
        listing.push('\n');
    }
    mock.expect(&["devices", "-l"], MockResponse::output(listing));
}

/// Expects one `getprop` read.
fn expect_property(mock: &MockAdb, serial: &str, name: &str, value: &str) {
    mock.expect(
        &["-s", serial, "shell", "getprop", name],
        MockResponse::output(value),
    );
}

/// Expects the `get-state` read of one device.
fn expect_state(mock: &MockAdb, serial: &str) {
    mock.expect(&["-s", serial, "get-state"], MockResponse::output("device"));
}

/// Expects the whole discovery of one online physical device.
fn expect_device(mock: &MockAdb, properties: &Properties<'_>) {
    properties.expect(mock);
    expect_state(mock, properties.serial);
}

/// Runs a scan and fails the test when ADB fails.
fn scan(mock: &MockAdb) -> Vec<AndroidDevice> {
    let scanner = DeviceScanner::new(mock.client());
    match scanner.scan() {
        Ok(devices) => return devices,
        Err(error) => panic!("the scan failed: {error}"),
    }
}

#[test]
fn discovers_no_devices() {
    let mock = MockAdb::new();
    mock.expect(
        &["devices", "-l"],
        MockResponse::output("List of devices attached\n"),
    );

    let devices = scan(&mock);

    assert!(devices.is_empty());
    mock.assert_complete();
}

#[test]
fn discovers_one_physical_device() {
    let mock = MockAdb::new();
    expect_listing(&mock, &[online_line(SERIAL)]);
    expect_device(&mock, &Properties::pixel(SERIAL));

    let devices = scan(&mock);

    assert_eq!(devices.len(), 1);
    let device = match devices.first() {
        Some(device) => device,
        None => panic!("the scan lost its only device"),
    };
    assert_eq!(device.identifier, SERIAL);
    assert_eq!(device.manufacturer, "Google");
    assert_eq!(device.model, "Pixel 4");
    assert_eq!(device.device, "flame");
    assert_eq!(device.android_version, "13");
    assert_eq!(device.sdk_version, "33");
    assert_eq!(device.build_fingerprint, FINGERPRINT);
    assert_eq!(device.state, "device");
    mock.assert_complete();
}

#[test]
fn sorts_devices_by_identifier() {
    let mock = MockAdb::new();
    let lines = vec![online_line("R58M99999"), online_line("R58M11111")];
    expect_listing(&mock, &lines);
    expect_device(&mock, &Properties::pixel("R58M99999"));
    expect_device(&mock, &Properties::pixel("R58M11111"));

    let devices = scan(&mock);

    let mut identifiers = Vec::new();
    for device in &devices {
        identifiers.push(device.identifier.clone());
    }
    let expected = vec![String::from("R58M11111"), String::from("R58M99999")];
    assert_eq!(identifiers, expected);
    mock.assert_complete();
}

#[test]
fn skips_offline_devices() {
    let mock = MockAdb::new();
    expect_listing(&mock, &[String::from("R58M12345\toffline")]);

    let devices = scan(&mock);

    assert!(devices.is_empty());
    mock.assert_complete();
}

#[test]
fn skips_unauthorized_devices() {
    let mock = MockAdb::new();
    expect_listing(&mock, &[String::from("R58M12345\tunauthorized")]);

    let devices = scan(&mock);

    assert!(devices.is_empty());
    mock.assert_complete();
}

#[test]
fn skips_emulators_by_serial_name() {
    let mock = MockAdb::new();
    expect_listing(&mock, &[online_line("emulator-5554")]);
    let mut properties = Properties::pixel("emulator-5554");
    properties.qemu = "1";
    properties.expect(&mock);

    let devices = scan(&mock);

    assert!(devices.is_empty());
    mock.assert_complete();
}

#[test]
fn skips_emulators_by_kernel_property() {
    let mock = MockAdb::new();
    expect_listing(&mock, &[online_line(SERIAL)]);
    let mut properties = Properties::pixel(SERIAL);
    properties.qemu = "1";
    properties.expect(&mock);

    let devices = scan(&mock);

    assert!(devices.is_empty());
    mock.assert_complete();
}

#[test]
fn skips_emulators_by_build_fingerprint() {
    let mock = MockAdb::new();
    expect_listing(&mock, &[online_line(SERIAL)]);
    let mut properties = Properties::pixel(SERIAL);
    properties.fingerprint =
        "generic/sdk/generic:13/TE1A.220922.021/1:eng/test-keys";
    properties.expect(&mock);

    let devices = scan(&mock);

    assert!(devices.is_empty());
    mock.assert_complete();
}

#[test]
fn skips_devices_that_cannot_describe_themselves() {
    let mock = MockAdb::new();
    expect_listing(&mock, &[online_line(SERIAL)]);
    let mut properties = Properties::pixel(SERIAL);
    properties.model = "";
    // The device answers everything, but a device without a model is rejected
    // instead of being shown with blank information.
    expect_device(&mock, &properties);

    let devices = scan(&mock);

    assert!(devices.is_empty());
    mock.assert_complete();
}

#[test]
fn skips_devices_with_unreadable_properties() {
    let mock = MockAdb::new();
    expect_listing(&mock, &[online_line(SERIAL)]);
    // The first property read fails, so no further property is read.
    mock.expect(
        &["-s", SERIAL, "shell", "getprop", PROP_QEMU],
        MockResponse::failure("error: device offline"),
    );

    let devices = scan(&mock);

    assert!(devices.is_empty());
    mock.assert_complete();
}

#[test]
fn propagates_scan_failures() {
    let mock = MockAdb::new();
    mock.expect(
        &["devices", "-l"],
        MockResponse::failure("error: adb server not running"),
    );

    let scanner = DeviceScanner::new(mock.client());
    let outcome = scanner.scan();

    assert!(outcome.is_err());
    mock.assert_complete();
}
