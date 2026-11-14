//! The Android application inactive state.
//!
//! Android can mark an installed application as inactive, which stops it from
//! running in the background until the user opens it again. The section lists
//! every installed application with the state the device reports and toggles
//! the selected application between `active` and `inactive`.
//!
//! The module is split by responsibility, like the other control areas:
//! `state` holds the two states and their boolean form, `commands` runs the
//! `am get-inactive` and `am set-inactive` commands, `scan` reads the state of
//! every installed application, `task` runs that work on the control worker
//! thread, and `section` owns the section state and turns key presses into
//! tasks.

mod commands;
mod scan;
mod section;
mod state;
mod task;

pub use section::InactiveSection;
pub use state::InactiveState;
