//! The Android standby buckets and the ones a device accepts.

/// A standard Android standby bucket.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum StandbyBucket {
    /// Android exempted the application from the bucket restrictions.
    ///
    /// The system assigns this bucket; a user cannot move an application into
    /// it, so it is shown and never changed. It says nothing about whether the
    /// application is a system application, which is a separate question.
    Exempted,
    /// The application is in active use.
    Active,
    /// The application ran recently or is used regularly.
    WorkingSet,
    /// The application is used often.
    Frequent,
    /// The application is used rarely.
    Rare,
    /// The application is restricted to very little background work.
    ///
    /// Android 11 and later only.
    Restricted,
}

impl StandbyBucket {
    /// The buckets a user can move an application into, from the most to the
    /// least active.
    ///
    /// [`StandbyBucket::Exempted`] is missing on purpose: it is read-only, so
    /// cycling never targets it.
    pub const MODIFIABLE: [StandbyBucket; 5] = [
        StandbyBucket::Active,
        StandbyBucket::WorkingSet,
        StandbyBucket::Frequent,
        StandbyBucket::Rare,
        StandbyBucket::Restricted,
    ];

    /// Returns the name that ADB prints and accepts for this bucket.
    pub fn name(self) -> &'static str {
        let name = match self {
            StandbyBucket::Exempted => "exempted",
            StandbyBucket::Active => "active",
            StandbyBucket::WorkingSet => "working_set",
            StandbyBucket::Frequent => "frequent",
            StandbyBucket::Rare => "rare",
            StandbyBucket::Restricted => "restricted",
        };
        return name;
    }

    /// Returns the numeric bucket id that Android reports.
    pub fn id(self) -> u32 {
        let id = match self {
            StandbyBucket::Exempted => 5,
            StandbyBucket::Active => 10,
            StandbyBucket::WorkingSet => 20,
            StandbyBucket::Frequent => 30,
            StandbyBucket::Rare => 40,
            StandbyBucket::Restricted => 45,
        };
        return id;
    }

    /// Reads a bucket from the name that ADB prints.
    pub fn from_name(name: &str) -> Option<Self> {
        let bucket = match name {
            "exempted" => StandbyBucket::Exempted,
            "active" => StandbyBucket::Active,
            "working_set" => StandbyBucket::WorkingSet,
            "frequent" => StandbyBucket::Frequent,
            "rare" => StandbyBucket::Rare,
            "restricted" => StandbyBucket::Restricted,
            _ => return None,
        };
        return Some(bucket);
    }

    /// Reads a bucket from the numeric id that Android reports.
    pub fn from_id(id: u32) -> Option<Self> {
        let bucket = match id {
            5 => StandbyBucket::Exempted,
            10 => StandbyBucket::Active,
            20 => StandbyBucket::WorkingSet,
            30 => StandbyBucket::Frequent,
            40 => StandbyBucket::Rare,
            45 => StandbyBucket::Restricted,
            _ => return None,
        };
        return Some(bucket);
    }

    /// Reads a bucket from what a device reports for one application.
    ///
    /// Android answers `am get-standby-bucket` with the name of the bucket on
    /// some versions and with its numeric id on others, so both forms are
    /// accepted. Anything else stays unknown instead of being rounded to a
    /// valid bucket.
    pub fn parse(value: &str) -> Option<Self> {
        if let Some(bucket) = Self::from_name(value) {
            return Some(bucket);
        }

        let id = match value.parse::<u32>() {
            Ok(id) => id,
            Err(_) => return None,
        };
        return Self::from_id(id);
    }

    /// Returns `true` when a user can move an application into this bucket.
    pub fn is_modifiable(self) -> bool {
        return self != StandbyBucket::Exempted;
    }
}

/// The first Android SDK level that accepts the `restricted` bucket.
const RESTRICTED_MIN_SDK: u32 = 30;

/// The standby buckets that a device accepts.
///
/// Android does not report the allowed buckets per application, so support is
/// decided from the system version instead, and every change is verified by
/// reading the bucket back.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct BucketSupport {
    restricted: bool,
}

impl BucketSupport {
    /// Derives the supported buckets from the SDK level of a device.
    pub fn for_sdk(sdk_version: &str) -> Self {
        let sdk_level = sdk_version.trim().parse::<u32>().unwrap_or_default();
        let support = Self {
            restricted: sdk_level >= RESTRICTED_MIN_SDK,
        };
        return support;
    }

    /// Returns `true` when the device accepts the given bucket.
    pub fn supports(&self, bucket: StandbyBucket) -> bool {
        let supported = match bucket {
            StandbyBucket::Restricted => self.restricted,
            _ => true,
        };
        return supported;
    }

    /// Returns the next supported bucket after `bucket`, if there is one.
    pub fn next(&self, bucket: StandbyBucket) -> Option<StandbyBucket> {
        let mut found = false;

        for candidate in StandbyBucket::MODIFIABLE {
            if found && self.supports(candidate) {
                return Some(candidate);
            }
            if candidate == bucket {
                found = true;
            }
        }
        return None;
    }

    /// Returns the next supported bucket before `bucket`, if there is one.
    pub fn previous(&self, bucket: StandbyBucket) -> Option<StandbyBucket> {
        let mut previous = None;

        for candidate in StandbyBucket::MODIFIABLE {
            if candidate == bucket {
                return previous;
            }
            if self.supports(candidate) {
                previous = Some(candidate);
            }
        }
        return None;
    }
}
