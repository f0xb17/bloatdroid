//! The `adb shell pm list packages` command.

use super::client::AdbClient;
use super::error::AdbError;

/// Lists the packages installed on a device, sorted and without duplicates.
///
/// Without `include_system` the device only reports the packages that the user
/// installed (`pm list packages -3`), which hides the system applications and
/// leaves the filtering to the package manager. With `include_system` the
/// system packages come along.
pub fn list_packages(
    client: &AdbClient,
    serial: &str,
    include_system: bool,
) -> Result<Vec<String>, AdbError> {
    let mut arguments = vec!["-s", serial, "shell", "pm", "list", "packages"];
    if !include_system {
        arguments.push("-3");
    }

    let output = client.run(&arguments)?;
    return Ok(parse_packages(&output));
}

/// Parses the `package:<name>` lines of `pm list packages`.
pub fn parse_packages(output: &str) -> Vec<String> {
    let mut packages = Vec::new();

    for line in output.lines() {
        let name = match line.trim().strip_prefix("package:") {
            Some(name) => name.trim(),
            None => continue,
        };
        if name.is_empty() {
            continue;
        }
        packages.push(String::from(name));
    }

    packages.sort();
    packages.dedup();
    return packages;
}
