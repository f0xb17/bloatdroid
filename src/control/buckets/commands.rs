//! The `am` commands that read and change standby buckets.
//!
//! The arguments are handed to ADB as separate process arguments, never as a
//! shell string, so package names and bucket names cannot be interpreted as
//! shell input.

use crate::adb::AdbClient;
use crate::adb::AdbError;

/// Reads the standby bucket that a device reports for one package.
pub fn get_standby_bucket(
    client: &AdbClient,
    serial: &str,
    package: &str,
) -> Result<String, AdbError> {
    let arguments =
        ["-s", serial, "shell", "am", "get-standby-bucket", package];
    let output = client.run(&arguments)?;
    return Ok(output.trim().to_string());
}

/// Asks a device to move one package into a standby bucket.
pub fn set_standby_bucket(
    client: &AdbClient,
    serial: &str,
    package: &str,
    bucket: &str,
) -> Result<(), AdbError> {
    let arguments =
        ["-s", serial, "shell", "am", "set-standby-bucket", package, bucket];
    client.run(&arguments)?;
    return Ok(());
}
