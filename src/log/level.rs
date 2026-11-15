//! The levels of the Operations Log.

/// The severity of one Operations Log entry.
///
/// The labels are fixed and short enough for the log level column of the
/// Operations panel: a longer label would move the message column, so the
/// terminal layer reserves room for the longest one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum LogLevel {
    /// Something was started, or an event worth reporting happened.
    Info,
    /// An operation finished and its result was verified.
    Success,
    /// Something did not work as expected while the operation went on.
    Warning,
    /// An operation failed.
    Error,
}

impl LogLevel {
    /// Returns the label that the log level column shows.
    pub fn label(self) -> &'static str {
        let label = match self {
            LogLevel::Info => "INFO",
            LogLevel::Success => "SUCCESS",
            LogLevel::Warning => "WARNING",
            LogLevel::Error => "ERROR",
        };
        return label;
    }
}
