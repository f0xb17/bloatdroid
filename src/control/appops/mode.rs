//! The AppOps modes and the order in which they cycle.

/// The mode of one AppOps operation.
///
/// These are the three modes Android prints and accepts for
/// `RUN_ANY_IN_BACKGROUND`: background execution is allowed, Android's own
/// default applies, or background execution is ignored.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AppOpMode {
    /// Background execution is allowed.
    Allow,
    /// Android's default behaviour applies.
    Default,
    /// Background execution is restricted.
    Ignore,
}

impl AppOpMode {
    /// Returns the value that Android prints and accepts for this mode.
    pub fn name(self) -> &'static str {
        let name = match self {
            AppOpMode::Allow => "allow",
            AppOpMode::Default => "default",
            AppOpMode::Ignore => "ignore",
        };
        return name;
    }

    /// Reads a mode from the value that Android prints.
    ///
    /// Only the three supported modes are recognized. Any other value,
    /// including one that a later Android release might add, yields [`None`]
    /// so the user sees that the mode is unknown instead of a guess.
    pub fn from_name(name: &str) -> Option<Self> {
        let mode = match name {
            "allow" => AppOpMode::Allow,
            "default" => AppOpMode::Default,
            "ignore" => AppOpMode::Ignore,
            _ => return None,
        };
        return Some(mode);
    }

    /// Returns the mode that follows this one: `allow`, `default`, `ignore`,
    /// and back to `allow`.
    pub fn next(self) -> Self {
        let mode = match self {
            AppOpMode::Allow => AppOpMode::Default,
            AppOpMode::Default => AppOpMode::Ignore,
            AppOpMode::Ignore => AppOpMode::Allow,
        };
        return mode;
    }

    /// Returns the mode that precedes this one: `allow`, `ignore`,
    /// `default`, and back to `allow`.
    pub fn previous(self) -> Self {
        let mode = match self {
            AppOpMode::Allow => AppOpMode::Ignore,
            AppOpMode::Default => AppOpMode::Allow,
            AppOpMode::Ignore => AppOpMode::Default,
        };
        return mode;
    }
}
