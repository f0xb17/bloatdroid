//! The package inventory shared by every control section.
//!
//! All four sections show the applications of the same device, so the panel
//! reads the installed package list once and hands the same immutable list to
//! every section. Nothing else asks the device for the package list, and the
//! list is dropped as soon as the selected device or the system application
//! filter changes.

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
    include_system: bool,
}

/// The result of an [`InventoryTask`].
#[non_exhaustive]
pub enum InventoryOutcome {
    /// The packages were read.
    Loaded {
        /// The device the packages belong to.
        serial: String,
        /// Whether the system packages were part of the request.
        include_system: bool,
        /// The installed packages, sorted by identifier.
        packages: PackageInventory,
    },
    /// The packages could not be read.
    Failed {
        /// The device the attempt was made on.
        serial: String,
        /// Whether the system packages were part of the request.
        include_system: bool,
        /// The failure, as shown to the user.
        message: String,
    },
}

impl InventoryTask {
    /// Builds a task that reads the installed packages of one device.
    ///
    /// `include_system` decides whether the system applications are part of
    /// the list; without it the package manager only reports the applications
    /// the user installed.
    pub fn load(
        client: AdbClient,
        serial: String,
        include_system: bool,
    ) -> Self {
        let task = Self {
            client,
            serial,
            include_system,
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

    /// Returns whether the request included the system applications.
    pub fn include_system(&self) -> bool {
        let include_system = match self {
            InventoryOutcome::Loaded {
                include_system,
                ..
            } => *include_system,
            InventoryOutcome::Failed {
                include_system,
                ..
            } => *include_system,
        };
        return include_system;
    }
}

impl WorkerTask for InventoryTask {
    type Response = InventoryOutcome;

    fn execute(self) -> InventoryOutcome {
        let listed =
            list_packages(&self.client, &self.serial, self.include_system);

        match listed {
            Ok(packages) => {
                let packages = PackageInventory::from(packages);
                let outcome = InventoryOutcome::Loaded {
                    serial: self.serial,
                    include_system: self.include_system,
                    packages,
                };
                return outcome;
            }
            Err(error) => {
                let message = format!("could not list applications: {error}");
                let outcome = InventoryOutcome::Failed {
                    serial: self.serial,
                    include_system: self.include_system,
                    message,
                };
                return outcome;
            }
        }
    }
}
