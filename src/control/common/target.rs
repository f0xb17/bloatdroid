//! The device that the control areas operate on.

/// The device that the control sections operate on.
///
/// The SDK level is part of the target because the values a device accepts
/// depend on its Android version, not only on the control area.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ControlTarget {
    /// The ADB serial number of the device.
    pub serial: String,
    /// The Android SDK level, for example `33`.
    pub sdk_version: String,
}

impl ControlTarget {
    /// Builds a target from a device serial number and its SDK level.
    pub fn new(serial: String, sdk_version: String) -> Self {
        let target = Self {
            serial,
            sdk_version,
        };
        return target;
    }
}
