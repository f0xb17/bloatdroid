//! Running commands through the external ADB executable.

use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use super::error::AdbError;
use super::runner::AdbRunner;

/// Runs commands by starting the ADB executable.
///
/// Arguments are passed to the process directly, never through a shell, so
/// serial numbers and property names cannot be interpreted as shell input.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ExecutableRunner {
    executable: PathBuf,
}

impl ExecutableRunner {
    /// Creates a runner that runs the given executable.
    pub fn new(executable: impl Into<OsString>) -> Self {
        let runner = Self {
            executable: PathBuf::from(executable.into()),
        };
        return runner;
    }

    /// Returns the executable this runner runs.
    pub fn executable(&self) -> &Path {
        return self.executable.as_path();
    }
}

impl Default for ExecutableRunner {
    fn default() -> Self {
        return Self::new("adb");
    }
}

impl AdbRunner for ExecutableRunner {
    fn run(&self, arguments: &[&str]) -> Result<String, AdbError> {
        let mut command = Command::new(&self.executable);
        for argument in arguments {
            command.arg(argument);
        }

        let output = match command.output() {
            Ok(output) => output,
            Err(error) => return Err(AdbError::Io(error)),
        };

        let label = label(&self.executable, arguments);
        if !output.status.success() {
            let stderr =
                String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(AdbError::CommandFailed {
                command: label,
                status: output.status,
                stderr,
            });
        }

        match String::from_utf8(output.stdout) {
            Ok(stdout) => return Ok(stdout),
            Err(_) => {
                return Err(AdbError::OutputNotUtf8 {
                    command: label,
                });
            }
        }
    }
}

/// Builds a readable label for a command, used in error messages.
fn label(executable: &Path, arguments: &[&str]) -> String {
    let mut label = executable.to_string_lossy().to_string();
    for argument in arguments {
        label.push(' ');
        label.push_str(argument);
    }
    return label;
}
