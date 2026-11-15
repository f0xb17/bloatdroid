//! The Android user that a removal targets.
//!
//! A user-level uninstall removes an application for one Android user, so the
//! active user is always read from the device instead of being assumed. The
//! command-line tools accept the identifier as a number, and the value is only
//! built from a numeric answer, so no uninstall is ever aimed at a user the
//! device did not name.

use crate::adb::AdbClient;
use crate::discover::AndroidDevice;

use super::commands::current_user_command;
use super::result::RemovalError;

/// One Android user of a device.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct AndroidUser {
    id: u32,
}

impl AndroidUser {
    /// Builds the user with the given identifier.
    pub fn new(id: u32) -> Self {
        let user = Self {
            id,
        };
        return user;
    }

    /// Returns the identifier that Android uses, for example `0`.
    pub fn id(self) -> u32 {
        return self.id;
    }

    /// Returns the identifier in the form the Package Manager takes it.
    pub fn argument(self) -> String {
        return self.id.to_string();
    }
}

/// Reads the user that is currently active on the device.
///
/// The Package Manager only accepts a user identifier, so the device is asked
/// for the active one before anything is uninstalled.
pub fn current_user(
    client: &AdbClient,
    device: &AndroidDevice,
) -> Result<AndroidUser, RemovalError> {
    let command = current_user_command(device);
    let arguments = command.as_adb_arguments();

    let output = match client.run(&arguments) {
        Ok(output) => output,
        Err(error) => return Err(RemovalError::Adb(error)),
    };
    return parse_current_user(output.as_str());
}

/// Parses the answer of `am get-current-user`.
///
/// The command prints the numeric identifier of the active user. Missing or
/// non-numeric output is rejected rather than guessed.
fn parse_current_user(output: &str) -> Result<AndroidUser, RemovalError> {
    let text = output.trim();
    if !is_numeric(text) {
        return Err(RemovalError::InvalidUser {
            output: String::from(text),
        });
    }

    let id = match text.parse::<u32>() {
        Ok(id) => id,
        Err(_) => {
            return Err(RemovalError::InvalidUser {
                output: String::from(text),
            });
        }
    };
    return Ok(AndroidUser::new(id));
}

/// Reports whether the text is a non-empty sequence of digits.
fn is_numeric(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }

    for character in text.chars() {
        if !character.is_ascii_digit() {
            return false;
        }
    }
    return true;
}
