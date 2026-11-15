//! Tests for the package inventory that every control section shares.

use crossterm::event::KeyCode;

use bloatdroid::control::ControlPanel;
use bloatdroid::control::ControlTarget;
use bloatdroid::discover::apps::ApplicationFilter;

use crate::common::MockAdb;
use crate::common::MockResponse;

use super::harness::ANDROID_13;
use super::harness::SERIAL;
use super::harness::entry_texts;
use super::harness::panel_for;
use super::harness::set_filter;
use super::harness::wait_for;

/// The serial of the second device used by the device switching tests.
const OTHER_SERIAL: &str = "other-serial";

/// The applications that every section shows in these tests.
const APPLICATIONS: [&str; 2] = ["com.example.alpha", "com.example.bravo"];

/// The application that the device reports as a system application.
const SYSTEM: &str = "com.android.systemui";

/// Expects the package listing of one device.
fn expect_listing(mock: &MockAdb, serial: &str, applications: &[&str]) {
    let mut listing = String::new();
    for package in applications {
        listing.push_str("package:");
        listing.push_str(package);
        listing.push('\n');
    }
    mock.expect(
        &["-s", serial, "shell", "pm", "list", "packages", "-3"],
        MockResponse::output(listing),
    );
}

/// Expects the standby bucket read of every given application.
fn expect_buckets(mock: &MockAdb, serial: &str, applications: &[&str]) {
    for package in applications {
        mock.expect(
            &["-s", serial, "shell", "am", "get-standby-bucket", package],
            MockResponse::output("active"),
        );
    }
}

/// Expects the AppOps read of every given application.
fn expect_appops(mock: &MockAdb, serial: &str, applications: &[&str]) {
    for package in applications {
        mock.expect(
            &[
                "-s",
                serial,
                "shell",
                "cmd",
                "appops",
                "get",
                package,
                "RUN_ANY_IN_BACKGROUND",
            ],
            MockResponse::output("RUN_ANY_IN_BACKGROUND: allow\n"),
        );
    }
}

/// Expects the inactive state read of every given application.
fn expect_inactive(mock: &MockAdb, serial: &str, applications: &[&str]) {
    for package in applications {
        mock.expect(
            &["-s", serial, "shell", "am", "get-inactive", package],
            MockResponse::output("Idle=false\n"),
        );
    }
}

/// Expects one read of the complete whitelist.
fn expect_whitelist(mock: &MockAdb, serial: &str) {
    mock.expect(
        &["-s", serial, "shell", "cmd", "deviceidle", "whitelist"],
        MockResponse::output(""),
    );
}

/// Expects the complete package listing of one device.
fn expect_full_listing(mock: &MockAdb, serial: &str, applications: &[&str]) {
    let mut listing = String::new();
    for package in applications {
        listing.push_str("package:");
        listing.push_str(package);
        listing.push('\n');
    }
    mock.expect(
        &["-s", serial, "shell", "pm", "list", "packages"],
        MockResponse::output(listing),
    );
}

/// Returns how often the filtered package list of a device was read.
fn package_listings(mock: &MockAdb, serial: &str) -> usize {
    let wanted = vec![
        String::from("-s"),
        String::from(serial),
        String::from("shell"),
        String::from("pm"),
        String::from("list"),
        String::from("packages"),
        String::from("-3"),
    ];

    let mut count = 0;
    for call in mock.calls() {
        if call == wanted {
            count += 1;
        }
    }
    return count;
}

/// Returns how often the complete package list of a device was read.
fn full_listings(mock: &MockAdb, serial: &str) -> usize {
    let wanted = vec![
        String::from("-s"),
        String::from(serial),
        String::from("shell"),
        String::from("pm"),
        String::from("list"),
        String::from("packages"),
    ];

    let mut count = 0;
    for call in mock.calls() {
        if call == wanted {
            count += 1;
        }
    }
    return count;
}

/// Expands the section that holds the cursor.
fn expand(panel: &mut ControlPanel) {
    assert!(panel.handle_key(KeyCode::Char(' ')));
}

/// Collapses the section that holds the cursor.
fn collapse(panel: &mut ControlPanel) {
    assert!(panel.handle_key(KeyCode::Char(' ')));
}

/// Moves the cursor to the next section header.
fn focus_next_section(panel: &mut ControlPanel) {
    assert!(panel.handle_key(KeyCode::Down));
}

/// Waits until the expanded section shows the expected number of rows.
fn wait_for_rows(panel: &mut ControlPanel, count: usize) {
    wait_for(panel, |panel| {
        return entry_texts(panel).len() == count;
    });
}

#[test]
fn shares_one_listing_with_every_section() {
    let mock = MockAdb::new();
    expect_listing(&mock, SERIAL, &APPLICATIONS);
    expect_buckets(&mock, SERIAL, &APPLICATIONS);
    expect_appops(&mock, SERIAL, &APPLICATIONS);
    expect_inactive(&mock, SERIAL, &APPLICATIONS);
    expect_whitelist(&mock, SERIAL);

    let mut panel = panel_for(&mock, ANDROID_13);

    expand(&mut panel);
    wait_for_rows(&mut panel, 2);
    assert_eq!(
        entry_texts(&panel),
        vec![
            String::from("com.example.alpha  active"),
            String::from("com.example.bravo  active"),
        ]
    );
    collapse(&mut panel);

    focus_next_section(&mut panel);
    expand(&mut panel);
    wait_for_rows(&mut panel, 2);
    assert_eq!(
        entry_texts(&panel),
        vec![
            String::from("com.example.alpha  allow"),
            String::from("com.example.bravo  allow"),
        ]
    );
    collapse(&mut panel);

    focus_next_section(&mut panel);
    expand(&mut panel);
    wait_for_rows(&mut panel, 2);
    assert_eq!(
        entry_texts(&panel),
        vec![
            String::from("com.example.alpha  active"),
            String::from("com.example.bravo  active"),
        ]
    );
    collapse(&mut panel);

    focus_next_section(&mut panel);
    expand(&mut panel);
    wait_for_rows(&mut panel, 2);
    assert_eq!(
        entry_texts(&panel),
        vec![
            String::from("com.example.alpha  none"),
            String::from("com.example.bravo  none"),
        ]
    );

    // Every section worked from the same list, which was read once.
    assert_eq!(package_listings(&mock, SERIAL), 1);
    mock.assert_complete();
}

#[test]
fn reads_a_feature_state_only_when_its_section_expands() {
    let mock = MockAdb::new();
    expect_listing(&mock, SERIAL, &APPLICATIONS);
    expect_buckets(&mock, SERIAL, &APPLICATIONS);

    let mut panel = panel_for(&mock, ANDROID_13);
    expand(&mut panel);
    wait_for_rows(&mut panel, 2);

    // The listing and the two bucket reads, and nothing of the other sections.
    assert_eq!(mock.calls().len(), 3);
    mock.assert_complete();
}

#[test]
fn switching_devices_drops_the_inventory() {
    let mock = MockAdb::new();
    expect_listing(&mock, SERIAL, &APPLICATIONS);
    expect_buckets(&mock, SERIAL, &APPLICATIONS);
    expect_listing(&mock, OTHER_SERIAL, &["com.example.other"]);
    expect_buckets(&mock, OTHER_SERIAL, &["com.example.other"]);

    let mut panel = panel_for(&mock, ANDROID_13);
    expand(&mut panel);
    wait_for_rows(&mut panel, 2);

    let target = ControlTarget::new(
        String::from(OTHER_SERIAL),
        String::from(ANDROID_13),
    );
    assert!(panel.set_target(Some(target)));

    wait_for(&mut panel, |panel| {
        let expected = vec![String::from("com.example.other  active")];
        return entry_texts(panel) == expected;
    });

    // The new device brought its own list; the old one was not reused.
    assert_eq!(package_listings(&mock, SERIAL), 1);
    assert_eq!(package_listings(&mock, OTHER_SERIAL), 1);
    mock.assert_complete();
}

#[test]
fn refreshes_the_inventory_on_request() {
    let mock = MockAdb::new();
    expect_listing(&mock, SERIAL, &APPLICATIONS);
    expect_buckets(&mock, SERIAL, &APPLICATIONS);
    expect_listing(&mock, SERIAL, &APPLICATIONS);
    expect_buckets(&mock, SERIAL, &APPLICATIONS);

    let mut panel = panel_for(&mock, ANDROID_13);
    expand(&mut panel);
    wait_for_rows(&mut panel, 2);
    assert_eq!(package_listings(&mock, SERIAL), 1);

    assert!(panel.refresh_inventory());
    wait_for(&mut panel, |_| {
        return mock.is_complete();
    });

    assert_eq!(package_listings(&mock, SERIAL), 2);
    assert_eq!(entry_texts(&panel).len(), 2);
    mock.assert_complete();
}

#[test]
fn ignores_the_inventory_of_a_previous_device() {
    let mock = MockAdb::new();
    // The list of the first device is read, but the device changes before any
    // of its buckets is, so no bucket is ever read for it.
    expect_listing(&mock, SERIAL, &["com.example.first"]);
    expect_listing(&mock, OTHER_SERIAL, &["com.example.other"]);
    expect_buckets(&mock, OTHER_SERIAL, &["com.example.other"]);

    let mut panel = panel_for(&mock, ANDROID_13);
    expand(&mut panel);
    // The inventory for the first device is requested, then the device changes.
    panel.poll();

    let target = ControlTarget::new(
        String::from(OTHER_SERIAL),
        String::from(ANDROID_13),
    );
    assert!(panel.set_target(Some(target)));

    wait_for(&mut panel, |panel| {
        let expected = vec![String::from("com.example.other  active")];
        return entry_texts(panel) == expected;
    });

    assert_eq!(package_listings(&mock, SERIAL), 1);
    assert_eq!(package_listings(&mock, OTHER_SERIAL), 1);
    // Both listings and the one bucket read of the new device.
    assert_eq!(mock.calls().len(), 3);
    mock.assert_complete();
}

#[test]
fn loads_the_new_device_with_the_current_filter() {
    let mock = MockAdb::new();
    let complete = [SYSTEM, "com.example.alpha", "com.example.bravo"];

    expect_listing(&mock, SERIAL, &APPLICATIONS);
    expect_buckets(&mock, SERIAL, &APPLICATIONS);
    // Turning the setting on reads the complete inventory of the first
    // device.
    expect_full_listing(&mock, SERIAL, &complete);
    expect_buckets(&mock, SERIAL, &complete);
    // The device changes while the setting is on, so the new device is read
    // with the same setting.
    expect_full_listing(&mock, OTHER_SERIAL, &["com.example.other"]);
    expect_buckets(&mock, OTHER_SERIAL, &["com.example.other"]);

    let mut panel = panel_for(&mock, ANDROID_13);
    expand(&mut panel);
    wait_for_rows(&mut panel, 2);

    assert!(set_filter(&mut panel, true));
    wait_for_rows(&mut panel, 3);

    let target = ControlTarget::new(
        String::from(OTHER_SERIAL),
        String::from(ANDROID_13),
    );
    assert!(panel.set_target(Some(target)));

    wait_for(&mut panel, |panel| {
        let expected = vec![String::from("com.example.other  active")];
        return entry_texts(panel) == expected;
    });

    assert_eq!(full_listings(&mock, SERIAL), 1);
    assert_eq!(full_listings(&mock, OTHER_SERIAL), 1);
    assert_eq!(package_listings(&mock, OTHER_SERIAL), 0);
    mock.assert_complete();
}

#[test]
fn ignores_a_read_that_a_newer_filter_change_overtook() {
    let mock = MockAdb::new();
    let complete = [SYSTEM, "com.example.alpha", "com.example.bravo"];

    expect_listing(&mock, SERIAL, &APPLICATIONS);
    expect_buckets(&mock, SERIAL, &APPLICATIONS);
    // The setting is turned on and off again before the complete inventory
    // answers, so that answer is outdated as soon as it arrives.
    expect_full_listing(&mock, SERIAL, &complete);
    expect_listing(&mock, SERIAL, &APPLICATIONS);
    expect_buckets(&mock, SERIAL, &APPLICATIONS);

    let mut panel = panel_for(&mock, ANDROID_13);
    expand(&mut panel);
    wait_for_rows(&mut panel, 2);

    assert!(set_filter(&mut panel, true));
    panel.set_filter(ApplicationFilter::new());
    wait_for(&mut panel, |_| {
        return mock.is_complete();
    });

    // The newest read wins: the system application of the overtaken answer
    // never reaches the list.
    let expected = vec![
        String::from("com.example.alpha  active"),
        String::from("com.example.bravo  active"),
    ];
    assert_eq!(entry_texts(&panel), expected);
    assert_eq!(full_listings(&mock, SERIAL), 1);
    assert_eq!(package_listings(&mock, SERIAL), 2);
    mock.assert_complete();
}
