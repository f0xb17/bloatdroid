//! Validation of removal requests.
//!
//! Only a [`RemovalRequest`] reaches the execution layer, and the only way to
//! obtain one is [`validate`]. The layer checks that a device is selected and
//! usable and that the package identifier follows Android's package name rules.
//! Everything else about a method, such as its permissions or whether the
//! package supports it, is established when the method runs.

use std::fmt;

use crate::discover::AndroidDevice;

use super::commands::RemovalMethod;
use super::package::RemovalPackage;

/// The ADB state of a device that can run commands.
const ONLINE_STATE: &str = "device";

/// A removal request that passed validation.
///
/// The request binds one device, one package, and one removal method. A value of
/// this type cannot be built outside [`validate`], so the execution layer can
/// rely on a request naming an online device and a well-formed package
/// identifier.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct RemovalRequest {
    device: AndroidDevice,
    package: RemovalPackage,
    method: RemovalMethod,
}

impl RemovalRequest {
    /// Returns the device the removal targets.
    pub fn device(&self) -> &AndroidDevice {
        return &self.device;
    }

    /// Returns the package the removal targets.
    pub fn package(&self) -> &RemovalPackage {
        return &self.package;
    }

    /// Returns the method that removes the package.
    pub fn method(&self) -> RemovalMethod {
        return self.method;
    }
}

/// Why a removal request was rejected.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ValidationError {
    /// No Android device is selected.
    NoDevice,
    /// The selected device cannot run commands.
    UnusableDevice {
        /// The ADB state the device reports, for example `offline`.
        state: String,
    },
    /// The request names no package.
    EmptyPackage,
    /// The package identifier is not a valid Android package name.
    InvalidPackage {
        /// The rejected identifier.
        identifier: String,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::NoDevice => {
                return formatter.write_str("no Android device is selected");
            }
            ValidationError::UnusableDevice {
                state,
            } => {
                return write!(
                    formatter,
                    "the Android device cannot run commands (state `{state}`)"
                );
            }
            ValidationError::EmptyPackage => {
                return formatter.write_str("the package identifier is empty");
            }
            ValidationError::InvalidPackage {
                identifier,
            } => {
                return write!(
                    formatter,
                    "`{identifier}` is not a valid Android package name"
                );
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// Validates a removal request and returns it when it can run.
///
/// The device has to be selected and online, and the package identifier has to
/// follow Android's package name rules. The method is carried into the request;
/// its own requirements, such as permissions and whether the package supports
/// it, are answered by the device when it runs.
pub fn validate(
    device: Option<&AndroidDevice>,
    package: RemovalPackage,
    method: RemovalMethod,
) -> Result<RemovalRequest, ValidationError> {
    let device = match device {
        Some(device) => device,
        None => return Err(ValidationError::NoDevice),
    };
    if device.state != ONLINE_STATE {
        let state = device.state.clone();
        return Err(ValidationError::UnusableDevice {
            state,
        });
    }
    if package.identifier().is_empty() {
        return Err(ValidationError::EmptyPackage);
    }
    if !is_package_identifier(package.identifier()) {
        let identifier = String::from(package.identifier());
        return Err(ValidationError::InvalidPackage {
            identifier,
        });
    }

    let request = RemovalRequest {
        device: device.clone(),
        package,
        method,
    };
    return Ok(request);
}

/// Reports whether an identifier follows Android's package name rules.
///
/// Every dot-separated segment starts with an ASCII letter and continues with
/// ASCII letters, digits, or underscores, which is the rule Android applies to
/// package names. A single segment without a dot is accepted, and no length or
/// naming convention beyond that format is imposed.
fn is_package_identifier(identifier: &str) -> bool {
    let mut segment_start = true;

    for character in identifier.chars() {
        if character == '.' {
            if segment_start {
                return false;
            }
            segment_start = true;
            continue;
        }
        if segment_start {
            if !character.is_ascii_alphabetic() {
                return false;
            }
            segment_start = false;
            continue;
        }
        if character.is_ascii_alphanumeric() || character == '_' {
            continue;
        }
        return false;
    }

    return !segment_start;
}
