//! Device Idle whitelist management.
//!
//! Android keeps a Device Idle whitelist of applications that may run when the
//! device is idle. The list has two lasting parts: entries the user added and
//! entries Android manages through a system whitelist. The section lists every
//! installed application with its state and moves the selected application
//! between the user whitelist and no whitelist at all; system entries are
//! shown but never changed.
//!
//! The module is split by responsibility, like the other control areas:
//! `state` holds the three states and the classification rules, `parser` reads
//! the complete whitelist output, `commands` runs the `cmd deviceidle
//! whitelist` commands, `scan` combines the installed packages with the parsed
//! whitelist, `task` runs that work on the control worker thread, and
//! `section` owns the section state and turns key presses into tasks.

mod commands;
mod parser;
mod scan;
mod section;
mod state;
mod task;

pub use section::DozeSection;
pub use state::WhitelistState;
