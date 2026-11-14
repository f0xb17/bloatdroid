//! The client every ADB command runs through.

use std::ffi::OsString;
use std::sync::Arc;

use super::error::AdbError;
use super::executable::ExecutableRunner;
use super::runner::AdbRunner;

/// Runs ADB commands through an [`AdbRunner`].
///
/// The client knows nothing about processes: by default it runs the `adb`
/// executable through an [`ExecutableRunner`], and a different runner can be
/// supplied, for example to answer commands without a device.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct AdbClient {
    runner: Arc<dyn AdbRunner>,
}

impl Default for AdbClient {
    fn default() -> Self {
        return Self::new();
    }
}

impl AdbClient {
    /// Creates a client that runs `adb` from `PATH`.
    pub fn new() -> Self {
        return Self::with_executable("adb");
    }

    /// Creates a client that runs the given executable.
    ///
    /// Use this when ADB is not on `PATH` or when `ADB` points elsewhere.
    pub fn with_executable(executable: impl Into<OsString>) -> Self {
        let runner = ExecutableRunner::new(executable);
        return Self::with_runner(Arc::new(runner));
    }

    /// Creates a client that runs commands through the given runner.
    ///
    /// This is the seam that lets tests run the application without a device.
    pub fn with_runner(runner: Arc<dyn AdbRunner>) -> Self {
        let client = Self {
            runner,
        };
        return client;
    }

    /// Runs ADB with the given arguments and returns its standard output.
    ///
    /// The command is waited for, so callers that must not block the user
    /// interface should run this from a worker thread.
    pub fn run(&self, arguments: &[&str]) -> Result<String, AdbError> {
        return self.runner.run(arguments);
    }
}
