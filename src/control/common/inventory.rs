//! The package inventory shared by every control section.
//!
//! All four sections show the applications of the same device, so the panel
//! reads the installed package list once and hands the same immutable list to
//! every section. Nothing else asks the device for the package list, and the
//! list is dropped as soon as the selected device changes.

use std::sync::Arc;

use crate::adb::AdbClient;
use crate::adb::list_packages;

use super::worker::WorkerTask;

/// The installed packages of one device.
///
/// The list is immutable and reference counted: the panel reads it once and
/// every section shares the same value, so reading it costs nothing after the
/// first load.
pub type PackageInventory = Arc<[String]>;

/// The package inventory a control section works with.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Inventory {
    /// No device is selected.
    Absent,
    /// The installed packages are being read.
    Loading,
    /// The installed packages of the selected device.
    Loaded(PackageInventory),
    /// The installed packages could not be read, with the failure as shown to
    /// the user.
    Failed(String),
}

/// Reads the installed packages of one device on the inventory worker.
#[non_exhaustive]
pub struct InventoryTask {
    client: AdbClient,
    serial: String,
}

/// The result of an [`InventoryTask`].
#[non_exhaustive]
pub enum InventoryOutcome {
    /// The packages were read.
    Loaded {
        /// The device the packages belong to.
        serial: String,
        /// The installed packages, sorted by identifier.
        packages: PackageInventory,
    },
    /// The packages could not be read.
    Failed {
        /// The device the attempt was made on.
        serial: String,
        /// The failure, as shown to the user.
        message: String,
    },
}

impl InventoryTask {
    /// Builds a task that reads the installed packages of one device.
    pub fn load(client: AdbClient, serial: String) -> Self {
        let task = Self {
            client,
            serial,
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
}

impl WorkerTask for InventoryTask {
    type Response = InventoryOutcome;

    fn execute(self) -> InventoryOutcome {
        match list_packages(&self.client, &self.serial) {
            Ok(packages) => {
                let packages = PackageInventory::from(packages);
                let outcome = InventoryOutcome::Loaded {
                    serial: self.serial,
                    packages,
                };
                return outcome;
            }
            Err(error) => {
                let message = format!("could not list applications: {error}");
                let outcome = InventoryOutcome::Failed {
                    serial: self.serial,
                    message,
                };
                return outcome;
            }
        }
    }
}
