//! The device data that discovery hands to the rest of the application.

/// An online, physical Android device.
///
/// The fields are filled from ADB system properties, so a device only becomes
/// one of these after it answered every question discovery needs.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct AndroidDevice {
    /// The serial number of the device.
    pub identifier: String,
    /// The manufacturer, for example `Google`.
    pub manufacturer: String,
    /// The marketing model name, for example `Pixel 4`.
    pub model: String,
    /// The device codename, for example `flame`.
    pub device: String,
    /// The Android release, for example `13`.
    pub android_version: String,
    /// The Android SDK level, for example `33`.
    pub sdk_version: String,
    /// The build fingerprint reported by the device.
    pub build_fingerprint: String,
    /// The ADB connection state, for example `device`.
    pub state: String,
}
