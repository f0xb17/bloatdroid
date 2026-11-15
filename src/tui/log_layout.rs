//! The fixed columns of the Operations Log.
//!
//! Every entry starts its message in the same column: the timestamp column and
//! the log level column are padded to a fixed width, and the message takes the
//! remaining width of the panel. The layout lives here, so the panel only has
//! to draw what it is given and no message has to carry spaces of its own.

use crate::log::LogEntry;

/// The columns a timestamp occupies.
pub const TIMESTAMP_WIDTH: usize = 8;

/// The columns a log level label occupies, including its padding.
pub const LEVEL_WIDTH: usize = 9;

/// The blank columns between two columns.
pub const COLUMN_GAP: usize = 2;

/// The column that every message starts in.
pub const MESSAGE_COLUMN: usize =
    TIMESTAMP_WIDTH + COLUMN_GAP + LEVEL_WIDTH + COLUMN_GAP;

/// One rendered row of the Operations Log.
///
/// The prefix carries the fixed columns of the row, and the message carries the
/// text that belongs to the entry, so the terminal layer can style the two
/// separately without knowing how the columns were built.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct LogRow {
    prefix: String,
    message: String,
}

impl LogRow {
    /// Returns the whole row as one piece of text.
    pub fn text(&self) -> String {
        return format!("{}{}", self.prefix, self.message);
    }

    /// Returns the fixed columns of the row.
    pub fn prefix(&self) -> &str {
        return self.prefix.as_str();
    }

    /// Returns the message part of the row.
    pub fn message(&self) -> &str {
        return self.message.as_str();
    }
}

/// Lays one entry out as the rows it occupies at the given width.
///
/// The first row carries the timestamp and the level label, and every
/// continuation row is indented by the same number of columns, so a wrapped
/// message always starts underneath the message column. A row that is too
/// narrow for the message column shows the fixed columns alone, shortened to
/// the width that is left.
pub fn entry_rows(entry: &LogEntry, width: usize) -> Vec<LogRow> {
    let prefix_width = MESSAGE_COLUMN.min(width);
    let message_width = width.saturating_sub(prefix_width);

    let first = format!(
        "{}{}{}{}",
        pad(entry.timestamp(), TIMESTAMP_WIDTH),
        spaces(COLUMN_GAP),
        pad(entry.level().label(), LEVEL_WIDTH),
        spaces(COLUMN_GAP)
    );
    let first = take(&first, prefix_width);
    let indent = spaces(prefix_width);

    let mut rows = Vec::new();
    if message_width == 0 {
        let row = LogRow {
            prefix: first,
            message: String::new(),
        };
        rows.push(row);
        return rows;
    }

    let mut first_row = true;
    for message in wrap(entry.message(), message_width) {
        let prefix = if first_row {
            first.clone()
        } else {
            indent.clone()
        };
        rows.push(LogRow {
            prefix,
            message,
        });
        first_row = false;
    }
    return rows;
}

/// Returns `text` padded or shortened to exactly `width` characters.
fn pad(text: &str, width: usize) -> String {
    let mut padded = take(text, width);
    let padding = width.saturating_sub(char_count(&padded));
    for _ in 0..padding {
        padded.push(' ');
    }
    return padded;
}

/// Returns at most `width` characters of `text`.
fn take(text: &str, width: usize) -> String {
    let mut taken = String::with_capacity(width);
    for character in text.chars().take(width) {
        taken.push(character);
    }
    return taken;
}

/// Returns one space per column.
fn spaces(width: usize) -> String {
    let mut spaces = String::with_capacity(width);
    for _ in 0..width {
        spaces.push(' ');
    }
    return spaces;
}

/// Returns how many characters a text has.
fn char_count(text: &str) -> usize {
    let mut count = 0;
    for _ in text.chars() {
        count += 1;
    }
    return count;
}

/// Wraps a message into lines of at most `width` characters.
///
/// Words are kept whole where they fit, and a word that is longer than the
/// whole line is broken, so a long package name can never produce a row that
/// runs past the panel.
fn wrap(message: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();

    if width == 0 {
        lines.push(String::new());
        return lines;
    }

    for word in message.split_whitespace() {
        let mut chunk = String::new();
        for character in word.chars() {
            if char_count(&chunk) == width {
                append_word(&mut lines, &mut line, &chunk, width);
                chunk = String::new();
            }
            chunk.push(character);
        }
        if chunk.is_empty() {
            continue;
        }
        append_word(&mut lines, &mut line, &chunk, width);
    }

    if !line.is_empty() {
        lines.push(line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    return lines;
}

/// Adds one word to the current line, starting a new line when it does not fit.
fn append_word(
    lines: &mut Vec<String>,
    line: &mut String,
    word: &str,
    width: usize,
) {
    let separator = if line.is_empty() {
        0
    } else {
        1
    };
    let needed = char_count(line) + separator + char_count(word);
    if needed > width && !line.is_empty() {
        lines.push(std::mem::take(line));
    }
    if !line.is_empty() {
        line.push(' ');
    }
    line.push_str(word);
}
