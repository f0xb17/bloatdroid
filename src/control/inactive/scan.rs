//! Reading the inactive state of every installed application.

use crate::adb::AdbClient;
use crate::adb::AdbError;
use crate::adb::list_packages;
use crate::control::common::ControlEntry;

use super::commands::get_inactive;

/// The value shown for an application whose state could not be read.
pub const UNKNOWN_STATE: &str = "unknown";

/// The applications of one device and their inactive states.
#[non_exhaustive]
pub struct InactiveSnapshot {
    /// One entry per application, sorted by package identifier.
    pub entries: Vec<ControlEntry>,
    /// How many applications could not be read.
    pub unreadable: usize,
}

/// Reads the inactive state of every installed application.
///
/// An application whose state cannot be read is kept in the list with
/// [`UNKNOWN_STATE`] as its value and counted in
/// [`InactiveSnapshot::unreadable`], so a single failure does not hide the
/// rest of the list. A failure to list the packages is returned as an error.
pub fn read_inactive(
    client: &AdbClient,
    serial: &str,
) -> Result<InactiveSnapshot, AdbError> {
    let packages = list_packages(client, serial)?;

    let mut entries = Vec::new();
    let mut unreadable = 0;

    for package in packages {
        let value = match get_inactive(client, serial, &package) {
            Ok(Some(state)) => String::from(state.name()),
            _ => {
                unreadable += 1;
                String::from(UNKNOWN_STATE)
            }
        };

        let entry = ControlEntry {
            identifier: package,
            value,
        };
        entries.push(entry);
    }

    let snapshot = InactiveSnapshot {
        entries,
        unreadable,
    };
    return Ok(snapshot);
}
