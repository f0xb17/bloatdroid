//! The `adb shell pm list packages` command.

use super::client::AdbClient;
use super::error::AdbError;

/// Lists the packages installed on a device, sorted and without duplicates.
pub fn list_packages(
    client: &AdbClient,
    serial: &str,
) -> Result<Vec<String>, AdbError> {
    let output =
        client.run(&["-s", serial, "shell", "pm", "list", "packages"])?;
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
