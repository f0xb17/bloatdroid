//! Tests for the construction of removal commands.
//!
//! The commands are values only: nothing here runs ADB or removes a package.

use bloatdroid::remove::RemovalCommand;
use bloatdroid::remove::RemovalMethod;

use super::harness::SERIAL;
use super::harness::device;

#[test]
fn selects_the_device_with_its_own_argument() {
    let device = device();
    let command = RemovalCommand::for_device(&device, &["first", "second"]);

    let expected = vec![
        String::from("-s"),
        String::from(SERIAL),
        String::from("first"),
        String::from("second"),
    ];
    assert_eq!(command.arguments(), expected);
}

#[test]
fn keeps_a_package_name_as_one_argument() {
    let device = device();
    let package = "com.example.app; rm -rf /";
    let command = RemovalCommand::for_device(&device, &[package]);

    // The package stays one argument, so nothing is concatenated into a shell
    // command and the text cannot be interpreted by a shell.
    let expected =
        vec![String::from("-s"), String::from(SERIAL), String::from(package)];
    assert_eq!(command.arguments(), expected);
}

#[test]
fn passes_the_arguments_to_adb_as_separate_values() {
    let device = device();
    let command = RemovalCommand::for_device(&device, &["--user", "current"]);

    assert_eq!(
        command.as_adb_arguments(),
        vec!["-s", SERIAL, "--user", "current"]
    );
}

#[test]
fn names_each_removal_method_separately() {
    assert_ne!(RemovalMethod::UserUninstall, RemovalMethod::SystemPackage);
    assert_eq!(RemovalMethod::UserUninstall.name(), "user uninstall");
    assert_eq!(RemovalMethod::SystemPackage.name(), "system package removal");
}
