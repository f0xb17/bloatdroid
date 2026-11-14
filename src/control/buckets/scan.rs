//! Reading the standby bucket of every application.

use crate::adb::AdbClient;
use crate::control::common::ControlEntry;
use crate::control::common::batch::query_packages;

use super::bucket::StandbyBucket;
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

/// Reads the standby bucket of every package of the shared inventory.
///
/// A device reports a bucket either by name or by its numeric id, so both are
/// read and the value is shown under its name. An application whose bucket
/// cannot be read, or whose answer is not a bucket, is kept in the list with
/// [`UNKNOWN_BUCKET`] as its value and counted in
/// [`BucketSnapshot::unreadable`], so a single failure does not hide the rest
/// of the list and an unknown value never passes for a valid bucket.
pub fn read_buckets(
    client: &AdbClient,
    serial: &str,
    packages: &[String],
) -> BucketSnapshot {
    let answers = query_packages(packages, |package| {
        return get_standby_bucket(client, serial, package);
    });

    let mut entries = Vec::new();
    let mut unreadable = 0;

    for (index, package) in packages.iter().enumerate() {
        let bucket = match answers.get(index) {
            Some(Ok(answer)) => StandbyBucket::parse(answer),
            _ => None,
        };
        let bucket = match bucket {
            Some(bucket) => String::from(bucket.name()),
            None => {
                unreadable += 1;
                String::from(UNKNOWN_BUCKET)
            }
        };

        let entry = ControlEntry {
            identifier: package.clone(),
            value: bucket,
        };
        entries.push(entry);
    }

    let snapshot = BucketSnapshot {
        entries,
        unreadable,
    };
    return snapshot;
}
