//! Property parsing, normalization, and device classification.

use super::device::AndroidDevice;

/// The property that reports whether the kernel runs under an emulator.
pub const PROP_QEMU: &str = "ro.kernel.qemu";
/// The property that reports the manufacturer.
pub const PROP_MANUFACTURER: &str = "ro.product.manufacturer";
/// The property that reports the model name.
pub const PROP_MODEL: &str = "ro.product.model";
/// The property that reports the device codename.
pub const PROP_DEVICE: &str = "ro.product.device";
/// The property that reports the Android release version.
pub const PROP_ANDROID_VERSION: &str = "ro.build.version.release";
/// The property that reports the Android SDK level.
pub const PROP_SDK_VERSION: &str = "ro.build.version.sdk";
/// The property that reports the build fingerprint.
pub const PROP_BUILD_FINGERPRINT: &str = "ro.build.fingerprint";

/// The system properties discovery collects for one device.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct DeviceProperties {
    /// The value of [`PROP_QEMU`], `1` on emulators.
    pub qemu: String,
    /// The value of [`PROP_MANUFACTURER`].
    pub manufacturer: String,
    /// The value of [`PROP_MODEL`].
    pub model: String,
    /// The value of [`PROP_DEVICE`].
    pub device: String,
    /// The value of [`PROP_ANDROID_VERSION`].
    pub android_version: String,
    /// The value of [`PROP_SDK_VERSION`].
    pub sdk_version: String,
    /// The value of [`PROP_BUILD_FINGERPRINT`].
    pub build_fingerprint: String,
}

/// Normalizes a property value by trimming surrounding whitespace.
pub fn normalize(value: &str) -> String {
    return value.trim().to_string();
}

/// Reports whether a device is an emulator or a simulator.
///
/// The kernel property is authoritative; the serial prefix and the build
/// fingerprint are additional signals, so a renamed emulator image cannot slip
/// through on its serial number alone.
pub fn is_virtual_device(serial: &str, properties: &DeviceProperties) -> bool {
    if properties.qemu == "1" {
        return true;
    }
    if serial.starts_with("emulator-") {
        return true;
    }

    let fingerprint = properties.build_fingerprint.as_str();
    if fingerprint.contains("generic") || fingerprint.contains("emulator") {
        return true;
    }
    if fingerprint.contains("goldfish") || fingerprint.contains("ranchu") {
        return true;
    }
    return false;
}

/// Builds a device from the collected properties.
///
/// Returns `None` when any field that the application needs is missing, so
/// devices that cannot describe themselves are left out of the list.
pub fn build_device(
    identifier: &str,
    state: &str,
    properties: &DeviceProperties,
) -> Option<AndroidDevice> {
    let manufacturer = normalize(&properties.manufacturer);
    let model = normalize(&properties.model);
    let device = normalize(&properties.device);
    let android_version = normalize(&properties.android_version);
    let sdk_version = normalize(&properties.sdk_version);
    let build_fingerprint = normalize(&properties.build_fingerprint);

    if manufacturer.is_empty()
        || model.is_empty()
        || device.is_empty()
        || android_version.is_empty()
        || sdk_version.is_empty()
        || build_fingerprint.is_empty()
    {
        return None;
    }

    let android_device = AndroidDevice {
        identifier: normalize(identifier),
        manufacturer,
        model,
        device,
        android_version,
        sdk_version,
        build_fingerprint,
        state: normalize(state),
    };
    return Some(android_device);
}
