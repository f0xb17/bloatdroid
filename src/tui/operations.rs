//! The Operations panel: the log entries, their styling, and scrolling.
//!
//! The panel owns how the log is presented and how the user moves through it.
//! It never records an entry itself and never touches ADB: modules report to
//! the log, and the panel only reads what the log already holds.

use std::sync::Arc;

use crossterm::event::KeyCode;
use ratatui::style::Color;
use ratatui::style::Style;

use crate::log::LogLevel;
use crate::log::OperationLog;

use super::log_layout::LogRow;
use super::log_layout::entry_rows;

/// Returns the style of one log level.
///
/// The colours are defined here once, so every level looks the same wherever
/// the panel draws it.
pub fn level_style(level: LogLevel) -> Style {
    let style = match level {
        LogLevel::Info => Style::new(),
        LogLevel::Success => Style::new().fg(Color::Green),
        LogLevel::Warning => Style::new().fg(Color::Yellow),
        LogLevel::Error => Style::new().fg(Color::Red),
    };
    return style;
}

/// The Operations panel.
///
/// The log itself stays where the modules report to it; the panel only keeps
/// how far back the user scrolled and whether the newest entry stays in view.
#[non_exhaustive]
pub struct OperationsPanel {
    log: Arc<OperationLog>,
    offset: usize,
    follow: bool,
    width: usize,
    height: usize,
    rendered_version: u64,
}

impl OperationsPanel {
    /// Creates a panel that shows the log and follows its newest entries.
    pub fn new(log: Arc<OperationLog>) -> Self {
        let rendered_version = log.version();
        let panel = Self {
            log,
            offset: 0,
            follow: true,
            width: 0,
            height: 0,
            rendered_version,
        };
        return panel;
    }

    /// Returns `true` while the panel keeps the newest entry in view.
    pub fn is_following(&self) -> bool {
        return self.follow;
    }

    /// Handles a key press while the panel has focus.
    ///
    /// The arrows scroll one row at a time, like the cursor of the other
    /// panels moves one row at a time. Returns `true` when the view changed.
    pub fn handle_key(&mut self, key: KeyCode) -> bool {
        let moved = match key {
            KeyCode::Up => self.scroll_up(),
            KeyCode::Down => self.scroll_down(),
            _ => false,
        };
        return moved;
    }

    /// Tells the panel how much room the log has.
    ///
    /// The renderer calls this with the size of the area it drew the panel in.
    /// Returns `true` when the visible rows changed.
    pub fn set_viewport(&mut self, width: usize, height: usize) -> bool {
        if self.width == width && self.height == height {
            return false;
        }

        self.width = width;
        self.height = height;
        let previous = self.offset;
        let maximum = self.maximum_offset();
        if self.follow || self.offset > maximum {
            self.offset = maximum;
        }
        return previous != self.offset;
    }

    /// Applies the entries that were recorded since the last call.
    ///
    /// Returns `true` when the visible rows changed. A new entry always changes
    /// the view while the panel follows the log, but it leaves the view alone
    /// while the user reads older entries, unless the log dropped its oldest
    /// ones to stay within its capacity.
    pub fn poll(&mut self) -> bool {
        let version = self.log.version();
        if version == self.rendered_version {
            return false;
        }
        self.rendered_version = version;

        let maximum = self.maximum_offset();
        if self.follow {
            self.offset = maximum;
            return true;
        }

        if self.offset > maximum {
            self.offset = maximum;
            self.follow = true;
            return true;
        }
        return self.log.len() == self.log.capacity();
    }

    /// Returns the rows that are visible, oldest first.
    pub fn visible(&self) -> Vec<(LogLevel, LogRow)> {
        let rows = self.all_rows();

        let mut visible = Vec::new();
        for (index, row) in rows.into_iter().enumerate() {
            if index < self.offset {
                continue;
            }
            if visible.len() >= self.height {
                break;
            }
            visible.push(row);
        }
        return visible;
    }

    /// Scrolls the view one row towards older entries.
    fn scroll_up(&mut self) -> bool {
        if self.height == 0 {
            return false;
        }

        let previous = self.offset;
        let was_following = self.follow;
        self.offset = self.offset.saturating_sub(1);
        self.follow = false;
        return previous != self.offset || was_following;
    }

    /// Scrolls the view one row towards the newest entry.
    ///
    /// Reaching the newest row makes the panel follow the log again.
    fn scroll_down(&mut self) -> bool {
        if self.height == 0 {
            return false;
        }

        let maximum = self.maximum_offset();
        let previous = self.offset;
        let was_following = self.follow;
        if self.offset < maximum {
            self.offset = self.offset.saturating_add(1);
        }
        self.follow = self.offset >= maximum;
        return previous != self.offset || was_following != self.follow;
    }

    /// Returns how many rows the whole log occupies at the current width.
    fn all_rows(&self) -> Vec<(LogLevel, LogRow)> {
        let entries = self.log.snapshot();

        let mut rows = Vec::new();
        for entry in entries {
            let level = entry.level();
            for row in entry_rows(&entry, self.width) {
                rows.push((level, row));
            }
        }
        return rows;
    }

    /// Returns the first row that still leaves the view filled.
    fn maximum_offset(&self) -> usize {
        return self.all_rows().len().saturating_sub(self.height);
    }
}
