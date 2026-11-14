//! Tests for the individual ADB commands.

use bloatdroid::adb::get_prop;
use bloatdroid::adb::get_state;
use bloatdroid::adb::list_devices;
use bloatdroid::adb::list_packages;

use crate::common::MockAdb;
use crate::common::MockResponse;

/// The serial of the device the tests talk to.
const SERIAL: &str = "R58M12345";

#[test]
fn reads_a_system_property_and_trims_it() {
    let mock = MockAdb::new();
    let arguments = ["-s", SERIAL, "shell", "getprop", "ro.product.model"];
    mock.expect(&arguments, MockResponse::output("Pixel 4\n"));

    let value = match get_prop(&mock.client(), SERIAL, "ro.product.model") {
        Ok(value) => value,
        Err(error) => panic!("the property read failed: {error}"),
    };

    assert_eq!(value, "Pixel 4");
    mock.assert_complete();
}

#[test]
fn reads_the_connection_state() {
    let mock = MockAdb::new();
    mock.expect(&["-s", SERIAL, "get-state"], MockResponse::output("device\n"));

    let state = match get_state(&mock.client(), SERIAL) {
        Ok(state) => state,
        Err(error) => panic!("the state read failed: {error}"),
    };

    assert_eq!(state, "device");
    mock.assert_complete();
}

#[test]
fn lists_devices_with_their_details() {
    let mock = MockAdb::new();
    let listing = concat!(
        "List of devices attached\n",
        "R58M12345\tdevice product:flame model:Pixel_4 device:flame",
        " transport_id:1\n",
    );
    mock.expect(&["devices", "-l"], MockResponse::output(listing));

    let devices = match list_devices(&mock.client()) {
        Ok(devices) => devices,
        Err(error) => panic!("the device listing failed: {error}"),
    };

    assert_eq!(devices.len(), 1);
    let device = match devices.first() {
        Some(device) => device,
        None => panic!("the listing lost its only device"),
    };
    assert_eq!(device.serial, SERIAL);
    assert_eq!(device.state, "device");
    assert_eq!(device.model, Some(String::from("Pixel_4")));
    assert_eq!(device.product, Some(String::from("flame")));
    mock.assert_complete();
}

#[test]
fn lists_installed_packages_sorted() {
    let mock = MockAdb::new();
    let listing = "package:com.example.bravo\npackage:com.example.alpha\n";
    mock.expect(
        &["-s", SERIAL, "shell", "pm", "list", "packages"],
        MockResponse::output(listing),
    );

    let packages = match list_packages(&mock.client(), SERIAL) {
        Ok(packages) => packages,
        Err(error) => panic!("the package listing failed: {error}"),
    };

    let expected = vec![
        String::from("com.example.alpha"),
        String::from("com.example.bravo"),
    ];
    assert_eq!(packages, expected);
    mock.assert_complete();
}

#[test]
fn propagates_property_failures() {
    let mock = MockAdb::new();
    let arguments = ["-s", SERIAL, "shell", "getprop", "ro.product.model"];
    mock.expect(&arguments, MockResponse::failure("error: no devices"));

    let outcome = get_prop(&mock.client(), SERIAL, "ro.product.model");

    assert!(outcome.is_err());
    mock.assert_complete();
}

#[test]
fn propagates_state_failures() {
    let mock = MockAdb::new();
    mock.expect(
        &["-s", SERIAL, "get-state"],
        MockResponse::failure("error: device offline"),
    );

    let outcome = get_state(&mock.client(), SERIAL);

    assert!(outcome.is_err());
    mock.assert_complete();
}

#[test]
fn propagates_listing_failures() {
    let mock = MockAdb::new();
    mock.expect(
        &["devices", "-l"],
        MockResponse::failure("error: adb server not running"),
    );

    let outcome = list_devices(&mock.client());

    assert!(outcome.is_err());
    mock.assert_complete();
}
