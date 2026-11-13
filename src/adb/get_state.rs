//! The `adb get-state` command.

use super::client::AdbClient;
use super::error::AdbError;

/// Asks ADB for the connection state of one device.
///
/// Returns the state as reported by ADB, for example `device` or `offline`.
pub fn get_state(client: &AdbClient, serial: &str) -> Result<String, AdbError> {
    let output = client.run(&["-s", serial, "get-state"])?;
    return Ok(output.trim().to_string());
}
