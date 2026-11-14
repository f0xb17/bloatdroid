//! Tests for the ADB client and its runners.

use std::io;

use bloatdroid::adb::AdbClient;
use bloatdroid::adb::AdbError;

use crate::common::MockAdb;
use crate::common::MockResponse;

/// The arguments of a `get-state` command.
const GET_STATE: [&str; 3] = ["-s", "R58M12345", "get-state"];

/// Runs the fixed test command and returns its outcome.
fn run_command(client: &AdbClient) -> Result<String, AdbError> {
    return client.run(&GET_STATE);
}

/// Returns the expected arguments as owned strings.
fn expected_arguments() -> Vec<String> {
    let mut arguments = Vec::new();
    for argument in GET_STATE {
        arguments.push(String::from(argument));
    }
    return arguments;
}

#[test]
fn passes_arguments_to_adb() {
    let mock = MockAdb::new();
    mock.expect(&GET_STATE, MockResponse::output("device\n"));

    let output = match run_command(&mock.client()) {
        Ok(output) => output,
        Err(error) => panic!("the command failed: {error}"),
    };

    assert_eq!(output, "device\n");
    assert_eq!(mock.calls(), vec![expected_arguments()]);
    mock.assert_complete();
}

#[test]
fn reports_command_failures_with_standard_error() {
    let mock = MockAdb::new();
    mock.expect(&GET_STATE, MockResponse::failure("error: device offline"));

    let error = match run_command(&mock.client()) {
        Ok(output) => panic!("expected a failure, got {output}"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("device offline"));
    match &error {
        AdbError::CommandFailed {
            status,
            stderr,
            ..
        } => {
            assert!(!status.success());
            assert_eq!(stderr, "error: device offline");
        }
        _ => panic!("expected a command failure, got {error}"),
    }
    mock.assert_complete();
}

#[test]
fn reports_a_missing_executable_from_the_mock() {
    let mock = MockAdb::new();
    mock.expect(&GET_STATE, MockResponse::missing());

    match run_command(&mock.client()) {
        Ok(output) => panic!("expected a failure, got {output}"),
        Err(AdbError::Io(error)) => {
            assert_eq!(error.kind(), io::ErrorKind::NotFound);
        }
        Err(error) => panic!("expected an I/O failure, got {error}"),
    }
    mock.assert_complete();
}

#[test]
fn reports_a_missing_executable_of_the_real_runner() {
    // The path does not exist, so no process is ever started.
    let client = AdbClient::with_executable("/nonexistent/bloatdroid/adb");

    match client.run(&["devices"]) {
        Ok(output) => panic!("expected a failure, got {output}"),
        Err(AdbError::Io(error)) => {
            assert_eq!(error.kind(), io::ErrorKind::NotFound);
        }
        Err(error) => panic!("expected an I/O failure, got {error}"),
    }
}

#[test]
fn fails_unexpected_commands_without_running_adb() {
    let mock = MockAdb::new();

    match run_command(&mock.client()) {
        Ok(output) => panic!("expected a failure, got {output}"),
        Err(error) => {
            assert!(error.to_string().contains("unexpected adb command"));
        }
    }

    assert_eq!(mock.calls(), vec![expected_arguments()]);
    assert_eq!(mock.unexpected(), vec![expected_arguments()]);
    assert!(!mock.is_complete());
}

#[cfg(unix)]
#[test]
fn real_runner_returns_standard_output() {
    // `sh` stands in for the ADB executable, so no ADB runs.
    let client = AdbClient::with_executable("sh");

    let output = match client.run(&["-c", "printf 'pixel 4'"]) {
        Ok(output) => output,
        Err(error) => panic!("the command failed: {error}"),
    };
    assert_eq!(output, "pixel 4");
}

#[cfg(unix)]
#[test]
fn real_runner_reports_exit_codes_and_standard_error() {
    let client = AdbClient::with_executable("sh");

    match client.run(&["-c", "printf 'standard error' >&2; exit 3"]) {
        Ok(output) => panic!("expected a failure, got {output}"),
        Err(AdbError::CommandFailed {
            status,
            stderr,
            ..
        }) => {
            assert_eq!(status.code(), Some(3));
            assert_eq!(stderr, "standard error");
        }
        Err(error) => panic!("expected a command failure, got {error}"),
    }
}

#[cfg(unix)]
#[test]
fn real_runner_rejects_output_that_is_not_utf8() {
    let client = AdbClient::with_executable("sh");

    match client.run(&["-c", "printf '\\377'"]) {
        Ok(output) => panic!("expected a failure, got {output}"),
        Err(AdbError::OutputNotUtf8 {
            ..
        }) => {}
        Err(error) => panic!("expected an encoding failure, got {error}"),
    }
}
