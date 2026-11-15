//! The background read of the package inventory shared by every section.
//!
//! The panel reads the installed packages of one device once and hands the same
//! immutable list to every section. The listing itself is application
//! discovery, so the scanner lives in [`crate::discover::apps`]; this module only
//! carries one read across the control worker thread and back.
//!
//! Every request carries the revision of the inventory it was started for. The
//! panel accepts only the answer of the newest request, so a read that a device
//! or filter change overtook cannot replace newer data.

use crate::adb::AdbClient;
use crate::discover::apps::ApplicationFilter;
use crate::discover::apps::ApplicationScanner;
use crate::discover::apps::PackageInventory;

use super::worker::WorkerTask;

/// Reads the installed packages of one device with the global filter.
#[non_exhaustive]
pub struct InventoryTask {
    scanner: ApplicationScanner,
    serial: String,
    filter: ApplicationFilter,
    revision: u64,
}

/// The result of an [`InventoryTask`].
#[non_exhaustive]
pub enum InventoryOutcome {
    /// The packages were read.
    Loaded {
        /// The device the packages belong to.
        serial: String,
        /// The revision of the request that produced this answer.
        revision: u64,
        /// The installed packages, sorted by identifier.
        packages: PackageInventory,
    },
    /// The packages could not be read.
    Failed {
        /// The device the attempt was made on.
        serial: String,
        /// The revision of the request that produced this answer.
        revision: u64,
        /// The failure, as shown to the user.
        message: String,
    },
}

impl InventoryTask {
    /// Builds a task that reads the installed packages of one device.
    ///
    /// The filter decides whether system applications are part of the list, and
    /// the revision identifies the request, so an answer that a newer request
    /// overtook is recognizable.
    pub fn load(
        client: AdbClient,
        serial: String,
        filter: ApplicationFilter,
        revision: u64,
    ) -> Self {
        let task = Self {
            scanner: ApplicationScanner::new(client),
            serial,
            filter,
            revision,
        };
        return task;
    }
}

impl InventoryOutcome {
    /// Returns the device the outcome belongs to.
    pub fn serial(&self) -> &str {
        let serial = match self {
            InventoryOutcome::Loaded {
                serial,
                ..
            } => serial,
            InventoryOutcome::Failed {
                serial,
                ..
            } => serial,
        };
        return serial.as_str();
    }

    /// Returns the revision of the request the outcome answers.
    pub fn revision(&self) -> u64 {
        let revision = match self {
            InventoryOutcome::Loaded {
                revision,
                ..
            } => *revision,
            InventoryOutcome::Failed {
                revision,
                ..
            } => *revision,
        };
        return revision;
    }
}

impl WorkerTask for InventoryTask {
    type Response = InventoryOutcome;

    fn execute(self) -> InventoryOutcome {
        let listed = self.scanner.scan(&self.serial, self.filter);

        match listed {
            Ok(packages) => {
                let outcome = InventoryOutcome::Loaded {
                    serial: self.serial,
                    revision: self.revision,
                    packages,
                };
                return outcome;
            }
            Err(error) => {
                let message = format!("could not list applications: {error}");
                let outcome = InventoryOutcome::Failed {
                    serial: self.serial,
                    revision: self.revision,
                    message,
                };
                return outcome;
            }
        }
    }
}
