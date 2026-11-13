//! The `adb devices -l` command.

use super::client::AdbClient;
use super::error::AdbError;

/// One line of `adb devices -l` output.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct AdbDeviceEntry {
    /// The serial number or transport identifier of the device.
    pub serial: String,
    /// The connection state, for example `device` or `unauthorized`.
    pub state: String,
    /// The product name reported by ADB, when present.
    pub product: Option<String>,
    /// The model name reported by ADB, when present.
    pub model: Option<String>,
    /// The device codename reported by ADB, when present.
    pub device: Option<String>,
}

/// Runs `adb devices -l` and returns one entry per attached device.
pub fn list_devices(
    client: &AdbClient,
) -> Result<Vec<AdbDeviceEntry>, AdbError> {
    let output = client.run(&["devices", "-l"])?;
    return Ok(parse_devices(&output));
}

/// Parses the output of `adb devices -l`.
pub fn parse_devices(output: &str) -> Vec<AdbDeviceEntry> {
    let mut entries = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with("List of devices")
            || line.starts_with('*')
        {
            continue;
        }

        let mut fields = line.split_whitespace();
        let serial = match fields.next() {
            Some(serial) => serial,
            None => continue,
        };
        let state = match fields.next() {
            Some(state) => state,
            None => continue,
        };

        let mut entry = AdbDeviceEntry {
            serial: serial.to_string(),
            state: state.to_string(),
            product: None,
            model: None,
            device: None,
        };
        for field in fields {
            apply_detail(&mut entry, field);
        }

        entries.push(entry);
    }

    return entries;
}

/// Stores one `key:value` detail of a device line on the entry.
fn apply_detail(entry: &mut AdbDeviceEntry, detail: &str) {
    let (key, value) = match detail.split_once(':') {
        Some(parts) => parts,
        None => return,
    };

    match key {
        "product" => {
            entry.product = Some(value.to_string());
        }
        "model" => {
            entry.model = Some(value.to_string());
        }
        "device" => {
            entry.device = Some(value.to_string());
        }
        _ => {}
    }
}
