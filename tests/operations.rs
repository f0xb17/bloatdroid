//! Tests for the centralized Operations Log and its panel.
//!
//! Nothing here needs a device or an ADB server: the log stamps its entries
//! from the system clock, and the panel layout is plain text.

use std::sync::Arc;
use std::thread;

use crossterm::event::KeyCode;

use bloatdroid::log::DEFAULT_CAPACITY;
use bloatdroid::log::LogEntry;
use bloatdroid::log::LogLevel;
use bloatdroid::log::OperationLog;
use bloatdroid::tui::LEVEL_WIDTH;
use bloatdroid::tui::MESSAGE_COLUMN;
use bloatdroid::tui::OperationsPanel;
use bloatdroid::tui::TIMESTAMP_WIDTH;
use bloatdroid::tui::entry_rows;

/// The timestamp every layout test uses.
const TIMESTAMP: &str = "12:41:03";

/// The width the layout tests lay their rows out in.
const WIDTH: usize = 60;

/// Returns how many characters a text has.
fn width_of(text: &str) -> usize {
    let mut width = 0;
    for _ in text.chars() {
        width += 1;
    }
    return width;
}

/// Returns the character at a position of a text.
fn character_at(text: &str, index: usize) -> Option<char> {
    let mut characters = text.chars();
    for _ in 0..index {
        characters.next()?;
    }
    return characters.next();
}

/// Returns `text` padded with spaces up to `width` characters.
fn padded(text: &str, width: usize) -> String {
    let mut padded = String::from(text);
    while width_of(&padded) < width {
        padded.push(' ');
    }
    return padded;
}

/// Builds an entry with a fixed timestamp.
fn entry(level: LogLevel, message: &str) -> LogEntry {
    return LogEntry::new(TIMESTAMP, level, message);
}

/// Returns the rendered rows of one entry.
fn row_texts(entry: &LogEntry, width: usize) -> Vec<String> {
    let mut texts = Vec::new();
    for row in entry_rows(entry, width) {
        texts.push(row.text());
    }
    return texts;
}

/// Returns the messages that are visible in the panel, oldest first.
fn visible_messages(panel: &OperationsPanel) -> Vec<String> {
    let mut messages = Vec::new();
    for (_level, row) in panel.visible() {
        messages.push(String::from(row.message()));
    }
    return messages;
}

/// Records `count` entries that are easy to tell apart.
fn fill(log: &OperationLog, count: usize) {
    for index in 0..count {
        log.record(LogLevel::Info, format!("entry {index}"));
    }
}

#[test]
fn records_an_entry_with_a_time_of_day() {
    let log = OperationLog::new();
    log.record(LogLevel::Info, "Loading the application inventory.");

    let entries = log.snapshot();
    assert_eq!(entries.len(), 1);
    let entry = match entries.first() {
        Some(entry) => entry,
        None => panic!("the entry disappeared"),
    };

    let timestamp = entry.timestamp();
    assert_eq!(width_of(timestamp), TIMESTAMP_WIDTH);
    assert_eq!(character_at(timestamp, 2), Some(':'));
    assert_eq!(character_at(timestamp, 5), Some(':'));
    for index in 0..TIMESTAMP_WIDTH {
        if index == 2 || index == 5 {
            continue;
        }
        match character_at(timestamp, index) {
            Some(character) => assert!(character.is_ascii_digit()),
            None => panic!("the timestamp is too short: {timestamp}"),
        }
    }

    assert_eq!(entry.level(), LogLevel::Info);
    assert_eq!(entry.message(), "Loading the application inventory.");
}

#[test]
fn labels_every_supported_level() {
    assert_eq!(LogLevel::Info.label(), "INFO");
    assert_eq!(LogLevel::Success.label(), "SUCCESS");
    assert_eq!(LogLevel::Warning.label(), "WARNING");
    assert_eq!(LogLevel::Error.label(), "ERROR");

    for level in
        [LogLevel::Info, LogLevel::Success, LogLevel::Warning, LogLevel::Error]
    {
        assert!(
            width_of(level.label()) <= LEVEL_WIDTH,
            "the label {} does not fit its column",
            level.label()
        );
    }
}

#[test]
fn keeps_the_newest_entries_and_drops_the_oldest() {
    let log = OperationLog::with_capacity(3);
    fill(&log, 5);

    assert_eq!(log.capacity(), 3);
    assert_eq!(log.len(), 3);

    let mut messages = Vec::new();
    for entry in log.snapshot() {
        messages.push(String::from(entry.message()));
    }
    assert_eq!(messages, vec!["entry 2", "entry 3", "entry 4"]);
}

#[test]
fn uses_the_default_capacity_and_never_zero() {
    assert_eq!(OperationLog::new().capacity(), DEFAULT_CAPACITY);
    assert_eq!(OperationLog::with_capacity(0).capacity(), 1);
}

#[test]
fn bumps_the_version_for_every_entry() {
    let log = OperationLog::new();
    let before = log.version();

    fill(&log, 2);

    assert_eq!(log.version(), before + 2);
}

#[test]
fn keeps_every_entry_of_concurrent_submissions() {
    let log = Arc::new(OperationLog::new());
    let workers = 4;
    let per_worker = 25;

    let mut handles = Vec::new();
    for worker in 0..workers {
        let log = Arc::clone(&log);
        let handle = thread::spawn(move || {
            for index in 0..per_worker {
                let message = format!("worker {worker} entry {index}");
                log.record(LogLevel::Info, message);
            }
        });
        handles.push(handle);
    }
    for handle in handles {
        match handle.join() {
            Ok(()) => {}
            Err(_) => panic!("a logging thread panicked"),
        }
    }

    let entries = log.snapshot();
    assert_eq!(entries.len(), workers * per_worker);
    for worker in 0..workers {
        for index in 0..per_worker {
            let expected = format!("worker {worker} entry {index}");
            let mut found = 0;
            for entry in &entries {
                if entry.message() == expected {
                    found += 1;
                }
            }
            assert_eq!(found, 1, "{expected} was logged {found} times");
        }
    }
}

#[test]
fn starts_every_message_in_the_same_column() {
    assert_eq!(MESSAGE_COLUMN, 21);

    for level in
        [LogLevel::Info, LogLevel::Success, LogLevel::Warning, LogLevel::Error]
    {
        let rows = entry_rows(&entry(level, "Loading."), WIDTH);
        let row = match rows.first() {
            Some(row) => row,
            None => panic!("the entry has no rows"),
        };

        let expected = format!(
            "{}  {}  Loading.",
            padded(TIMESTAMP, TIMESTAMP_WIDTH),
            padded(level.label(), LEVEL_WIDTH)
        );
        assert_eq!(row.text(), expected);
        assert_eq!(width_of(row.prefix()), MESSAGE_COLUMN);
    }
}

#[test]
fn wraps_a_long_message_inside_the_message_column() {
    let message = "Failed to change the App Inactive state because the \
                   Android command returned an error.";
    let rows = entry_rows(&entry(LogLevel::Error, message), 40);

    assert!(rows.len() > 1, "the message did not wrap: {rows:?}");
    for row in &rows {
        assert!(
            width_of(&row.text()) <= 40,
            "a row is wider than the panel: {}",
            row.text()
        );
    }

    let mut joined = String::new();
    for row in &rows {
        if !joined.is_empty() {
            joined.push(' ');
        }
        joined.push_str(row.message());
    }
    assert_eq!(joined, message);
}

#[test]
fn indents_continuation_lines_under_the_message_column() {
    let message = "Failed to change the App Inactive state because the \
                   Android command returned an error.";
    let rows = entry_rows(&entry(LogLevel::Error, message), 40);

    let mut first = true;
    for row in &rows {
        if first {
            first = false;
            continue;
        }
        assert_eq!(width_of(row.prefix()), MESSAGE_COLUMN);
        assert!(row.prefix().trim().is_empty());
        assert!(!row.message().is_empty());
    }
}

#[test]
fn keeps_long_words_inside_a_narrow_panel() {
    for width in 1..=MESSAGE_COLUMN + 4 {
        let message = "com.example.a.very.long.application.name";
        let rows = entry_rows(&entry(LogLevel::Warning, message), width);
        for row in &rows {
            assert!(
                width_of(&row.text()) <= width,
                "a row of width {width} is too wide: {}",
                row.text()
            );
        }
    }
}

#[test]
fn shows_the_fixed_columns_when_the_message_column_does_not_fit() {
    let rows = entry_rows(&entry(LogLevel::Info, "Loading."), 12);

    assert_eq!(rows.len(), 1);
    let row = match rows.first() {
        Some(row) => row,
        None => panic!("the entry has no rows"),
    };
    assert_eq!(row.message(), "");
    assert_eq!(row.prefix(), "12:41:03  IN");
    assert_eq!(width_of(&row.text()), 12);
}

#[test]
fn survives_every_narrow_width() {
    for width in 0..=MESSAGE_COLUMN {
        let rows = row_texts(&entry(LogLevel::Info, "Loading."), width);
        assert!(!rows.is_empty(), "no row at width {width}");
        for text in &rows {
            assert!(
                width_of(text) <= width,
                "too wide at width {width}: {text}"
            );
        }
    }
}

#[test]
fn follows_the_newest_entries() {
    let log = Arc::new(OperationLog::new());
    let mut panel = OperationsPanel::new(Arc::clone(&log));
    panel.set_viewport(WIDTH, 2);

    fill(&log, 3);
    assert!(panel.poll());

    assert!(panel.is_following());
    assert_eq!(visible_messages(&panel), vec!["entry 1", "entry 2"]);
}

#[test]
fn keeps_the_scroll_position_while_older_entries_are_read() {
    let log = Arc::new(OperationLog::new());
    let mut panel = OperationsPanel::new(Arc::clone(&log));
    panel.set_viewport(WIDTH, 2);

    fill(&log, 4);
    panel.poll();
    assert_eq!(visible_messages(&panel), vec!["entry 2", "entry 3"]);

    assert!(panel.handle_key(KeyCode::Up));
    assert!(!panel.is_following());
    assert_eq!(visible_messages(&panel), vec!["entry 1", "entry 2"]);

    log.record(LogLevel::Info, "entry 4");
    assert!(!panel.poll(), "the view should not scroll by itself");
    assert_eq!(visible_messages(&panel), vec!["entry 1", "entry 2"]);
}

#[test]
fn follows_again_at_the_newest_entry() {
    let log = Arc::new(OperationLog::new());
    let mut panel = OperationsPanel::new(Arc::clone(&log));
    panel.set_viewport(WIDTH, 2);

    fill(&log, 4);
    panel.poll();
    assert!(panel.handle_key(KeyCode::Up));
    assert!(!panel.is_following());

    assert!(panel.handle_key(KeyCode::Down));
    assert!(panel.is_following());
    assert_eq!(visible_messages(&panel), vec!["entry 2", "entry 3"]);

    log.record(LogLevel::Info, "entry 4");
    assert!(panel.poll());
    assert_eq!(visible_messages(&panel), vec!["entry 3", "entry 4"]);
}

#[test]
fn redraws_when_the_log_drops_its_oldest_entry() {
    let log = Arc::new(OperationLog::with_capacity(3));
    let mut panel = OperationsPanel::new(Arc::clone(&log));
    panel.set_viewport(WIDTH, 2);

    fill(&log, 3);
    panel.poll();
    assert!(panel.handle_key(KeyCode::Up));

    log.record(LogLevel::Info, "entry 3");
    assert!(panel.poll(), "the evicted entry changed the visible rows");
}

#[test]
fn scrolls_nothing_when_there_is_no_room() {
    let log = Arc::new(OperationLog::new());
    let mut panel = OperationsPanel::new(Arc::clone(&log));
    panel.set_viewport(WIDTH, 0);

    fill(&log, 3);
    panel.poll();

    assert!(!panel.handle_key(KeyCode::Up));
    assert!(!panel.handle_key(KeyCode::Down));
    assert!(panel.visible().is_empty());
}
