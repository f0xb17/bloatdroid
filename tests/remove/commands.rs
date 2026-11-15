//! Tests for the construction of removal commands.
//!
//! The commands are values only: nothing here runs ADB or removes a package.

use bloatdroid::remove::AndroidUser;
use bloatdroid::remove::RemovalCommand;
use bloatdroid::remove::RemovalMethod;
use bloatdroid::remove::RemovalPackage;
use bloatdroid::remove::commands::current_user_command;
use bloatdroid::remove::commands::installed_command;
use bloatdroid::remove::commands::uninstall_command;

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

#[test]
fn asks_the_device_for_its_active_user() {
    let device = device();
    let command = current_user_command(&device);

    assert_eq!(
        command.as_adb_arguments(),
        vec!["-s", SERIAL, "shell", "am", "get-current-user"]
    );
}

#[test]
fn names_the_user_and_the_package_of_an_uninstall() {
    let device = device();
    let package = RemovalPackage::new("com.example.app");
    let command = uninstall_command(&device, &package, AndroidUser::new(42));

    assert_eq!(
        command.as_adb_arguments(),
        vec![
            "-s",
            SERIAL,
            "shell",
            "pm",
            "uninstall",
            "--user",
            "42",
            "com.example.app",
        ]
    );
}

#[test]
fn never_targets_user_zero_unless_the_device_reported_it() {
    let device = device();
    let package = RemovalPackage::new("com.example.app");
    let command = uninstall_command(&device, &package, AndroidUser::new(7));

    // The user is always the one that was read from the device.
    assert!(command.arguments().contains(&String::from("7")));
    assert!(!command.arguments().contains(&String::from("0")));
}

#[test]
fn keeps_the_package_of_an_installation_check_separate() {
    let device = device();
    let package = RemovalPackage::new("com.example.app");
    let command = installed_command(&device, &package, AndroidUser::new(10));

    assert_eq!(
        command.as_adb_arguments(),
        vec![
            "-s",
            SERIAL,
            "shell",
            "pm",
            "list",
            "packages",
            "--user",
            "10",
            "com.example.app",
        ]
    );
}

#[test]
fn reports_the_android_user_as_the_package_manager_takes_it() {
    let user = AndroidUser::new(10);

    assert_eq!(user.id(), 10);
    assert_eq!(user.argument(), "10");
}
