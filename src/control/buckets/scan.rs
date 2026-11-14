//! Reading the standby bucket of every installed application.

use crate::adb::AdbClient;
use crate::adb::AdbError;
use crate::adb::list_packages;
use crate::control::common::ControlEntry;

use super::commands::get_standby_bucket;

/// The value shown for an application whose bucket could not be read.
pub const UNKNOWN_BUCKET: &str = "unknown";

/// The applications of one device and their standby buckets.
#[non_exhaustive]
pub struct BucketSnapshot {
    /// One entry per application, sorted by package identifier.
    pub entries: Vec<ControlEntry>,
    /// How many applications could not be read.
    pub unreadable: usize,
}

/// Reads the standby bucket of every installed application.
///
/// An application whose bucket cannot be read is kept in the list with
/// [`UNKNOWN_BUCKET`] as its value and counted in
/// [`BucketSnapshot::unreadable`], so a single failure does not hide the rest
/// of the list. A failure to list the packages is returned as an error.
pub fn read_buckets(
    client: &AdbClient,
    serial: &str,
) -> Result<BucketSnapshot, AdbError> {
    let packages = list_packages(client, serial)?;

    let mut entries = Vec::new();
    let mut unreadable = 0;

    for package in packages {
        let bucket = match get_standby_bucket(client, serial, &package) {
            Ok(bucket) if !bucket.is_empty() => bucket,
            _ => {
                unreadable += 1;
                String::from(UNKNOWN_BUCKET)
            }
        };

        let entry = ControlEntry {
            identifier: package,
            value: bucket,
        };
        entries.push(entry);
    }

    let snapshot = BucketSnapshot {
        entries,
        unreadable,
    };
    return Ok(snapshot);
}
