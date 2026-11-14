//! Combining installed applications with the Device Idle whitelist.

use std::fmt;

use crate::adb::AdbClient;
use crate::adb::AdbError;
use crate::control::common::ControlEntry;

use super::commands::get_whitelist;
use super::parser::WhitelistError;
use super::parser::parse_whitelist;

/// Why the whitelist state of the installed applications could not be read.
#[derive(Debug)]
#[non_exhaustive]
pub enum DozeError {
    /// The whitelist could not be read from the device.
    Whitelist(AdbError),
    /// The whitelist output could not be understood.
    Parse(WhitelistError),
}

impl fmt::Display for DozeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DozeError::Whitelist(error) => {
                return write!(
                    formatter,
                    "could not read the whitelist: {error}"
                );
            }
            DozeError::Parse(error) => {
                return write!(
                    formatter,
                    "could not read the whitelist: {error}"
                );
            }
        }
    }
}

impl std::error::Error for DozeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DozeError::Whitelist(error) => {
                return Some(error);
            }
            DozeError::Parse(error) => {
                return Some(error);
            }
        }
    }
}

/// Reads the whitelist state of every package of the shared inventory.
///
/// The complete whitelist is read once per scan and every package is
/// classified from it, so an application that neither whitelist holds is not
/// whitelisted. A whitelist that cannot be read or understood is an error
/// rather than an empty one: no application can be classified safely without
/// it, and reporting every application as not whitelisted would invite changes
/// the device does not need.
pub fn read_doze(
    client: &AdbClient,
    serial: &str,
    packages: &[String],
) -> Result<Vec<ControlEntry>, DozeError> {
    let output = match get_whitelist(client, serial) {
        Ok(output) => output,
        Err(error) => return Err(DozeError::Whitelist(error)),
    };
    let whitelist = match parse_whitelist(&output) {
        Ok(whitelist) => whitelist,
        Err(error) => return Err(DozeError::Parse(error)),
    };

    let mut entries = Vec::new();
    for package in packages {
        let state = whitelist.state_of(package);
        let entry = ControlEntry {
            identifier: package.clone(),
            value: String::from(state.name()),
        };
        entries.push(entry);
    }
    return Ok(entries);
}
