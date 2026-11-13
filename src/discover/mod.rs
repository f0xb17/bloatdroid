//! Device discovery on top of the ADB abstraction.
//!
//! The scanner asks ADB which devices are attached, keeps the online physical
//! ones, reads their properties, and hands structured [`AndroidDevice`] values
//! to the interface. Parsing and classification live in [`parser`], the data
//! structure in [`device`], and the orchestration in [`scanner`].

pub mod device;
pub mod parser;
pub mod scanner;

pub use device::AndroidDevice;
pub use scanner::DeviceScanner;

use std::time::Duration;

/// How often the interface refreshes the device list automatically.
pub const REFRESH_INTERVAL: Duration = Duration::from_secs(10);
