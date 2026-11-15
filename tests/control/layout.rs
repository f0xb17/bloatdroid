//! Tests for the two-column layout of the application lists.
//!
//! The renderer lays an application row out with `application_row`, so these
//! tests exercise that helper directly and check through the panel that every
//! section hands it a package name and a state label.

use crossterm::event::KeyCode;

use bloatdroid::control::ControlLine;
use bloatdroid::control::ControlPanel;
use bloatdroid::control::common::application_row;

use crate::common::MockAdb;
use crate::common::MockResponse;

use super::harness::ANDROID_13;
use super::harness::SERIAL;
use super::harness::entry_texts;
use super::harness::panel_for;
use super::harness::wait_for;

/// The width the section tests lay their rows out in.
const WIDTH: usize = 48;

/// The width of a row that is too narrow for both columns.
const NARROW: usize = 8;

/// Returns how many characters a text has.
fn width_of(text: &str) -> usize {
    let mut width = 0;
    for _ in text.chars() {
        width += 1;
    }
    return width;
}

/// Expects the package listing of one application.
fn expect_listing(mock: &MockAdb, package: &str) {
    let mut listing = String::new();
    listing.push_str("package:");
    listing.push_str(package);
    listing.push('\n');
    mock.expect(
        &["-s", SERIAL, "shell", "pm", "list", "packages", "-3"],
        MockResponse::output(listing),
    );
}

/// Returns the only application row of the panel.
fn only_entry(panel: &ControlPanel) -> ControlLine {
    let mut rows = Vec::new();
    for line in panel.lines(true) {
        if line.is_header() || line.is_status() {
            continue;
        }
        rows.push(line);
    }
    assert_eq!(rows.len(), 1, "expected exactly one application row");

    let mut rows = rows.into_iter();
    return match rows.next() {
        Some(row) => row,
        None => panic!("the application row disappeared"),
    };
}

/// Asserts that a row shows the name on the left and the state label right
/// aligned against the right edge of a row of [`WIDTH`] characters.
fn assert_two_columns(row: &ControlLine, name: &str, state: &str) {
    assert_eq!(row.text(), name);
    assert_eq!(row.state(), Some(state));

    let text = application_row(row.text(), state, WIDTH);
    let padding = WIDTH - width_of(name) - width_of(state);
    let expected = format!("{name}{}{state}", " ".repeat(padding));
    assert_eq!(text, expected);
    assert_eq!(width_of(&text), WIDTH);
}

/// Polls until the expanded section shows its single application.
fn wait_for_row(panel: &mut ControlPanel) {
    wait_for(panel, |panel| {
        return entry_texts(panel).len() == 1;
    });
}

#[test]
fn aligns_the_state_label_against_the_right_edge() {
    let row = application_row("com.example.app", "active", 32);

    assert_eq!(row, format!("com.example.app{}{}", " ".repeat(11), "active"));
    assert_eq!(width_of(&row), 32);
}

#[test]
fn ends_the_state_label_in_the_same_column_for_every_name_length() {
    let short = application_row("a", "active", 32);
    let long = application_row("com.example.a.very.long.name", "active", 40);

    assert_eq!(short, format!("a{}{}", " ".repeat(25), "active"));
    assert_eq!(
        long,
        format!("com.example.a.very.long.name{}{}", " ".repeat(6), "active")
    );
    assert_eq!(width_of(&short), 32);
    assert_eq!(width_of(&long), 40);
}

#[test]
fn truncates_a_long_package_name_with_an_ellipsis() {
    let name = "com.example.a.very.long.application.name";
    let row = application_row(name, "active", 32);

    assert_eq!(width_of(&row), 32);
    assert!(row.starts_with("com.example.a.very.long…"));
    assert!(row.ends_with("active"));
    assert!(!row.contains(name), "the name was not shortened: {row}");
}

#[test]
fn keeps_the_complete_state_label() {
    let name = "com.example.a.very.long.application.name";

    for state in ["active", "working_set", "restricted", "default"] {
        let row = application_row(name, state, WIDTH);
        assert!(row.ends_with(state), "the label {state} was cut off: {row}");
        assert_eq!(width_of(&row), WIDTH);
    }
}

#[test]
fn keeps_the_state_label_at_the_edge_of_a_narrow_row() {
    let row = application_row("com.example.app", "active", NARROW);

    assert_eq!(row, "  active");
    assert_eq!(width_of(&row), NARROW);
}

#[test]
fn shortens_a_state_label_that_does_not_fit_on_its_own() {
    let row = application_row("com.example.app", "working_set", 6);

    assert_eq!(row, "worki…");
    assert_eq!(width_of(&row), 6);
}

#[test]
fn buckets_rows_align_the_state_label() {
    let mock = MockAdb::new();
    expect_listing(&mock, "com.example.app");
    mock.expect(
        &["-s", SERIAL, "shell", "am", "get-standby-bucket", "com.example.app"],
        MockResponse::output("working_set"),
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for_row(&mut panel);

    assert_two_columns(&only_entry(&panel), "com.example.app", "working_set");
    mock.assert_complete();
}

#[test]
fn appops_rows_align_the_state_label() {
    let mock = MockAdb::new();
    expect_listing(&mock, "com.example.app");
    mock.expect(
        &[
            "-s",
            SERIAL,
            "shell",
            "cmd",
            "appops",
            "get",
            "com.example.app",
            "RUN_ANY_IN_BACKGROUND",
        ],
        MockResponse::output("RUN_ANY_IN_BACKGROUND: ignore"),
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    assert!(panel.handle_key(KeyCode::Down));
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for_row(&mut panel);

    assert_two_columns(&only_entry(&panel), "com.example.app", "ignore");
    mock.assert_complete();
}

#[test]
fn inactive_rows_align_the_state_label() {
    let mock = MockAdb::new();
    expect_listing(&mock, "com.example.app");
    mock.expect(
        &["-s", SERIAL, "shell", "am", "get-inactive", "com.example.app"],
        MockResponse::output("Idle=true\n"),
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    assert!(panel.handle_key(KeyCode::Down));
    assert!(panel.handle_key(KeyCode::Down));
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for_row(&mut panel);

    assert_two_columns(&only_entry(&panel), "com.example.app", "inactive");
    mock.assert_complete();
}

#[test]
fn doze_rows_align_the_state_label() {
    let mock = MockAdb::new();
    expect_listing(&mock, "com.example.app");
    mock.expect(
        &["-s", SERIAL, "shell", "cmd", "deviceidle", "whitelist"],
        MockResponse::output("user,com.example.app,10123\n"),
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    for _ in 0..3 {
        assert!(panel.handle_key(KeyCode::Down));
    }
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for_row(&mut panel);

    assert_two_columns(&only_entry(&panel), "com.example.app", "user");
    mock.assert_complete();
}

#[test]
fn keeps_the_complete_row_of_the_focused_application() {
    let mock = MockAdb::new();
    let mut listing = String::new();
    for package in ["com.example.alpha", "com.example.mike"] {
        listing.push_str("package:");
        listing.push_str(package);
        listing.push('\n');
    }
    mock.expect(
        &["-s", SERIAL, "shell", "pm", "list", "packages", "-3"],
        MockResponse::output(listing),
    );
    mock.expect(
        &[
            "-s",
            SERIAL,
            "shell",
            "am",
            "get-standby-bucket",
            "com.example.alpha",
        ],
        MockResponse::output("active"),
    );
    mock.expect(
        &[
            "-s",
            SERIAL,
            "shell",
            "am",
            "get-standby-bucket",
            "com.example.mike",
        ],
        MockResponse::output("working_set"),
    );

    let mut panel = panel_for(&mock, ANDROID_13);
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for(&mut panel, |panel| {
        return entry_texts(panel).len() == 2;
    });
    // The cursor moves from the header onto the first application.
    assert!(panel.handle_key(KeyCode::Down));

    let mut focused = 0;
    for line in panel.lines(true) {
        if !line.is_focused() {
            continue;
        }
        focused += 1;
        assert_eq!(line.text(), "com.example.alpha");
        assert_eq!(line.state(), Some("active"));
    }
    assert_eq!(focused, 1, "exactly one row should hold the cursor");
    mock.assert_complete();
}
