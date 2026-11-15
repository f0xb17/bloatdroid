//! Tests for the user-level uninstall method.
//!
//! Every command is answered by the mock ADB runner, so no Android device is
//! needed and no package can be removed.

use bloatdroid::discover::AndroidDevice;
use bloatdroid::log::LogLevel;
use bloatdroid::log::OperationLog;
use bloatdroid::remove::RemovalError;
use bloatdroid::remove::RemovalMethod;
use bloatdroid::remove::RemovalOutcome;
use bloatdroid::remove::RemovalPackage;
use bloatdroid::remove::RemovalRequest;
use bloatdroid::remove::RemovalStrategy;
use bloatdroid::remove::UserUninstall;
use bloatdroid::remove::validate;

use crate::common::MockAdb;
use crate::common::MockResponse;

use super::harness::SERIAL;
use super::harness::device_with_mock;

/// The active user that most tests read from the device.
const USER: &str = "10";

/// The package that most tests uninstall.
const PACKAGE: &str = "com.example.app";

/// Validates the request of the test package.
fn request(device: &AndroidDevice, package: &str) -> RemovalRequest {
    let removal = RemovalPackage::new(package);
    match validate(Some(device), removal, RemovalMethod::UserUninstall) {
        Ok(request) => return request,
        Err(error) => panic!("a valid request was rejected: {error}"),
    }
}

/// Runs the user-level uninstall of the package through the mock.
fn uninstall(
    mock: &MockAdb,
    device: &AndroidDevice,
    package: &str,
) -> (RemovalOutcome, OperationLog) {
    let log = OperationLog::new();
    let request = request(device, package);
    let strategy = UserUninstall::new(mock.client());

    let outcome = strategy.run(&request, &log);
    mock.assert_complete();
    return (outcome, log);
}

/// Expects the read of the active Android user.
fn expect_user(mock: &MockAdb, answer: MockResponse) {
    mock.expect(&["-s", SERIAL, "shell", "am", "get-current-user"], answer);
}

/// Expects one installation check for one user.
fn expect_installation(
    mock: &MockAdb,
    user: &str,
    package: &str,
    installed: bool,
) {
    let listing = if installed {
        format!("package:{package}\n")
    } else {
        String::new()
    };
    mock.expect(
        &[
            "-s", SERIAL, "shell", "pm", "list", "packages", "--user", user,
            package,
        ],
        MockResponse::output(listing),
    );
}

/// Expects the uninstall of the package for one user.
fn expect_uninstall(
    mock: &MockAdb,
    user: &str,
    package: &str,
    answer: MockResponse,
) {
    mock.expect(
        &["-s", SERIAL, "shell", "pm", "uninstall", "--user", user, package],
        answer,
    );
}

/// Returns whether the log holds the message at the level.
fn log_has(log: &OperationLog, level: LogLevel, message: &str) -> bool {
    for entry in log.snapshot() {
        if entry.level() == level && entry.message() == message {
            return true;
        }
    }
    return false;
}

/// Returns whether any log message contains the fragment.
fn log_contains(log: &OperationLog, fragment: &str) -> bool {
    for entry in log.snapshot() {
        if entry.message().contains(fragment) {
            return true;
        }
    }
    return false;
}

/// Returns every log message, joined for text assertions.
fn log_text(log: &OperationLog) -> String {
    let mut text = String::new();
    for entry in log.snapshot() {
        text.push_str(entry.message());
        text.push('\n');
    }
    return text;
}

#[test]
fn uninstalls_for_the_active_user_and_verifies() {
    let (mock, device) = device_with_mock();
    expect_user(&mock, MockResponse::output(format!("{USER}\n")));
    expect_installation(&mock, USER, PACKAGE, true);
    expect_uninstall(&mock, USER, PACKAGE, MockResponse::output("Success\n"));
    expect_installation(&mock, USER, PACKAGE, false);

    let (outcome, log) = uninstall(&mock, &device, PACKAGE);

    assert!(outcome.is_removed());
    assert!(log_has(
        &log,
        LogLevel::Info,
        &format!("Preparing user-level uninstall of {PACKAGE}.")
    ));
    assert!(log_has(
        &log,
        LogLevel::Info,
        &format!("Target Android user: {USER}.")
    ));
    assert!(log_has(
        &log,
        LogLevel::Info,
        &format!("Uninstalling {PACKAGE} for user {USER}.")
    ));
    assert!(log_has(
        &log,
        LogLevel::Success,
        &format!(
            "User-level uninstall of {PACKAGE} for user {USER} succeeded and was verified."
        )
    ));
}

#[test]
fn refuses_a_package_that_is_not_installed() {
    let (mock, device) = device_with_mock();
    expect_user(&mock, MockResponse::output(format!("{USER}\n")));
    expect_installation(&mock, USER, PACKAGE, false);

    let (outcome, log) = uninstall(&mock, &device, PACKAGE);

    assert!(matches!(outcome, RemovalOutcome::NotInstalled));
    assert!(log_has(
        &log,
        LogLevel::Info,
        &format!("Package {PACKAGE} is not installed for user {USER}.")
    ));
}

#[test]
fn reports_a_rejected_uninstall() {
    let (mock, device) = device_with_mock();
    expect_user(&mock, MockResponse::output(format!("{USER}\n")));
    expect_installation(&mock, USER, PACKAGE, true);
    expect_uninstall(
        &mock,
        USER,
        PACKAGE,
        MockResponse::output("Failure [DELETE_FAILED_INTERNAL_ERROR]\n"),
    );

    let (outcome, log) = uninstall(&mock, &device, PACKAGE);

    match outcome {
        RemovalOutcome::NotRemoved(RemovalError::Rejected {
            reason,
        }) => {
            assert_eq!(reason, "DELETE_FAILED_INTERNAL_ERROR");
        }
        _ => panic!("the rejected uninstall was not reported as rejected"),
    }
    assert!(log_has(
        &log,
        LogLevel::Error,
        &format!(
            "Package Manager rejected the user-level uninstall of {PACKAGE} for user {USER}: DELETE_FAILED_INTERNAL_ERROR."
        )
    ));
}

#[test]
fn reports_a_permission_refusal() {
    let (mock, device) = device_with_mock();
    expect_user(&mock, MockResponse::output(format!("{USER}\n")));
    expect_installation(&mock, USER, PACKAGE, true);
    expect_uninstall(
        &mock,
        USER,
        PACKAGE,
        MockResponse::output("Failure [DELETE_FAILED_USER_RESTRICTED]\n"),
    );

    let (outcome, log) = uninstall(&mock, &device, PACKAGE);

    assert!(matches!(
        outcome,
        RemovalOutcome::NotRemoved(RemovalError::PermissionDenied)
    ));
    assert!(log_has(
        &log,
        LogLevel::Error,
        &format!(
            "Package Manager denied permission for the user-level uninstall of {PACKAGE}."
        )
    ));
}

#[test]
fn reports_a_permission_refusal_from_adb() {
    let (mock, device) = device_with_mock();
    expect_user(&mock, MockResponse::output(format!("{USER}\n")));
    expect_installation(&mock, USER, PACKAGE, true);
    expect_uninstall(
        &mock,
        USER,
        PACKAGE,
        MockResponse::failure(
            "java.lang.SecurityException: Shell does not have permission",
        ),
    );

    let (outcome, log) = uninstall(&mock, &device, PACKAGE);

    assert!(matches!(
        outcome,
        RemovalOutcome::NotRemoved(RemovalError::PermissionDenied)
    ));
    assert!(log_contains(
        &log,
        "Uninstalling com.example.app for user 10 failed"
    ));
}

#[test]
fn reports_an_unsupported_operation() {
    let (mock, device) = device_with_mock();
    expect_user(&mock, MockResponse::output(format!("{USER}\n")));
    expect_installation(&mock, USER, PACKAGE, true);
    expect_uninstall(
        &mock,
        USER,
        PACKAGE,
        MockResponse::output("Failure [operation not supported]\n"),
    );

    let (outcome, log) = uninstall(&mock, &device, PACKAGE);

    assert!(matches!(
        outcome,
        RemovalOutcome::NotRemoved(RemovalError::UnsupportedMethod)
    ));
    assert!(log_contains(
        &log,
        "Package Manager does not support the user-level uninstall of com.example.app"
    ));
}

#[test]
fn reports_a_package_that_is_still_installed() {
    let (mock, device) = device_with_mock();
    expect_user(&mock, MockResponse::output(format!("{USER}\n")));
    expect_installation(&mock, USER, PACKAGE, true);
    expect_uninstall(&mock, USER, PACKAGE, MockResponse::output("Success\n"));
    expect_installation(&mock, USER, PACKAGE, true);

    let (outcome, log) = uninstall(&mock, &device, PACKAGE);

    // A successful command alone is not a removal.
    assert!(matches!(outcome, RemovalOutcome::StillPresent));
    assert!(log_has(
        &log,
        LogLevel::Error,
        &format!(
            "Package {PACKAGE} is still installed for user {USER} after the user-level uninstall."
        )
    ));
    assert!(!log_contains(&log, "succeeded and was verified"));
}

#[test]
fn reports_a_failed_verification() {
    let (mock, device) = device_with_mock();
    expect_user(&mock, MockResponse::output(format!("{USER}\n")));
    expect_installation(&mock, USER, PACKAGE, true);
    expect_uninstall(&mock, USER, PACKAGE, MockResponse::output("Success\n"));
    mock.expect(
        &[
            "-s", SERIAL, "shell", "pm", "list", "packages", "--user", USER,
            PACKAGE,
        ],
        MockResponse::failure("device offline"),
    );

    let (outcome, log) = uninstall(&mock, &device, PACKAGE);

    // The verification command itself failed, so the removal is an ADB
    // failure that the verification could not settle.
    assert!(matches!(
        outcome,
        RemovalOutcome::NotRemoved(RemovalError::Adb(_))
    ));
    assert!(log_contains(
        &log,
        "Could not verify the user-level uninstall of com.example.app"
    ));
}

#[test]
fn reports_an_ambiguous_verification() {
    let (mock, device) = device_with_mock();
    expect_user(&mock, MockResponse::output(format!("{USER}\n")));
    expect_installation(&mock, USER, PACKAGE, true);
    expect_uninstall(&mock, USER, PACKAGE, MockResponse::output("Success\n"));
    mock.expect(
        &[
            "-s", SERIAL, "shell", "pm", "list", "packages", "--user", USER,
            PACKAGE,
        ],
        MockResponse::output("Error: device offline\n"),
    );

    let (outcome, _log) = uninstall(&mock, &device, PACKAGE);

    // The answer does not say whether the package is installed, so the removal
    // is not reported as verified.
    assert!(matches!(
        outcome,
        RemovalOutcome::NotRemoved(RemovalError::VerificationFailed(_))
    ));
}

#[test]
fn reports_an_ambiguous_installation_check() {
    let (mock, device) = device_with_mock();
    expect_user(&mock, MockResponse::output(format!("{USER}\n")));
    mock.expect(
        &[
            "-s", SERIAL, "shell", "pm", "list", "packages", "--user", USER,
            PACKAGE,
        ],
        MockResponse::output("Error: unknown option\n"),
    );

    let (outcome, log) = uninstall(&mock, &device, PACKAGE);

    assert!(matches!(
        outcome,
        RemovalOutcome::NotRemoved(RemovalError::VerificationFailed(_))
    ));
    assert!(log_contains(
        &log,
        "Could not check whether com.example.app is installed"
    ));
}

#[test]
fn rejects_an_invalid_user_id() {
    let (mock, device) = device_with_mock();
    expect_user(&mock, MockResponse::output("not-a-user\n"));

    let (outcome, log) = uninstall(&mock, &device, PACKAGE);

    match outcome {
        RemovalOutcome::NotRemoved(RemovalError::InvalidUser {
            output,
        }) => {
            assert_eq!(output, "not-a-user");
        }
        _ => panic!("an invalid user was accepted"),
    }
    assert!(log_contains(&log, "Could not determine the current Android user"));
}

#[test]
fn rejects_a_missing_user_id() {
    let (mock, device) = device_with_mock();
    expect_user(&mock, MockResponse::output("\n"));

    let (outcome, _log) = uninstall(&mock, &device, PACKAGE);

    match outcome {
        RemovalOutcome::NotRemoved(RemovalError::InvalidUser {
            output,
        }) => {
            assert!(output.is_empty());
        }
        _ => panic!("a missing user was accepted"),
    }
}

#[test]
fn reports_a_failed_user_read() {
    let (mock, device) = device_with_mock();
    expect_user(&mock, MockResponse::failure("device offline"));

    let (outcome, log) = uninstall(&mock, &device, PACKAGE);

    assert!(matches!(
        outcome,
        RemovalOutcome::NotRemoved(RemovalError::Adb(_))
    ));
    assert!(log_contains(&log, "Could not determine the current Android user"));
}

#[test]
fn keeps_a_system_package_removal_user_level() {
    let (mock, device) = device_with_mock();
    let system = "com.android.systemui";
    expect_user(&mock, MockResponse::output(format!("{USER}\n")));
    expect_installation(&mock, USER, system, true);
    expect_uninstall(&mock, USER, system, MockResponse::output("Success\n"));
    expect_installation(&mock, USER, system, false);

    let (outcome, log) = uninstall(&mock, &device, system);

    assert!(outcome.is_removed());
    assert!(log_has(
        &log,
        LogLevel::Success,
        &format!(
            "User-level uninstall of {system} for user {USER} succeeded and was verified."
        )
    ));

    // The method is reported as a user-level uninstall, never as the permanent
    // removal of an APK from the system image.
    let text = log_text(&log);
    assert!(text.contains("user-level uninstall"));
    assert!(!text.contains("permanent"));
    assert!(!text.contains("system image"));
}
