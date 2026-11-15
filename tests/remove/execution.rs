//! Tests for the execution interface of package removal.
//!
//! The strategy below builds a command but never runs it, so the suite cannot
//! remove a package even on a device.

use bloatdroid::remove::RemovalCommand;
use bloatdroid::remove::RemovalError;
use bloatdroid::remove::RemovalMethod;
use bloatdroid::remove::RemovalOperation;
use bloatdroid::remove::RemovalPackage;
use bloatdroid::remove::RemovalRequest;
use bloatdroid::remove::RemovalStrategy;
use bloatdroid::remove::validate;

use super::harness::SERIAL;
use super::harness::device;

/// A strategy that only describes its command.
struct ExampleStrategy {
    method: RemovalMethod,
}

impl RemovalStrategy for ExampleStrategy {
    fn method(&self) -> RemovalMethod {
        return self.method;
    }

    fn command(&self, request: &RemovalRequest) -> RemovalCommand {
        let arguments = ["shell", "example", request.package().identifier()];
        return RemovalCommand::for_device(request.device(), &arguments);
    }
}

/// Validates a request for a freshly discovered device.
fn request(method: RemovalMethod) -> RemovalRequest {
    let device = device();
    let package = RemovalPackage::new("com.example.app");
    match validate(Some(&device), package, method) {
        Ok(request) => return request,
        Err(error) => panic!("a valid request was rejected: {error}"),
    }
}

/// Pairs a request with the strategy of the given method.
fn operation(
    method: RemovalMethod,
    strategy_method: RemovalMethod,
) -> Result<RemovalOperation, RemovalError> {
    let strategy = Box::new(ExampleStrategy {
        method: strategy_method,
    });
    return RemovalOperation::new(request(method), strategy);
}

#[test]
fn associates_an_operation_with_its_device_and_package() {
    let method = RemovalMethod::UserUninstall;
    let operation = match operation(method, method) {
        Ok(operation) => operation,
        Err(error) => panic!("the operation was rejected: {error}"),
    };

    assert_eq!(operation.device().identifier, SERIAL);
    assert_eq!(operation.package().identifier(), "com.example.app");
    assert_eq!(operation.method(), RemovalMethod::UserUninstall);
    assert_eq!(operation.request().package().identifier(), "com.example.app");
}

#[test]
fn builds_the_command_of_its_strategy() {
    let method = RemovalMethod::SystemPackage;
    let operation = match operation(method, method) {
        Ok(operation) => operation,
        Err(error) => panic!("the operation was rejected: {error}"),
    };

    assert_eq!(
        operation.command().as_adb_arguments(),
        vec!["-s", SERIAL, "shell", "example", "com.example.app"]
    );
}

#[test]
fn refuses_a_strategy_of_another_method() {
    // Each method has its own permissions and safety rules, so a request is
    // never run by a strategy of another method.
    let outcome =
        operation(RemovalMethod::SystemPackage, RemovalMethod::UserUninstall);

    match outcome {
        Err(RemovalError::UnsupportedMethod) => {}
        Err(error) => panic!("the wrong error was reported: {error}"),
        Ok(_) => panic!("a mismatched strategy was accepted"),
    }
}
