//! Device and application discovery on top of the ADB abstraction.
//!
//! [`scanner`] asks ADB which devices are attached, keeps the online physical
//! ones, reads their properties, and hands structured [`AndroidDevice`] values
//! to the interface. [`apps`] reads the packages of one device and holds the
//! filter that decides whether system applications are part of them. The two
//! stay apart: the device scanner knows nothing about packages, and the
//! application scanner knows nothing about the device list.
//!
//! The system application filter is application-wide state. It is created once,
//! handed to the interface, and applied to every application inventory, so no
//! feature that lists applications keeps a setting of its own. Parsing and
//! classification live in [`parser`], the device structure in [`device`], and
//! the orchestration of a device scan in [`scanner`].

pub mod apps;
pub mod device;
pub mod parser;
pub mod scanner;

pub use apps::ApplicationFilter;
pub use apps::ApplicationScanner;
pub use apps::Inventory;
pub use apps::PackageInventory;
pub use device::AndroidDevice;
pub use scanner::DeviceScanner;

use std::time::Duration;

/// How often the interface refreshes the device list automatically.
pub const REFRESH_INTERVAL: Duration = Duration::from_secs(10);
