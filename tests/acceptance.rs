//! The device-independent test suite.
//!
//! Nothing here needs a physical Android device or a running ADB server:
//! every ADB command goes through a mock runner, and the tests never touch
//! terminal rendering.

mod adb;
mod common;
mod control;
mod discover;
mod tui;
