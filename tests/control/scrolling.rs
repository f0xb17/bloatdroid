//! Tests for the viewport-aware scrolling of the Standby Crates panel.

use crossterm::event::KeyCode;

use bloatdroid::control::ControlPanel;

use crate::common::MockAdb;
use crate::common::MockResponse;

use super::harness::ANDROID_13;
use super::harness::SERIAL;
use super::harness::entry_texts;
use super::harness::focused_text;
use super::harness::line_text;
use super::harness::panel_for;
use super::harness::wait_for;

/// The applications shown by the scrolling tests, in display order.
const APPLICATIONS: [&str; 8] = [
    "com.example.app1",
    "com.example.app2",
    "com.example.app3",
    "com.example.app4",
    "com.example.app5",
    "com.example.app6",
    "com.example.app7",
    "com.example.app8",
];

/// The title of the section that holds the cursor at the start.
const FIRST_HEADER: &str = "Standby Buckets";

/// Expects the listing and the bucket of every application.
fn expect_load(mock: &MockAdb) {
    let mut listing = String::new();
    for package in APPLICATIONS {
        listing.push_str("package:");
        listing.push_str(package);
        listing.push('\n');
    }
    mock.expect(
        &["-s", SERIAL, "shell", "pm", "list", "packages", "-3"],
        MockResponse::output(listing),
    );

    for package in APPLICATIONS {
        mock.expect(
            &["-s", SERIAL, "shell", "am", "get-standby-bucket", package],
            MockResponse::output("active"),
        );
    }
}

/// Returns the text of every visible line, in display order.
fn visible_texts(panel: &ControlPanel) -> Vec<String> {
    let mut texts = Vec::new();
    for line in panel.lines(true) {
        texts.push(line_text(&line));
    }
    return texts;
}

/// Returns the row text of one application.
fn row(index: usize) -> String {
    let package = match APPLICATIONS.get(index) {
        Some(package) => *package,
        None => panic!("no application at position {index}"),
    };
    return format!("{package}  active");
}

/// Expands the Standby Buckets section and waits for every row.
fn load_buckets(panel: &mut ControlPanel) {
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for(panel, |panel| {
        return entry_texts(panel).len() == APPLICATIONS.len();
    });
}

/// Moves the cursor down the given number of lines.
fn move_down(panel: &mut ControlPanel, count: usize) {
    for _ in 0..count {
        assert!(panel.handle_key(KeyCode::Down));
    }
}

/// Moves the cursor up the given number of lines.
fn move_up(panel: &mut ControlPanel, count: usize) {
    for _ in 0..count {
        assert!(panel.handle_key(KeyCode::Up));
    }
}

#[test]
fn keeps_the_cursor_visible_when_moving_down() {
    let mock = MockAdb::new();
    expect_load(&mock);

    let mut panel = panel_for(&mock, ANDROID_13);
    load_buckets(&mut panel);
    assert!(panel.set_viewport(4));

    // The cursor starts on the header, so the view shows the beginning.
    assert_eq!(
        visible_texts(&panel),
        vec![String::from(FIRST_HEADER), row(0), row(1), row(2),]
    );

    move_down(&mut panel, APPLICATIONS.len());
    assert_eq!(focused_text(&panel), Some(row(7)));
    assert_eq!(visible_texts(&panel), vec![row(4), row(5), row(6), row(7)]);
    mock.assert_complete();
}

#[test]
fn scrolls_back_up_to_the_headers() {
    let mock = MockAdb::new();
    expect_load(&mock);

    let mut panel = panel_for(&mock, ANDROID_13);
    load_buckets(&mut panel);
    assert!(panel.set_viewport(4));

    move_down(&mut panel, APPLICATIONS.len());
    move_up(&mut panel, APPLICATIONS.len());

    assert_eq!(focused_text(&panel), Some(String::from(FIRST_HEADER)));
    assert_eq!(
        visible_texts(&panel),
        vec![String::from(FIRST_HEADER), row(0), row(1), row(2),]
    );
    mock.assert_complete();
}

#[test]
fn scrolls_through_the_section_headers() {
    let mock = MockAdb::new();

    let mut panel = panel_for(&mock, ANDROID_13);
    assert!(panel.set_viewport(3));

    move_down(&mut panel, 3);
    assert_eq!(
        visible_texts(&panel),
        vec![
            String::from("AppOps"),
            String::from("App Inactive"),
            String::from("Doze Whitelist"),
        ]
    );
    assert_eq!(focused_text(&panel), Some(String::from("Doze Whitelist")));

    move_up(&mut panel, 3);
    assert_eq!(
        visible_texts(&panel),
        vec![
            String::from(FIRST_HEADER),
            String::from("AppOps"),
            String::from("App Inactive"),
        ]
    );
    // No device state is read while no section is expanded.
    mock.assert_complete();
}

#[test]
fn keeps_the_view_valid_when_a_section_expands_and_collapses() {
    let mock = MockAdb::new();
    expect_load(&mock);

    let mut panel = panel_for(&mock, ANDROID_13);
    assert!(panel.set_viewport(3));

    // Expanding with a short viewport keeps the header and the rows that fit.
    assert!(panel.handle_key(KeyCode::Char(' ')));
    wait_for(&mut panel, |panel| {
        return visible_texts(panel).get(1) == Some(&row(0));
    });
    assert_eq!(focused_text(&panel), Some(String::from(FIRST_HEADER)));
    assert_eq!(
        visible_texts(&panel),
        vec![String::from(FIRST_HEADER), row(0), row(1)]
    );

    move_down(&mut panel, 3);
    assert_eq!(focused_text(&panel), Some(row(2)));
    assert_eq!(visible_texts(&panel), vec![row(0), row(1), row(2)]);

    // Collapsing returns the cursor to the header, and the view with it.
    assert!(panel.handle_key(KeyCode::Char(' ')));
    assert_eq!(focused_text(&panel), Some(String::from(FIRST_HEADER)));
    assert_eq!(
        visible_texts(&panel),
        vec![
            String::from(FIRST_HEADER),
            String::from("AppOps"),
            String::from("App Inactive"),
        ]
    );
    mock.assert_complete();
}

#[test]
fn follows_the_viewport_when_the_terminal_resizes() {
    let mock = MockAdb::new();
    expect_load(&mock);

    let mut panel = panel_for(&mock, ANDROID_13);
    load_buckets(&mut panel);

    // The whole panel fits into a tall terminal.
    assert!(panel.set_viewport(9));
    assert_eq!(visible_texts(&panel).len(), 9);

    // The cursor is on the header, so a short terminal shows the beginning.
    assert!(panel.set_viewport(3));
    assert_eq!(
        visible_texts(&panel),
        vec![String::from(FIRST_HEADER), row(0), row(1)]
    );

    move_down(&mut panel, APPLICATIONS.len());
    assert!(panel.set_viewport(2));
    assert_eq!(visible_texts(&panel), vec![row(6), row(7)]);

    assert!(panel.set_viewport(9));
    assert_eq!(visible_texts(&panel).len(), 9);
    mock.assert_complete();
}

#[test]
fn keeps_the_selection_visible_after_a_refresh() {
    let mock = MockAdb::new();
    expect_load(&mock);
    // The refresh reads the list and every bucket once more.
    expect_load(&mock);

    let mut panel = panel_for(&mock, ANDROID_13);
    load_buckets(&mut panel);
    assert!(panel.set_viewport(4));

    move_down(&mut panel, 6);
    assert_eq!(focused_text(&panel), Some(row(5)));

    assert!(panel.refresh_inventory());
    wait_for(&mut panel, |_| {
        return mock.is_complete();
    });

    assert_eq!(focused_text(&panel), Some(row(5)));
    let visible = visible_texts(&panel);
    assert!(
        visible.contains(&row(5)),
        "the selected row is out of view: {visible:?}"
    );
    mock.assert_complete();
}

#[test]
fn renders_the_status_of_an_empty_section() {
    let mock = MockAdb::new();

    let mut panel = panel_for(&mock, ANDROID_13);
    assert!(panel.set_viewport(2));
    assert!(panel.handle_key(KeyCode::Char(' ')));

    // The section has no applications yet, and its status stays inside the
    // viewport right below the header.
    assert_eq!(
        visible_texts(&panel),
        vec![String::from(FIRST_HEADER), String::from("no applications")]
    );
    mock.assert_complete();
}
