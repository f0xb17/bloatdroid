//! Tests for the execution interface of package removal.
//!
//! The strategy below only reports an outcome, so the suite cannot remove a
//! package even on a device.

use bloatdroid::log::LogLevel;
use bloatdroid::log::OperationLog;
use bloatdroid::remove::RemovalError;
use bloatdroid::remove::RemovalMethod;
use bloatdroid::remove::RemovalOperation;
use bloatdroid::remove::RemovalOutcome;
use bloatdroid::remove::RemovalPackage;
use bloatdroid::remove::RemovalRequest;
use bloatdroid::remove::RemovalStrategy;
use bloatdroid::remove::validate;

use super::harness::SERIAL;
use super::harness::device;

/// The message the example strategy reports when it runs.
const RAN: &str = "the example strategy ran";

/// A strategy that only describes its removal.
struct ExampleStrategy {
    method: RemovalMethod,
}

impl RemovalStrategy for ExampleStrategy {
    fn method(&self) -> RemovalMethod {
        return self.method;
    }

    fn run(
        &self,
        request: &RemovalRequest,
        log: &OperationLog,
    ) -> RemovalOutcome {
        let message = format!("{RAN} for {}", request.package().identifier());
        log.record(LogLevel::Info, message);
        return RemovalOutcome::Removed;
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
fn runs_the_strategy_with_the_operations_log() {
    let method = RemovalMethod::SystemPackage;
    let operation = match operation(method, method) {
        Ok(operation) => operation,
        Err(error) => panic!("the operation was rejected: {error}"),
    };
    let log = OperationLog::new();

    let outcome = operation.run(&log);

    assert!(outcome.is_removed());
    assert!(log_has(&log, "the example strategy ran for com.example.app"));
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

/// Returns whether the log holds a message with the given fragment.
fn log_has(log: &OperationLog, fragment: &str) -> bool {
    for entry in log.snapshot() {
        if entry.message().contains(fragment) {
            return true;
        }
    }
    return false;
}
