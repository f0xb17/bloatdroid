//! Tests for the validation of removal requests.

use bloatdroid::remove::RemovalMethod;
use bloatdroid::remove::RemovalPackage;
use bloatdroid::remove::RemovalRequest;
use bloatdroid::remove::ValidationError;
use bloatdroid::remove::validate;

use super::harness::SERIAL;
use super::harness::device;
use super::harness::device_in_state;

/// Validates a request for a freshly discovered device.
fn request(
    package: &str,
    method: RemovalMethod,
) -> Result<RemovalRequest, ValidationError> {
    let device = device();
    return validate(Some(&device), RemovalPackage::new(package), method);
}

#[test]
fn accepts_a_valid_request() {
    let request = match request("com.example.app", RemovalMethod::UserUninstall)
    {
        Ok(request) => request,
        Err(error) => panic!("a valid request was rejected: {error}"),
    };

    assert_eq!(request.package().identifier(), "com.example.app");
    assert_eq!(request.device().identifier, SERIAL);
    assert_eq!(request.method(), RemovalMethod::UserUninstall);
}

#[test]
fn rejects_a_request_without_a_device() {
    let package = RemovalPackage::new("com.example.app");
    let outcome = validate(None, package, RemovalMethod::UserUninstall);

    assert!(matches!(outcome, Err(ValidationError::NoDevice)));
}

#[test]
fn rejects_a_device_that_cannot_run_commands() {
    let device = device_in_state("offline");
    let package = RemovalPackage::new("com.example.app");
    let outcome =
        validate(Some(&device), package, RemovalMethod::UserUninstall);

    match outcome {
        Err(ValidationError::UnusableDevice {
            state,
        }) => {
            assert_eq!(state, "offline");
        }
        _ => panic!("the unusable device was accepted"),
    }
}

#[test]
fn rejects_an_empty_package() {
    let outcome = request("", RemovalMethod::UserUninstall);

    assert!(matches!(outcome, Err(ValidationError::EmptyPackage)));
}

#[test]
fn rejects_malformed_package_names() {
    let names = [
        ".com.example",
        "com.example.",
        "com..example",
        "1com.example",
        "com.example-app",
        "com example",
        "com/example",
        "com.example;rm",
        " com.example",
        "com.example ",
        "com.exämple",
    ];

    for name in names {
        let outcome = request(name, RemovalMethod::UserUninstall);
        match outcome {
            Err(ValidationError::InvalidPackage {
                identifier,
            }) => {
                assert_eq!(identifier, name);
            }
            _ => panic!("{name:?} was accepted"),
        }
    }
}

#[test]
fn accepts_every_well_formed_identifier() {
    let names =
        ["android", "com.example.app", "com.example.App_2", "a.b.c.d.e1"];

    for name in names {
        let outcome = request(name, RemovalMethod::SystemPackage);
        match outcome {
            Ok(request) => assert_eq!(request.package().identifier(), name),
            Err(error) => panic!("{name:?} was rejected: {error}"),
        }
    }
}

#[test]
fn keeps_the_removal_method_separate_from_the_package() {
    let device = device();
    let package = RemovalPackage::new("com.example.app");
    let uninstall =
        validate(Some(&device), package.clone(), RemovalMethod::UserUninstall);
    let system = validate(Some(&device), package, RemovalMethod::SystemPackage);

    // The same package can be named for either method; what a method requires
    // from the device is not decided here.
    match (uninstall, system) {
        (Ok(uninstall), Ok(system)) => {
            assert_eq!(uninstall.method(), RemovalMethod::UserUninstall);
            assert_eq!(system.method(), RemovalMethod::SystemPackage);
            assert_eq!(uninstall.package(), system.package());
        }
        _ => panic!("a valid request was rejected"),
    }
}
