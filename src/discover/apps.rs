//! Application package discovery.
//!
//! Device discovery answers which devices are attached; application discovery
//! answers which packages one of them has installed. The two are kept apart:
//! [`ApplicationScanner`] never looks at the device list, and
//! [`DeviceScanner`](super::DeviceScanner) never looks at packages.
//!
//! [`ApplicationFilter`] is the one place that decides whether system
//! applications are part of an inventory. Every feature that lists or processes
//! applications reads its packages through this module and the filter it was
//! given, so they all work with the same applications.

use std::sync::Arc;

use crate::adb::AdbClient;
use crate::adb::AdbError;
use crate::adb::list_packages;

/// The installed packages of one device.
///
/// The list is immutable and reference counted, so every feature that shows the
/// inventory shares one value instead of copying it.
pub type PackageInventory = Arc<[String]>;

/// The global decision about which applications are part of the inventory.
///
/// System applications are excluded by default, so an inventory holds only the
/// applications the user installed until the setting is turned on. There is one
/// filter for the whole application: nothing that lists applications keeps a
/// setting of its own.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct ApplicationFilter {
    show_system: bool,
}

impl ApplicationFilter {
    /// Creates a filter that leaves the system applications out.
    pub fn new() -> Self {
        return Self::with_show_system(false);
    }

    /// Creates a filter with the given system application setting.
    pub fn with_show_system(show_system: bool) -> Self {
        let filter = Self {
            show_system,
        };
        return filter;
    }

    /// Reports whether system applications are part of the inventory.
    pub fn show_system(self) -> bool {
        return self.show_system;
    }

    /// Turns the system application setting on or off.
    ///
    /// Returns the setting as it is afterwards, so the caller can report it.
    pub fn toggle(&mut self) -> bool {
        self.show_system = !self.show_system;
        return self.show_system;
    }
}

/// The application inventory that the interface works with.
///
/// The inventory belongs to one device and one filter setting. A device change
/// drops it, and a filter change reads it again, so a list is never reused for a
/// device or a setting it was not read for.
#[derive(Clone, Debug, Eq, PartialEq)]
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

/// Reads the installed packages of one device.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ApplicationScanner {
    client: AdbClient,
}

impl ApplicationScanner {
    /// Creates a scanner that runs ADB through the given client.
    pub fn new(client: AdbClient) -> Self {
        let scanner = Self {
            client,
        };
        return scanner;
    }

    /// Lists the applications of one device that the filter includes.
    ///
    /// The package manager does the filtering: without system applications the
    /// device only reports the packages the user installed (`pm list packages
    /// -3`), and with them it reports the complete inventory. No package name is
    /// examined here, so the result is exactly what the device installs and the
    /// filter allows.
    pub fn scan(
        &self,
        serial: &str,
        filter: ApplicationFilter,
    ) -> Result<PackageInventory, AdbError> {
        let listed = list_packages(&self.client, serial, filter.show_system())?;
        let packages = PackageInventory::from(listed);
        return Ok(packages);
    }
}
