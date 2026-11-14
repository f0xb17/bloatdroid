//! Reading the AppOps mode of every installed application.

use crate::adb::AdbClient;
use crate::control::common::ControlEntry;
use crate::control::common::batch::query_packages;

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

/// Reads the mode of every package of the shared inventory.
///
/// An application whose mode cannot be read is kept in the list with
/// [`UNKNOWN_MODE`] as its value and counted in
/// [`AppOpsSnapshot::unreadable`], so a single failure does not hide the rest
/// of the list.
pub fn read_appops(
    client: &AdbClient,
    serial: &str,
    packages: &[String],
) -> AppOpsSnapshot {
    let answers = query_packages(packages, |package| {
        return get_mode(client, serial, package);
    });

    let mut entries = Vec::new();
    let mut unreadable = 0;

    for (index, package) in packages.iter().enumerate() {
        let value = match answers.get(index) {
            Some(Ok(Some(mode))) => String::from(mode.name()),
            _ => {
                unreadable += 1;
                String::from(UNKNOWN_MODE)
            }
        };

        let entry = ControlEntry {
            identifier: package.clone(),
            value,
        };
        entries.push(entry);
    }

    let snapshot = AppOpsSnapshot {
        entries,
        unreadable,
    };
    return snapshot;
}
