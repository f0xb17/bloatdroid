//! The `adb shell getprop` command.

use super::client::AdbClient;
use super::error::AdbError;

/// Reads one Android system property from a device.
///
/// Returns the property value, trimmed. An unset property yields an empty
/// string, which callers should treat as missing information.
pub fn get_prop(
    client: &AdbClient,
    serial: &str,
    property: &str,
) -> Result<String, AdbError> {
    let output = client.run(&["-s", serial, "shell", "getprop", property])?;
    return Ok(output.trim().to_string());
}
