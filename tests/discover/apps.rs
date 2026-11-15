//! Tests for application package discovery and the global filter.
//!
//! The scanner runs ADB through the mock, so no device is needed. The tests
//! check which package manager command the filter produces and what an
//! inventory holds.

use bloatdroid::discover::ApplicationFilter;
use bloatdroid::discover::ApplicationScanner;
use bloatdroid::discover::PackageInventory;

use crate::common::MockAdb;
use crate::common::MockResponse;

/// The serial of the device the tests read.
const SERIAL: &str = "serial";

/// The command that lists only the applications the user installed.
const THIRD_PARTY_COMMAND: [&str; 7] =
    ["-s", SERIAL, "shell", "pm", "list", "packages", "-3"];

/// The command that lists every installed application.
const COMPLETE_COMMAND: [&str; 6] =
    ["-s", SERIAL, "shell", "pm", "list", "packages"];

/// Runs a scan and fails the test when ADB fails.
fn scan(mock: &MockAdb, filter: ApplicationFilter) -> PackageInventory {
    let scanner = ApplicationScanner::new(mock.client());
    match scanner.scan(SERIAL, filter) {
        Ok(packages) => return packages,
        Err(error) => panic!("the scan failed: {error}"),
    }
}

/// Returns the package names of an inventory as a plain list.
fn names(packages: &PackageInventory) -> Vec<String> {
    let mut names = Vec::new();
    for package in packages.iter() {
        names.push(package.clone());
    }
    return names;
}

#[test]
fn excludes_system_applications_by_default() {
    let filter = ApplicationFilter::new();
    assert!(!filter.show_system());

    let mock = MockAdb::new();
    mock.expect(
        &THIRD_PARTY_COMMAND,
        MockResponse::output(
            "package:com.example.alpha\npackage:com.example.bravo\n",
        ),
    );

    let packages = scan(&mock, filter);

    let expected = vec![
        String::from("com.example.alpha"),
        String::from("com.example.bravo"),
    ];
    assert_eq!(names(&packages), expected);
    mock.assert_complete();
}

#[test]
fn includes_system_applications_when_the_filter_is_enabled() {
    let mut filter = ApplicationFilter::new();
    assert!(filter.toggle());
    assert!(filter.show_system());

    let mock = MockAdb::new();
    mock.expect(
        &COMPLETE_COMMAND,
        MockResponse::output(
            "package:com.android.systemui\npackage:com.example.alpha\n",
        ),
    );

    let packages = scan(&mock, filter);

    let expected = vec![
        String::from("com.android.systemui"),
        String::from("com.example.alpha"),
    ];
    assert_eq!(names(&packages), expected);
    mock.assert_complete();
}

#[test]
fn toggling_the_filter_changes_what_the_package_manager_reports() {
    let mock = MockAdb::new();
    mock.expect(
        &THIRD_PARTY_COMMAND,
        MockResponse::output("package:com.example.alpha\n"),
    );
    mock.expect(
        &COMPLETE_COMMAND,
        MockResponse::output(
            "package:com.android.systemui\npackage:com.example.alpha\n",
        ),
    );

    let mut filter = ApplicationFilter::new();
    let filtered = scan(&mock, filter);
    assert_eq!(names(&filtered), vec![String::from("com.example.alpha")]);

    assert!(filter.toggle());
    let complete = scan(&mock, filter);
    assert_eq!(
        names(&complete),
        vec![
            String::from("com.android.systemui"),
            String::from("com.example.alpha"),
        ]
    );
    mock.assert_complete();
}

#[test]
fn leaving_the_filter_alone_keeps_the_third_party_command() {
    let filter = ApplicationFilter::with_show_system(false);
    assert!(!filter.show_system());
    assert_eq!(filter, ApplicationFilter::new());

    let enabled = ApplicationFilter::with_show_system(true);
    assert!(enabled.show_system());
    assert_ne!(enabled, filter);
}

#[test]
fn reports_a_listing_failure() {
    let mock = MockAdb::new();
    mock.expect(
        &THIRD_PARTY_COMMAND,
        MockResponse::failure("error: device offline"),
    );

    let scanner = ApplicationScanner::new(mock.client());
    let outcome = scanner.scan(SERIAL, ApplicationFilter::new());

    assert!(outcome.is_err(), "a failed listing produced an inventory");
    mock.assert_complete();
}
