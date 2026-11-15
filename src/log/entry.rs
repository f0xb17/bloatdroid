//! One entry of the Operations Log.

use super::level::LogLevel;

/// One timestamped message of the Operations Log.
///
/// The timestamp is a plain time of day: the log keeps it as text, so an entry
/// can be built from a recorded clock and stays reproducible, and the terminal
/// layer decides how much of it fits.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct LogEntry {
    timestamp: String,
    level: LogLevel,
    message: String,
}

impl LogEntry {
    /// Builds an entry from a timestamp, a level, and a message.
    pub fn new(
        timestamp: impl Into<String>,
        level: LogLevel,
        message: impl Into<String>,
    ) -> Self {
        let entry = Self {
            timestamp: timestamp.into(),
            level,
            message: message.into(),
        };
        return entry;
    }

    /// Returns the time of day the entry was recorded at, as `HH:MM:SS`.
    pub fn timestamp(&self) -> &str {
        return self.timestamp.as_str();
    }

    /// Returns the level of the entry.
    pub fn level(&self) -> LogLevel {
        return self.level;
    }

    /// Returns the message of the entry.
    pub fn message(&self) -> &str {
        return self.message.as_str();
    }
}
