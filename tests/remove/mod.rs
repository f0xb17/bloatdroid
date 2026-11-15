//! Tests for package removal.
//!
//! The suite uses the mock ADB runner and never executes a removal command, so
//! no Android device is needed.

mod commands;
mod execution;
mod harness;
mod package;
mod result;
mod uninstall;
mod validation;
