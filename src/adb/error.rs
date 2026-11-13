//! Errors produced by the ADB abstraction.

use std::fmt;
use std::io;
use std::process::ExitStatus;

/// A failure while locating, running, or reading the output of ADB.
#[derive(Debug)]
#[non_exhaustive]
pub enum AdbError {
    /// The ADB executable could not be started.
    Io(io::Error),
    /// The command ran but exited with a failure status.
    CommandFailed {
        /// The command that was run, as a readable label.
        command: String,
        /// The exit status the command returned.
        status: ExitStatus,
        /// The standard error output of the command, trimmed.
        stderr: String,
    },
    /// The command succeeded but its output was not valid UTF-8.
    OutputNotUtf8 {
        /// The command that was run, as a readable label.
        command: String,
    },
}

impl fmt::Display for AdbError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdbError::Io(error) => {
                return write!(formatter, "could not run adb: {error}");
            }
            AdbError::CommandFailed {
                command,
                status,
                stderr,
            } => {
                if stderr.is_empty() {
                    return write!(
                        formatter,
                        "`{command}` failed with {status}"
                    );
                }
                return write!(
                    formatter,
                    "`{command}` failed with {status}: {stderr}"
                );
            }
            AdbError::OutputNotUtf8 {
                command,
            } => {
                return write!(
                    formatter,
                    "`{command}` produced output that is not UTF-8"
                );
            }
        }
    }
}

impl std::error::Error for AdbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AdbError::Io(error) => {
                return Some(error);
            }
            AdbError::CommandFailed {
                ..
            } => {
                return None;
            }
            AdbError::OutputNotUtf8 {
                ..
            } => {
                return None;
            }
        }
    }
}

impl From<io::Error> for AdbError {
    fn from(error: io::Error) -> Self {
        return AdbError::Io(error);
    }
}
