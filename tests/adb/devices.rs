//! Tests for the output parsers of the ADB module.

use bloatdroid::adb::parse_devices;
use bloatdroid::adb::parse_packages;

#[test]
fn parses_every_detail_of_a_device_line() {
    let output = concat!(
        "List of devices attached\n",
        "R58M12345\tdevice product:flame model:Pixel_4 device:flame",
        " transport_id:1\n",
    );

    let devices = parse_devices(output);

    assert_eq!(devices.len(), 1);
    let device = match devices.first() {
        Some(device) => device,
        None => panic!("the output lost its only device"),
    };
    assert_eq!(device.serial, "R58M12345");
    assert_eq!(device.state, "device");
    assert_eq!(device.product, Some(String::from("flame")));
    assert_eq!(device.model, Some(String::from("Pixel_4")));
    assert_eq!(device.device, Some(String::from("flame")));
}

#[test]
fn parses_the_devices_of_an_empty_list() {
    let output = "List of devices attached\n\n";

    assert!(parse_devices(output).is_empty());
}

#[test]
fn ignores_daemon_notes_and_blank_lines() {
    let output = concat!(
        "List of devices attached\n",
        "* daemon started successfully *\n",
        "\n",
    );

    assert!(parse_devices(output).is_empty());
}

#[test]
fn keeps_devices_without_details() {
    let devices = parse_devices("emulator-5554\toffline\n");

    assert_eq!(devices.len(), 1);
    let device = match devices.first() {
        Some(device) => device,
        None => panic!("the output lost its only device"),
    };
    assert_eq!(device.serial, "emulator-5554");
    assert_eq!(device.state, "offline");
    assert!(device.product.is_none());
    assert!(device.model.is_none());
    assert!(device.device.is_none());
}

#[test]
fn ignores_details_without_a_key() {
    let devices = parse_devices("R58M12345\tdevice nonsense product:flame\n");

    let device = match devices.first() {
        Some(device) => device,
        None => panic!("the output lost its only device"),
    };
    assert_eq!(device.product, Some(String::from("flame")));
    assert!(device.model.is_none());
}

#[test]
fn parses_packages_sorted_and_without_duplicates() {
    let output = concat!(
        "package:com.example.charlie\n",
        "package:com.example.alpha\n",
        "package:com.example.alpha\n",
        "package:com.example.bravo\n",
    );

    let packages = parse_packages(output);

    let expected = vec![
        String::from("com.example.alpha"),
        String::from("com.example.bravo"),
        String::from("com.example.charlie"),
    ];
    assert_eq!(packages, expected);
}

#[test]
fn ignores_malformed_package_lines() {
    let output = concat!(
        "package:\n",
        "error: device offline\n",
        "com.example.nothing\n",
    );

    assert!(parse_packages(output).is_empty());
}
