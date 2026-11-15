//! Package removal for Android devices.
//!
//! The module owns the removal of a package and the command that performs it.
//! It is a layer of its own: [`crate::adb`] runs the external ADB executable,
//! [`crate::discover`] reports devices and their packages, [`crate::control`]
//! manages application states, and [`crate::tui`] draws the interface. Nothing
//! here knows about the terminal, so a caller can run the work on a background
//! worker.
//!
//! The files are split by responsibility:
//!
//! - [`package`] holds the identity of the package to remove.
//! - [`validation`] rejects requests that cannot run and hands out only
//!   validated requests.
//! - [`commands`] builds the ADB command of a removal method without running
//!   it.
//! - [`execution`] defines the operation and the interface that removal methods
//!   will implement.
//! - [`result`] describes what a removal achieved and why it failed.
//!
//! Removal methods are not implemented yet. Each method arrives on its own: a
//! user-level uninstall removes the application for the current Android user
//! without claiming that its APK is gone from the system image, while a system
//! package removal changes the system image and needs its own permissions and
//! safety rules. No method is assumed to support every package or every device.
//!
//! Removal operations report to the centralized Operations Log through
//! [`crate::log::OperationLog`]; this module neither owns nor duplicates a log
//! and records nothing on its own.

pub mod commands;
pub mod execution;
pub mod package;
pub mod result;
pub mod validation;

pub use commands::RemovalCommand;
pub use commands::RemovalMethod;
pub use execution::RemovalOperation;
pub use execution::RemovalProgress;
pub use execution::RemovalStrategy;
pub use package::RemovalPackage;
pub use result::RemovalError;
pub use result::RemovalOutcome;
pub use validation::RemovalRequest;
pub use validation::ValidationError;
pub use validation::validate;
