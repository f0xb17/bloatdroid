//! The standby bucket work that runs on the control worker thread.

use crate::adb::AdbClient;
use crate::control::common::ControlEntry;
use crate::control::common::PackageInventory;
use crate::control::common::WorkerTask;

use super::commands::get_standby_bucket;
use super::commands::set_standby_bucket;
use super::scan::read_buckets;

/// A standby bucket operation that runs on the worker thread.
#[non_exhaustive]
pub struct BucketTask {
    client: AdbClient,
    serial: String,
    operation: BucketOperation,
}

/// The operation a [`BucketTask`] performs.
enum BucketOperation {
    /// Read the bucket of every package of the shared inventory.
    Load {
        /// The applications to read.
        packages: PackageInventory,
    },
    /// Move one application into a bucket and verify the result.
    Change {
        /// The package to change.
        package: String,
        /// The bucket to move it into.
        bucket: String,
    },
}

/// The result of a [`BucketTask`].
#[non_exhaustive]
pub enum BucketOutcome {
    /// The applications and their buckets were read.
    Loaded {
        /// The device the snapshot belongs to.
        serial: String,
        /// The applications, sorted by package identifier.
        entries: Vec<ControlEntry>,
        /// How many applications could not be read.
        unreadable: usize,
    },
    /// A bucket was applied and read back.
    Changed {
        /// The device the change was made on.
        serial: String,
        /// The package that was changed.
        package: String,
        /// The bucket the device reports after the change.
        bucket: String,
    },
    /// A bucket change failed or was not applied.
    ChangeFailed {
        /// The device the attempt was made on.
        serial: String,
        /// The package that was meant to change.
        package: String,
        /// The failure, as shown to the user.
        message: String,
    },
}

impl BucketTask {
    /// Builds a task that reads the bucket of every package of an inventory.
    pub fn load(
        client: AdbClient,
        serial: String,
        packages: PackageInventory,
    ) -> Self {
        let operation = BucketOperation::Load {
            packages,
        };
        let task = Self {
            client,
            serial,
            operation,
        };
        return task;
    }

    /// Builds a task that moves one package into a bucket.
    pub fn change(
        client: AdbClient,
        serial: String,
        package: String,
        bucket: String,
    ) -> Self {
        let operation = BucketOperation::Change {
            package,
            bucket,
        };
        let task = Self {
            client,
            serial,
            operation,
        };
        return task;
    }
}

impl BucketOutcome {
    /// Returns the device the outcome belongs to.
    pub fn serial(&self) -> &str {
        let serial = match self {
            BucketOutcome::Loaded {
                serial,
                ..
            } => serial,
            BucketOutcome::Changed {
                serial,
                ..
            } => serial,
            BucketOutcome::ChangeFailed {
                serial,
                ..
            } => serial,
        };
        return serial.as_str();
    }
}

impl WorkerTask for BucketTask {
    type Response = BucketOutcome;

    fn execute(self) -> BucketOutcome {
        match self.operation {
            BucketOperation::Load {
                packages,
            } => {
                return load(&self.client, &self.serial, &packages);
            }
            BucketOperation::Change {
                package,
                bucket,
            } => {
                return change(&self.client, &self.serial, &package, &bucket);
            }
        }
    }
}

/// Reads the bucket of every package of an inventory.
fn load(
    client: &AdbClient,
    serial: &str,
    packages: &[String],
) -> BucketOutcome {
    let snapshot = read_buckets(client, serial, packages);

    let outcome = BucketOutcome::Loaded {
        serial: String::from(serial),
        entries: snapshot.entries,
        unreadable: snapshot.unreadable,
    };
    return outcome;
}

/// Applies one bucket and verifies it by reading it back.
fn change(
    client: &AdbClient,
    serial: &str,
    package: &str,
    bucket: &str,
) -> BucketOutcome {
    match set_standby_bucket(client, serial, package, bucket) {
        Ok(()) => {}
        Err(error) => {
            let message = format!("the device rejected the change: {error}");
            return failure(serial, package, message);
        }
    }

    let actual = match get_standby_bucket(client, serial, package) {
        Ok(actual) => actual,
        Err(error) => {
            let message = format!("could not verify the change: {error}");
            return failure(serial, package, message);
        }
    };

    if actual != bucket {
        let message = format!("the device kept `{actual}`");
        return failure(serial, package, message);
    }

    let outcome = BucketOutcome::Changed {
        serial: String::from(serial),
        package: String::from(package),
        bucket: actual,
    };
    return outcome;
}

/// Builds a failed change outcome.
fn failure(serial: &str, package: &str, message: String) -> BucketOutcome {
    let outcome = BucketOutcome::ChangeFailed {
        serial: String::from(serial),
        package: String::from(package),
        message,
    };
    return outcome;
}
