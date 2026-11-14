//! Tests for the state shared by the control sections.

use bloatdroid::control::Section;
use bloatdroid::control::SectionState;

#[test]
fn a_section_starts_collapsed_and_empty() {
    let state = SectionState::new(Section::Buckets);

    assert_eq!(state.section(), Section::Buckets);
    assert!(!state.is_expanded());
    assert!(state.entries().is_empty());
    assert!(state.entry(0).is_none());
    assert!(state.status().is_none());
}

#[test]
fn expanding_and_collapsing_report_the_new_state() {
    let mut state = SectionState::new(Section::Doze);

    assert!(state.toggle_expanded());
    assert!(state.is_expanded());
    assert!(!state.toggle_expanded());
    assert!(!state.is_expanded());
}

#[test]
fn status_changes_are_reported_once() {
    let mut state = SectionState::new(Section::AppOps);

    assert!(state.set_status(Some(String::from("loading"))));
    assert_eq!(state.status(), Some("loading"));

    assert!(!state.set_status(Some(String::from("loading"))));
    assert_eq!(state.status(), Some("loading"));

    assert!(state.set_status(None));
    assert!(state.status().is_none());
}

#[test]
fn sections_report_their_titles() {
    assert_eq!(Section::Buckets.title(), "Standby Buckets");
    assert_eq!(Section::AppOps.title(), "AppOps");
    assert_eq!(Section::Inactive.title(), "App Inactive");
    assert_eq!(Section::Doze.title(), "Doze Whitelist");
}

#[test]
fn sections_are_distinct() {
    assert_ne!(Section::Buckets, Section::AppOps);
    assert_ne!(Section::Inactive, Section::Doze);
}
