//! Reading the AppOps mode of every installed application.

use crate::adb::AdbClient;
use crate::adb::AdbError;
use crate::adb::list_packages;
use crate::control::common::ControlEntry;

use super::commands::get_mode;

/// The value shown for an application whose mode could not be read.
pub const UNKNOWN_MODE: &str = "unknown";

/// The applications of one device and their AppOps modes.
#[non_exhaustive]
pub struct AppOpsSnapshot {
    /// One entry per application, sorted by package identifier.
    pub entries: Vec<ControlEntry>,
    /// How many applications could not be read.
    pub unreadable: usize,
}

/// Reads the mode of every installed application.
///
/// An application whose mode cannot be read is kept in the list with
/// [`UNKNOWN_MODE`] as its value and counted in
/// [`AppOpsSnapshot::unreadable`], so a single failure does not hide the rest
/// of the list. A failure to list the packages is returned as an error.
pub fn read_appops(
    client: &AdbClient,
    serial: &str,
) -> Result<AppOpsSnapshot, AdbError> {
    let packages = list_packages(client, serial)?;

    let mut entries = Vec::new();
    let mut unreadable = 0;

    for package in packages {
        let value = match get_mode(client, serial, &package) {
            Ok(Some(mode)) => String::from(mode.name()),
            _ => {
                unreadable += 1;
                String::from(UNKNOWN_MODE)
            }
        };

        let entry = ControlEntry {
            identifier: package,
            value,
        };
        entries.push(entry);
    }

    let snapshot = AppOpsSnapshot {
        entries,
        unreadable,
    };
    return Ok(snapshot);
}
