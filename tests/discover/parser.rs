//! Tests for the device property helpers.

use bloatdroid::discover::parser::normalize;

#[test]
fn normalize_trims_surrounding_whitespace() {
    assert_eq!(normalize("  Pixel 4 \n"), "Pixel 4");
}

#[test]
fn normalize_keeps_inner_whitespace() {
    assert_eq!(normalize(" Pixel  4 "), "Pixel  4");
}

#[test]
fn normalize_keeps_an_empty_value_empty() {
    assert_eq!(normalize(" \n\t"), "");
}
