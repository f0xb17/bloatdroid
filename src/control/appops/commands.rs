//! The `appops` commands that read and change one AppOps mode.
//!
//! The arguments are handed to ADB as separate process arguments, never as a
//! shell string, so package names cannot be interpreted as shell input.

use crate::adb::AdbClient;
use crate::adb::AdbError;

use super::mode::AppOpMode;

/// The AppOps operation this module reads and changes.
const OPERATION: &str = "RUN_ANY_IN_BACKGROUND";

/// Reads the mode that a device reports for one package.
///
/// Android omits the operation when the effective mode is the default, so an
/// empty answer means [`AppOpMode::Default`]. An answer that cannot be parsed
/// yields [`None`], which the section shows as an unknown mode.
pub fn get_mode(
    client: &AdbClient,
    serial: &str,
    package: &str,
) -> Result<Option<AppOpMode>, AdbError> {
    let arguments =
        ["-s", serial, "shell", "cmd", "appops", "get", package, OPERATION];
    let output = client.run(&arguments)?;
    return Ok(parse_mode(&output));
}

/// Asks a device to set the mode of one package.
pub fn set_mode(
    client: &AdbClient,
    serial: &str,
    package: &str,
    mode: AppOpMode,
) -> Result<(), AdbError> {
    let arguments = [
        "-s",
        serial,
        "shell",
        "cmd",
        "appops",
        "set",
        package,
        OPERATION,
        mode.name(),
    ];
    client.run(&arguments)?;
    return Ok(());
}

/// Parses the output of `cmd appops get <package> RUN_ANY_IN_BACKGROUND`.
///
/// Android prints an entry only for an operation that differs from the
/// package defaults, so empty output means [`AppOpMode::Default`]. An entry
/// looks like `RUN_ANY_IN_BACKGROUND: allow` and can carry further fields, as
/// in `RUN_ANY_IN_BACKGROUND: ignore; time=+1h2m3s`. Anything else, including
/// an entry with a mode this application does not know, yields [`None`].
fn parse_mode(output: &str) -> Option<AppOpMode> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return Some(AppOpMode::Default);
    }

    for line in trimmed.lines() {
        let line = line.trim();
        if line.eq_ignore_ascii_case("no operations.") {
            return Some(AppOpMode::Default);
        }

        let (name, value) = match line.split_once(':') {
            Some(parts) => parts,
            None => continue,
        };
        if name.trim() != OPERATION {
            continue;
        }

        let mode = match value.split_whitespace().next() {
            Some(mode) => mode,
            None => return Some(AppOpMode::Default),
        };
        return AppOpMode::from_name(mode.trim_end_matches(';'));
    }
    return None;
}
