//! The `deviceidle` commands that read and change the whitelist.
//!
//! The arguments are handed to ADB as separate process arguments, never as a
//! shell string, so package names cannot be interpreted as shell input. Only
//! the user whitelist is changed: the temporary whitelist and the system
//! whitelist are never touched.

use crate::adb::AdbClient;
use crate::adb::AdbError;

/// Reads the complete Device Idle whitelist of a device.
pub fn get_whitelist(
    client: &AdbClient,
    serial: &str,
) -> Result<String, AdbError> {
    let arguments = ["-s", serial, "shell", "cmd", "deviceidle", "whitelist"];
    return client.run(&arguments);
}

/// Adds a package to the user whitelist.
pub fn add_whitelist_entry(
    client: &AdbClient,
    serial: &str,
    package: &str,
) -> Result<(), AdbError> {
    let argument = format!("+{package}");
    let arguments = [
        "-s",
        serial,
        "shell",
        "cmd",
        "deviceidle",
        "whitelist",
        argument.as_str(),
    ];
    client.run(&arguments)?;
    return Ok(());
}

/// Removes a package from the user whitelist.
pub fn remove_whitelist_entry(
    client: &AdbClient,
    serial: &str,
    package: &str,
) -> Result<(), AdbError> {
    let argument = format!("-{package}");
    let arguments = [
        "-s",
        serial,
        "shell",
        "cmd",
        "deviceidle",
        "whitelist",
        argument.as_str(),
    ];
    client.run(&arguments)?;
    return Ok(());
}
