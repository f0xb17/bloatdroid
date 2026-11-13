//! Locating and running the ADB executable.

use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use super::error::AdbError;

/// Runs the ADB executable with explicit arguments.
///
/// Arguments are passed to the process directly, never through a shell, so
/// serial numbers and property names cannot be interpreted as shell input.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct AdbClient {
    executable: PathBuf,
}

impl Default for AdbClient {
    fn default() -> Self {
        return Self::new();
    }
}

impl AdbClient {
    /// Creates a client that looks up `adb` on `PATH`.
    pub fn new() -> Self {
        return Self::with_executable("adb");
    }

    /// Creates a client that runs the given executable.
    ///
    /// Use this when ADB is not on `PATH` or when `ADB` points elsewhere.
    pub fn with_executable(executable: impl Into<OsString>) -> Self {
        let client = Self {
            executable: PathBuf::from(executable.into()),
        };
        return client;
    }

    /// Returns the executable this client runs.
    pub fn executable(&self) -> &Path {
        return self.executable.as_path();
    }

    /// Runs ADB with the given arguments and returns its standard output.
    ///
    /// The command is waited for, so callers that must not block the user
    /// interface should run this from a worker thread.
    pub fn run(&self, arguments: &[&str]) -> Result<String, AdbError> {
        let mut command = Command::new(&self.executable);
        for argument in arguments {
            command.arg(argument);
        }

        let output = match command.output() {
            Ok(output) => output,
            Err(error) => return Err(AdbError::Io(error)),
        };

        let label = self.label(arguments);
        if !output.status.success() {
            let stderr =
                String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(AdbError::CommandFailed {
                command: label,
                status: output.status,
                stderr,
            });
        }

        let stdout = match String::from_utf8(output.stdout) {
            Ok(stdout) => stdout,
            Err(_) => {
                return Err(AdbError::OutputNotUtf8 {
                    command: label,
                });
            }
        };
        return Ok(stdout);
    }

    /// Builds a readable label for a command, used in error messages.
    fn label(&self, arguments: &[&str]) -> String {
        let mut label = self.executable.to_string_lossy().to_string();
        for argument in arguments {
            label.push(' ');
            label.push_str(argument);
        }
        return label;
    }
}
