//! Discovery of the physical devices that are ready to use.

use std::cmp::Ordering;

use crate::adb::AdbClient;
use crate::adb::AdbDeviceEntry;
use crate::adb::AdbError;
use crate::adb::get_prop;
use crate::adb::get_state;
use crate::adb::list_devices;

use super::device::AndroidDevice;
use super::parser::DeviceProperties;
use super::parser::PROP_ANDROID_VERSION;
use super::parser::PROP_BUILD_FINGERPRINT;
use super::parser::PROP_DEVICE;
use super::parser::PROP_MANUFACTURER;
use super::parser::PROP_MODEL;
use super::parser::PROP_QEMU;
use super::parser::PROP_SDK_VERSION;
use super::parser::build_device;
use super::parser::is_virtual_device;

/// The ADB state of a device that is online and usable.
const ONLINE_STATE: &str = "device";

/// Discovers connected Android devices through ADB.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct DeviceScanner {
    client: AdbClient,
}

impl DeviceScanner {
    /// Creates a scanner that runs ADB through the given client.
    pub fn new(client: AdbClient) -> Self {
        return Self {
            client,
        };
    }

    /// Returns the online physical devices, sorted by identifier.
    ///
    /// A device is left out when it is offline, unauthorized, an emulator, or
    /// unable to report the properties the application needs. A failure to run
    /// ADB at all is returned as an error.
    pub fn scan(&self) -> Result<Vec<AndroidDevice>, AdbError> {
        let entries = list_devices(&self.client)?;
        let mut devices = Vec::new();

        for entry in entries {
            if entry.state != ONLINE_STATE {
                continue;
            }
            match self.inspect(&entry) {
                Some(device) => devices.push(device),
                None => continue,
            }
        }

        devices.sort_by(compare_identifiers);
        return Ok(devices);
    }

    /// Collects the properties of one device, or `None` when it cannot answer.
    fn inspect(&self, entry: &AdbDeviceEntry) -> Option<AndroidDevice> {
        let properties = self.properties(&entry.serial)?;
        if is_virtual_device(&entry.serial, &properties) {
            return None;
        }

        let state = match get_state(&self.client, &entry.serial) {
            Ok(state) => state,
            Err(_) => return None,
        };
        return build_device(&entry.serial, &state, &properties);
    }

    /// Reads every property discovery needs from one device.
    fn properties(&self, serial: &str) -> Option<DeviceProperties> {
        let properties = DeviceProperties {
            qemu: self.property(serial, PROP_QEMU)?,
            manufacturer: self.property(serial, PROP_MANUFACTURER)?,
            model: self.property(serial, PROP_MODEL)?,
            device: self.property(serial, PROP_DEVICE)?,
            android_version: self.property(serial, PROP_ANDROID_VERSION)?,
            sdk_version: self.property(serial, PROP_SDK_VERSION)?,
            build_fingerprint: self.property(serial, PROP_BUILD_FINGERPRINT)?,
        };
        return Some(properties);
    }

    /// Reads one property, treating a failed read as missing information.
    fn property(&self, serial: &str, name: &str) -> Option<String> {
        match get_prop(&self.client, serial, name) {
            Ok(value) => return Some(value),
            Err(_) => return None,
        }
    }
}

/// Orders devices by their identifier.
fn compare_identifiers(
    left: &AndroidDevice,
    right: &AndroidDevice,
) -> Ordering {
    return left.identifier.cmp(&right.identifier);
}
