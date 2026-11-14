//! The `am` commands that read and change the inactive state.
//!
//! The arguments are handed to ADB as separate process arguments, never as a
//! shell string, so package names cannot be interpreted as shell input.

use crate::adb::AdbClient;
use crate::adb::AdbError;

use super::state::InactiveState;

/// The key that `am get-inactive` prints in front of the state.
const KEY: &str = "Idle";

/// Reads the inactive state that a device reports for one package.
///
/// `am get-inactive` prints `Idle=true` or `Idle=false`. An answer that cannot
/// be parsed yields [`None`], which the section shows as an unknown state.
pub fn get_inactive(
    client: &AdbClient,
    serial: &str,
    package: &str,
) -> Result<Option<InactiveState>, AdbError> {
    let arguments = ["-s", serial, "shell", "am", "get-inactive", package];
    let output = client.run(&arguments)?;
    return Ok(parse_state(&output));
}

/// Asks a device to set the inactive state of one package.
pub fn set_inactive(
    client: &AdbClient,
    serial: &str,
    package: &str,
    state: InactiveState,
) -> Result<(), AdbError> {
    let arguments =
        ["-s", serial, "shell", "am", "set-inactive", package, state.flag()];
    client.run(&arguments)?;
    return Ok(());
}

/// Parses the output of `am get-inactive <package>`.
///
/// The command prints `Idle=true` for an inactive application and
/// `Idle=false` for an active one. A bare boolean is accepted as well, and
/// lines that carry neither are ignored. An empty answer, an unrelated answer,
/// or a state this application does not know yields [`None`].
fn parse_state(output: &str) -> Option<InactiveState> {
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let (name, value) = match line.split_once('=') {
            Some(parts) => parts,
            None => {
                let state = InactiveState::from_flag(line);
                if state.is_some() {
                    return state;
                }
                continue;
            }
        };
        if !name.trim().eq_ignore_ascii_case(KEY) {
            continue;
        }

        let flag = value.split_whitespace().next()?;
        return InactiveState::from_flag(flag.trim_end_matches(';'));
    }
    return None;
}
