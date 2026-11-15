//! Two-column rows of an application list.
//!
//! Every control area lists applications the same way: the package name on the
//! left and the state of the area on the right, with the state label against
//! the right edge of the row. The layout lives here, so all four areas share
//! one implementation and one set of truncation rules.

/// The least number of blank columns between the name and the state label.
const COLUMN_GAP: usize = 2;

/// The character that marks a text that had to be shortened.
const ELLIPSIS: char = '…';

/// Formats one application row into two columns that are `width` characters
/// wide.
///
/// The name is left-aligned in the first column and the state label is
/// right-aligned against the last one, so the label of every row ends in the
/// same column no matter how long the package name is. A name that does not fit
/// into its column is shortened and ends in an ellipsis, so it can never run
/// into the label or push it out of place.
///
/// A row that is too narrow for both columns shows the state label alone,
/// right-aligned like in a full row, because that is the part the user acts on.
/// The label itself is shortened only when it does not fit even on its own.
pub fn application_row(name: &str, state: &str, width: usize) -> String {
    let label = shorten(state, width);
    let name_width = width.saturating_sub(char_count(state) + COLUMN_GAP);

    let mut row = String::with_capacity(width);
    if name_width > 0 {
        row.push_str(&shorten(name, name_width));
    }

    let padding = width.saturating_sub(char_count(&row) + char_count(&label));
    for _ in 0..padding {
        row.push(' ');
    }
    row.push_str(&label);
    return row;
}

/// Returns `text` shortened to at most `width` characters.
///
/// A shortened text ends in an ellipsis, so the cut is visible. A text that
/// already fits is returned unchanged.
fn shorten(text: &str, width: usize) -> String {
    if char_count(text) <= width {
        return String::from(text);
    }
    if width == 0 {
        return String::new();
    }

    let mut shortened = String::with_capacity(width);
    for character in text.chars().take(width - 1) {
        shortened.push(character);
    }
    shortened.push(ELLIPSIS);
    return shortened;
}

/// Returns how many characters a text has.
fn char_count(text: &str) -> usize {
    let mut count = 0;
    for _ in text.chars() {
        count += 1;
    }
    return count;
}
