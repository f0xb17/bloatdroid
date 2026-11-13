//! A thin abstraction over the external `adb` executable.
//!
//! This module locates and runs ADB, passes arguments without a shell,
//! captures standard output and standard error, and reports failures as
//! [`AdbError`] values. It knows nothing about the application: device
//! discovery, package management, and Standby Crates build on it from the
//! outside.
//!
//! Every command lives in its own file so that later commands can be added
//! without growing this module:
//!
//! - [`devices`] runs `adb devices -l`.
//! - [`get_state`] runs `adb -s <serial> get-state`.
//! - [`get_prop`] runs `adb -s <serial> shell getprop <property>`.

pub mod client;
pub mod devices;
pub mod error;
pub mod get_prop;
pub mod get_state;

pub use client::AdbClient;
pub use devices::{AdbDeviceEntry, list_devices, parse_devices};
pub use error::AdbError;
pub use get_prop::get_prop;
pub use get_state::get_state;
