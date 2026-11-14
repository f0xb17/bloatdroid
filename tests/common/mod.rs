//! Helpers shared by the test suites.
//!
//! Each suite uses only a part of these helpers, so unused ones must not
//! warn.
#![allow(dead_code)]

pub mod mock_adb;

pub use mock_adb::MockAdb;
pub use mock_adb::MockResponse;
