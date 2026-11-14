//! One row of a control section.

/// One application and the value that a control area currently has for it.
///
/// The identifier is the stable part: a section uses it to keep the selection
/// and to address the application in ADB commands, while the value changes as
/// the device reports new values.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ControlEntry {
    /// A stable identifier, normally the package name.
    pub identifier: String,
    /// The value that is currently set, for example `active`.
    pub value: String,
}
