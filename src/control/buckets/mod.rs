//! Android standby buckets.
//!
//! A standby bucket tells Android how much background work an application may
//! do. The section lists every installed application with its current bucket
//! and moves the selected application between the buckets that the connected
//! device supports.
//!
//! The buckets module is split by responsibility: `bucket` holds the buckets
//! and the rules for the ones a device accepts, `commands` runs the
//! `am get-standby-bucket` and `am set-standby-bucket` commands, `scan` reads
//! the bucket of every installed application, `task` runs that work on the
//! control worker thread, and `section` owns the section state and turns key
//! presses into tasks.

mod bucket;
mod commands;
mod scan;
mod section;
mod task;

pub use bucket::StandbyBucket;
pub use section::BucketsSection;
