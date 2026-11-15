//! Tests for the batched and bounded concurrent application-state scanning.

use std::time::Duration;

use crossterm::event::KeyCode;

use bloatdroid::control::ControlPanel;
use bloatdroid::control::common::batch::QUERY_WORKERS;

use crate::common::MockAdb;
use crate::common::MockResponse;

use super::harness::ANDROID_13;
use super::harness::SERIAL;
use super::harness::entry_texts;
use super::harness::panel_for;
use super::harness::wait_for;

/// How many applications the scanning tests read.
const PACKAGES: usize = 24;

/// The application whose bucket cannot be read by the failure test.
const FAILING: usize = 7;

/// Returns the package identifiers used by the scanning tests.
fn packages() -> Vec<String> {
    let mut packages = Vec::new();
    for index in 0..PACKAGES {
        packages.push(format!("com.example.app{index:02}"));
    }
    return packages;
}

/// Expects the listing and one bucket answer per package.
fn expect_scan(mock: &MockAdb, failing: Option<usize>) {
    let mut listing = String::new();
    for package in packages() {
        listing.push_str("package:");
        listing.push_str(&package);
        listing.push('\n');
    }
    mock.expect(
        &["-s", SERIAL, "shell", "pm", "list", "packages", "-3"],
        MockResponse::output(listing),
    );

    for (index, package) in packages().iter().enumerate() {
        let response = if Some(index) == failing {
            MockResponse::failure("error: device offline")
        } else {
            MockResponse::output("active")
        };
        mock.expect(
            &[
                "-s",
                SERIAL,
                "shell",
                "am",
                "get-standby-bucket",
                package.as_str(),
            ],
            response,
        );
    }
}

/// Expands the Standby Buckets section and waits for every row.
fn load_buckets(panel: &mut ControlPanel) {
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for(panel, |panel| {
        return entry_texts(panel).len() == PACKAGES;
    });
}

#[test]
fn runs_the_queries_within_the_concurrency_bound() {
    let mock = MockAdb::new();
    expect_scan(&mock, None);
    // A delay per command makes overlapping commands observable.
    mock.set_delay(Duration::from_millis(5));

    let mut panel = panel_for(&mock, ANDROID_13);
    load_buckets(&mut panel);

    let peak = mock.peak_concurrency();
    assert!(peak > 1, "the queries ran one after another: {peak}");
    assert!(
        peak <= QUERY_WORKERS,
        "more queries ran at once than the bound allows: {peak}"
    );
    mock.assert_complete();
}

#[test]
fn starts_one_process_per_package_and_none_more() {
    let mock = MockAdb::new();
    expect_scan(&mock, None);

    let mut panel = panel_for(&mock, ANDROID_13);
    load_buckets(&mut panel);

    // One listing and exactly one query per package.
    assert_eq!(mock.calls().len(), PACKAGES + 1);
    mock.assert_complete();
}

#[test]
fn keeps_one_failing_package_from_failing_the_others() {
    let mock = MockAdb::new();
    expect_scan(&mock, Some(FAILING));

    let mut panel = panel_for(&mock, ANDROID_13);
    load_buckets(&mut panel);

    let rows = entry_texts(&panel);
    assert_eq!(rows.len(), PACKAGES);

    let mut unknown = 0;
    for row in &rows {
        if row.ends_with("unknown") {
            unknown += 1;
        }
    }
    assert_eq!(
        unknown, 1,
        "exactly one package should be unreadable: {rows:?}"
    );
    mock.assert_complete();
}

#[test]
fn targets_the_selected_device_for_every_query() {
    let mock = MockAdb::new();
    expect_scan(&mock, None);

    let mut panel = panel_for(&mock, ANDROID_13);
    load_buckets(&mut panel);

    for call in mock.calls() {
        assert_eq!(call.first().map(String::as_str), Some("-s"));
        assert_eq!(call.get(1).map(String::as_str), Some(SERIAL));
    }
    mock.assert_complete();
}
