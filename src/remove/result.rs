//! The structured results and errors of a removal.
//!
//! A removal is never reported from the exit status of ADB alone. A command can
//! succeed while the package is still installed, for example because the device
//! protects it, so the outcome keeps a package that is gone apart from one that
//! is still present, and every refusal and failure has its own reason.

use std::fmt;

use crate::adb::AdbError;

/// What a removal operation achieved.
#[derive(Debug)]
#[non_exhaustive]
pub enum RemovalOutcome {
    /// The package is no longer installed for the target user.
    Removed,
    /// The command was accepted, but the package is still installed.
    StillPresent,
    /// The device does not have the package installed.
    NotInstalled,
    /// The package was not removed, with the reason.
    NotRemoved(RemovalError),
}

impl RemovalOutcome {
    /// Returns `true` only when the device confirmed that the package is gone.
    ///
    /// A command that succeeded is not enough: [`RemovalOutcome::StillPresent`]
    /// describes exactly that case and is not a removal.
    pub fn is_removed(&self) -> bool {
        return matches!(self, RemovalOutcome::Removed);
    }
}

impl fmt::Display for RemovalOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RemovalOutcome::Removed => {
                return formatter.write_str("removed");
            }
            RemovalOutcome::StillPresent => {
                return formatter.write_str("still present");
            }
            RemovalOutcome::NotInstalled => {
                return formatter.write_str("not installed");
            }
            RemovalOutcome::NotRemoved(error) => {
                return write!(formatter, "not removed: {error}");
            }
        }
    }
}

/// Why a package was not removed.
#[derive(Debug)]
#[non_exhaustive]
pub enum RemovalError {
    /// The device refused the operation.
    PermissionDenied,
    /// The removal method is not available for the package or the device.
    UnsupportedMethod,
    /// ADB could not run the command or answered with a failure.
    Adb(AdbError),
    /// The removal could not be verified, with the reason.
    VerificationFailed(String),
    /// The operation failed for another reason.
    Failed(String),
}

impl fmt::Display for RemovalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RemovalError::PermissionDenied => {
                return formatter
                    .write_str("the device denied permission for the removal");
            }
            RemovalError::UnsupportedMethod => {
                return formatter
                    .write_str("the removal method is not supported");
            }
            RemovalError::Adb(error) => {
                return write!(formatter, "{error}");
            }
            RemovalError::VerificationFailed(message) => {
                return write!(
                    formatter,
                    "the removal could not be verified: {message}"
                );
            }
            RemovalError::Failed(message) => {
                return write!(formatter, "the removal failed: {message}");
            }
        }
    }
}

impl std::error::Error for RemovalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RemovalError::Adb(error) => return Some(error),
            _ => return None,
        }
    }
}
