//! Reading the inactive state of every installed application.

use crate::adb::AdbClient;
use crate::control::common::ControlEntry;
use crate::control::common::batch::query_packages;

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

/// Reads the inactive state of every package of the shared inventory.
///
/// An application whose state cannot be read is kept in the list with
/// [`UNKNOWN_STATE`] as its value and counted in
/// [`InactiveSnapshot::unreadable`], so a single failure does not hide the
/// rest of the list.
pub fn read_inactive(
    client: &AdbClient,
    serial: &str,
    packages: &[String],
) -> InactiveSnapshot {
    let answers = query_packages(packages, |package| {
        return get_inactive(client, serial, package);
    });

    let mut entries = Vec::new();
    let mut unreadable = 0;

    for (index, package) in packages.iter().enumerate() {
        let value = match answers.get(index) {
            Some(Ok(Some(state))) => String::from(state.name()),
            _ => {
                unreadable += 1;
                String::from(UNKNOWN_STATE)
            }
        };

        let entry = ControlEntry {
            identifier: package.clone(),
            value,
        };
        entries.push(entry);
    }

    let snapshot = InactiveSnapshot {
        entries,
        unreadable,
    };
    return snapshot;
}
