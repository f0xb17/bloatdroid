//! The Android standby buckets and the ones a device accepts.

/// A standard Android standby bucket, ordered from the most to the least
/// active.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum StandbyBucket {
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
    /// Every bucket, in the order Android ranks them.
    pub const ALL: [StandbyBucket; 5] = [
        StandbyBucket::Active,
        StandbyBucket::WorkingSet,
        StandbyBucket::Frequent,
        StandbyBucket::Rare,
        StandbyBucket::Restricted,
    ];

    /// Returns the value that ADB prints and accepts for this bucket.
    pub fn name(self) -> &'static str {
        let name = match self {
            StandbyBucket::Active => "active",
            StandbyBucket::WorkingSet => "working_set",
            StandbyBucket::Frequent => "frequent",
            StandbyBucket::Rare => "rare",
            StandbyBucket::Restricted => "restricted",
        };
        return name;
    }

    /// Reads a bucket from the value that ADB prints.
    pub fn from_name(name: &str) -> Option<Self> {
        let bucket = match name {
            "active" => StandbyBucket::Active,
            "working_set" => StandbyBucket::WorkingSet,
            "frequent" => StandbyBucket::Frequent,
            "rare" => StandbyBucket::Rare,
            "restricted" => StandbyBucket::Restricted,
            _ => return None,
        };
        return Some(bucket);
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

        for candidate in StandbyBucket::ALL {
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

        for candidate in StandbyBucket::ALL {
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
