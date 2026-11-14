//! The four control areas.

/// One of the control areas shown in the Standby Crates panel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Section {
    /// Android standby buckets.
    Buckets,
    /// AppOps background execution restrictions.
    AppOps,
    /// The Android application inactive state.
    Inactive,
    /// The Doze whitelist.
    Doze,
}

impl Section {
    /// Returns the title that is shown in the section header.
    pub fn title(self) -> &'static str {
        let title = match self {
            Section::Buckets => "Standby Buckets",
            Section::AppOps => "AppOps",
            Section::Inactive => "App Inactive",
            Section::Doze => "Doze Whitelist",
        };
        return title;
    }
}
