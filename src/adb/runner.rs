//! The interface between the ADB client and command execution.

use std::fmt;

use super::error::AdbError;

/// Runs one ADB command and returns its standard output.
///
/// [`AdbClient`](super::AdbClient) runs everything through a runner, so the
/// application can use the external ADB executable while tests answer
/// commands without a device or an ADB server.
pub trait AdbRunner: Send + Sync + fmt::Debug {
    /// Runs ADB with the given arguments and returns its standard output.
    ///
    /// Arguments are passed to the process directly, never through a shell,
    /// so serial numbers and property names cannot be interpreted as shell
    /// input.
    fn run(&self, arguments: &[&str]) -> Result<String, AdbError>;
}
