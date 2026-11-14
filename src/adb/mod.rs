//! A thin abstraction over the external `adb` executable.
//!
//! This module locates and runs ADB, passes arguments without a shell,
//! captures standard output and standard error, and reports failures as
//! [`AdbError`] values. It knows nothing about the application: device
//! discovery, package management, and Standby Crates build on it from the
//! outside.
//!
//! Commands run through an [`AdbRunner`], so the way ADB is executed can be
//! replaced — by a test double, for instance — without changing any command.
//! The pieces are split by responsibility:
//!
//! - [`client`] is the entry point every command runs through.
//! - [`runner`] is the interface between the client and execution.
//! - [`executable`] runs the external `adb` executable.
//!
//! Every command lives in its own file so that later commands can be added
//! without growing this module:
//!
//! - [`devices`] runs `adb devices -l`.
//! - [`get_state`] runs `adb -s <serial> get-state`.
//! - [`get_prop`] runs `adb -s <serial> shell getprop <property>`.
//! - [`list_packages`] runs `adb -s <serial> shell pm list packages`.

pub mod client;
pub mod devices;
pub mod error;
pub mod executable;
pub mod get_prop;
pub mod get_state;
pub mod list_packages;
pub mod runner;

pub use client::AdbClient;
pub use devices::{AdbDeviceEntry, list_devices, parse_devices};
pub use error::AdbError;
pub use executable::ExecutableRunner;
pub use get_prop::get_prop;
pub use get_state::get_state;
pub use list_packages::{list_packages, parse_packages};
pub use runner::AdbRunner;
