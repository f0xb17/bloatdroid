//! AppOps background execution restrictions.
//!
//! An AppOps operation tells Android what an application may do in the
//! background. This first iteration covers one operation,
//! `RUN_ANY_IN_BACKGROUND`: the section lists every installed application with
//! the mode the device reports and cycles the selected application through the
//! modes that Android accepts.
//!
//! The module is split by responsibility, like the standby buckets: `mode`
//! holds the modes and the order in which they cycle, `commands` runs the
//! `cmd appops get` and `cmd appops set` commands, `scan` reads the mode of
//! every installed application, `task` runs that work on the control worker
//! thread, and `section` owns the section state and turns key presses into
//! tasks.

mod commands;
mod mode;
mod scan;
mod section;
mod task;

pub use mode::AppOpMode;
pub use section::AppOpsSection;
