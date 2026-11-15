//! Tests for the removal results and errors.

use std::error::Error;
use std::io;

use bloatdroid::adb::AdbError;
use bloatdroid::remove::RemovalError;
use bloatdroid::remove::RemovalOutcome;
use bloatdroid::remove::RemovalProgress;

#[test]
fn reports_a_removal_only_when_the_package_is_gone() {
    assert!(RemovalOutcome::Removed.is_removed());

    // A successful command is not proof of a removal.
    assert!(!RemovalOutcome::StillPresent.is_removed());
    assert!(!RemovalOutcome::NotInstalled.is_removed());
    assert!(
        !RemovalOutcome::NotRemoved(RemovalError::PermissionDenied)
            .is_removed()
    );
}

#[test]
fn keeps_every_failure_reason_apart() {
    let errors = [
        RemovalError::PermissionDenied,
        RemovalError::UnsupportedMethod,
        RemovalError::Adb(AdbError::Io(io::Error::other("adb is missing"))),
        RemovalError::VerificationFailed(String::from("the listing failed")),
        RemovalError::Failed(String::from("the device rejected the change")),
    ];

    for error in errors {
        let outcome = RemovalOutcome::NotRemoved(error);
        assert!(!outcome.is_removed());
    }
}

#[test]
fn describes_what_a_removal_achieved() {
    assert_eq!(RemovalOutcome::Removed.to_string(), "removed");
    assert_eq!(RemovalOutcome::StillPresent.to_string(), "still present");
    assert_eq!(RemovalOutcome::NotInstalled.to_string(), "not installed");

    let outcome = RemovalOutcome::NotRemoved(RemovalError::VerificationFailed(
        String::from("no answer"),
    ));
    assert_eq!(
        outcome.to_string(),
        "not removed: the removal could not be verified: no answer"
    );
}

#[test]
fn describes_every_failure_reason() {
    assert!(RemovalError::PermissionDenied.to_string().contains("permission"));
    assert!(
        RemovalError::UnsupportedMethod.to_string().contains("not supported")
    );

    let adb =
        RemovalError::Adb(AdbError::Io(io::Error::other("adb is missing")));
    assert!(adb.to_string().contains("adb is missing"));

    let verification =
        RemovalError::VerificationFailed(String::from("no answer"));
    assert!(verification.to_string().contains("could not be verified"));

    let failed =
        RemovalError::Failed(String::from("the device gave no reason"));
    assert!(failed.to_string().contains("the device gave no reason"));
}

#[test]
fn exposes_the_adb_failure_as_the_source() {
    let adb =
        RemovalError::Adb(AdbError::Io(io::Error::other("adb is missing")));
    assert!(adb.source().is_some());
    assert!(RemovalError::PermissionDenied.source().is_none());
}

#[test]
fn reports_progress_without_removal_work() {
    // The progress steps only describe what a caller reports; carrying an
    // outcome does not run or verify anything.
    assert!(!matches!(RemovalProgress::Started, RemovalProgress::Finished(_)));
    assert!(!matches!(
        RemovalProgress::Verifying,
        RemovalProgress::Finished(_)
    ));

    match RemovalProgress::Finished(RemovalOutcome::Removed) {
        RemovalProgress::Finished(outcome) => assert!(outcome.is_removed()),
        _ => panic!("the progress step lost its outcome"),
    }
}
