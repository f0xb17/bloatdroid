//! Tests for the sections shown in the Standby Crates panel.
//!
//! These tests exercise state and key handling only; terminal rendering is
//! never involved.

use crossterm::event::KeyCode;

use crate::common::MockAdb;

use super::harness::ANDROID_13;
use super::harness::entry_texts;
use super::harness::focused_text;
use super::harness::header_expansion;
use super::harness::header_texts;
use super::harness::overlay_is_focused;
use super::harness::panel_for;
use super::harness::status_texts;

#[test]
fn lists_the_four_sections_in_order() {
    let mock = MockAdb::new();
    let panel = panel_for(&mock, ANDROID_13);

    let expected = vec![
        String::from("Standby Buckets"),
        String::from("AppOps"),
        String::from("App Inactive"),
        String::from("Doze Whitelist"),
    ];
    assert_eq!(header_texts(&panel), expected);
    mock.assert_complete();
}

#[test]
fn starts_with_every_section_collapsed() {
    let mock = MockAdb::new();
    let panel = panel_for(&mock, ANDROID_13);

    assert_eq!(header_expansion(&panel), vec![false, false, false, false]);
    assert!(entry_texts(&panel).is_empty());
    assert!(status_texts(&panel).is_empty());
    mock.assert_complete();
}

#[test]
fn expands_and_collapses_the_focused_section() {
    let mock = MockAdb::new();
    let mut panel = panel_for(&mock, ANDROID_13);
    assert!(panel.handle_key(KeyCode::Down));

    assert!(panel.handle_key(KeyCode::Char(' ')));
    assert_eq!(header_expansion(&panel), vec![false, true, false, false]);
    assert_eq!(status_texts(&panel), vec![String::from("no applications")]);

    assert!(panel.handle_key(KeyCode::Char(' ')));
    assert_eq!(header_expansion(&panel), vec![false, false, false, false]);
    assert!(status_texts(&panel).is_empty());
    mock.assert_complete();
}

#[test]
fn keeps_the_expansion_of_each_section_separately() {
    let mock = MockAdb::new();
    let mut panel = panel_for(&mock, ANDROID_13);

    assert!(panel.handle_key(KeyCode::Down));
    assert!(panel.handle_key(KeyCode::Char(' ')));
    assert!(panel.handle_key(KeyCode::Down));
    assert!(panel.handle_key(KeyCode::Char(' ')));

    assert_eq!(header_expansion(&panel), vec![false, true, true, false]);
    mock.assert_complete();
}

#[test]
fn expanding_keeps_the_cursor_on_the_header() {
    let mock = MockAdb::new();
    let mut panel = panel_for(&mock, ANDROID_13);
    assert!(panel.handle_key(KeyCode::Down));

    assert!(panel.handle_key(KeyCode::Char(' ')));

    assert_eq!(focused_text(&panel), Some(String::from("AppOps")));
    mock.assert_complete();
}

#[test]
fn moves_the_cursor_between_the_sections() {
    let mock = MockAdb::new();
    let mut panel = panel_for(&mock, ANDROID_13);
    assert_eq!(focused_text(&panel), Some(String::from("Standby Buckets")));

    assert!(panel.handle_key(KeyCode::Down));
    assert_eq!(focused_text(&panel), Some(String::from("AppOps")));

    assert!(panel.handle_key(KeyCode::Down));
    assert_eq!(focused_text(&panel), Some(String::from("App Inactive")));

    assert!(panel.handle_key(KeyCode::Down));
    assert_eq!(focused_text(&panel), Some(String::from("Doze Whitelist")));

    assert!(panel.handle_key(KeyCode::Up));
    assert_eq!(focused_text(&panel), Some(String::from("App Inactive")));
    mock.assert_complete();
}

#[test]
fn stops_at_the_first_and_last_row() {
    let mock = MockAdb::new();
    let mut panel = panel_for(&mock, ANDROID_13);

    assert!(!panel.handle_key(KeyCode::Up));
    assert_eq!(focused_text(&panel), Some(String::from("Standby Buckets")));

    for _ in 0..3 {
        assert!(panel.handle_key(KeyCode::Down));
    }
    assert_eq!(focused_text(&panel), Some(String::from("Doze Whitelist")));

    // The control strip below the sections is the last row of the panel.
    assert!(panel.handle_key(KeyCode::Down));
    assert!(overlay_is_focused(&panel));
    assert!(!panel.handle_key(KeyCode::Down));
    mock.assert_complete();
}

#[test]
fn ignores_keys_that_no_section_handles() {
    let mock = MockAdb::new();
    let mut panel = panel_for(&mock, ANDROID_13);

    assert!(!panel.handle_key(KeyCode::Char('x')));
    assert!(!panel.handle_key(KeyCode::Left));

    assert_eq!(focused_text(&panel), Some(String::from("Standby Buckets")));
    mock.assert_complete();
}
