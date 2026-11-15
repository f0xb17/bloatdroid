//! The centralized Operations Log.
//!
//! Every module reports what it did through this log instead of the terminal
//! layer, and the Operations panel renders what the log holds. Storage is
//! bounded and thread-safe, so a control worker can report an event while the
//! event loop reads the log, and neither side waits for an ADB command.
//!
//! A timestamp is the time of day on the clock of the user, so an entry reads
//! the same as the terminal window next to it. The standard library has no time
//! zone support, and the local offset cannot be derived from the system clock
//! alone, so the offset comes from [`chrono`].

mod entry;
mod level;

pub use entry::LogEntry;
pub use level::LogLevel;

use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use chrono::Local;

/// How many entries a log keeps before it drops the oldest ones.
pub const DEFAULT_CAPACITY: usize = 200;

/// The central Operations Log.
///
/// Its handle is cheap to clone: every module gets the same log through an
/// [`std::sync::Arc`], so an event is reported in one place and rendered in
/// another.
#[non_exhaustive]
pub struct OperationLog {
    entries: Mutex<VecDeque<LogEntry>>,
    capacity: usize,
    version: AtomicU64,
}

impl Default for OperationLog {
    fn default() -> Self {
        return Self::new();
    }
}

impl OperationLog {
    /// Creates an empty log with the default capacity.
    pub fn new() -> Self {
        return Self::with_capacity(DEFAULT_CAPACITY);
    }

    /// Creates an empty log that keeps at most `capacity` entries.
    ///
    /// A capacity of zero is raised to one, so recording an entry never throws
    /// it away immediately.
    pub fn with_capacity(capacity: usize) -> Self {
        let log = Self {
            entries: Mutex::new(VecDeque::new()),
            capacity: capacity.max(1),
            version: AtomicU64::new(0),
        };
        return log;
    }

    /// Returns how many entries the log keeps at most.
    pub fn capacity(&self) -> usize {
        return self.capacity;
    }

    /// Returns how many entries the log currently holds.
    pub fn len(&self) -> usize {
        return self.lock().len();
    }

    /// Returns `true` when the log holds no entries.
    pub fn is_empty(&self) -> bool {
        return self.lock().is_empty();
    }

    /// Returns how often the log changed.
    ///
    /// The terminal layer compares versions to redraw only when the content
    /// changed, and to tell whether it is looking at what it already drew.
    pub fn version(&self) -> u64 {
        return self.version.load(Ordering::SeqCst);
    }

    /// Records one message at the given level.
    ///
    /// The timestamp is taken here, so every module reports its events the same
    /// way and no module needs to know the clock. The oldest entry is dropped
    /// once the capacity is reached; the lock is released before the version is
    /// published, so no reader waits for the bookkeeping.
    pub fn record(&self, level: LogLevel, message: impl Into<String>) {
        let entry = LogEntry::new(current_timestamp(), level, message);

        {
            let mut entries = self.lock();
            entries.push_back(entry);
            while entries.len() > self.capacity {
                entries.pop_front();
            }
        }

        self.version.fetch_add(1, Ordering::SeqCst);
    }

    /// Returns a copy of the entries, oldest first.
    ///
    /// The copy is taken under the lock and returned without it, so rendering
    /// never holds the lock while it lays the entries out.
    pub fn snapshot(&self) -> Vec<LogEntry> {
        let mut snapshot = Vec::new();
        let entries = self.lock();
        for entry in entries.iter() {
            snapshot.push(entry.clone());
        }
        return snapshot;
    }

    /// Locks the entries.
    ///
    /// A panic elsewhere must not stop the log from reporting: a poisoned lock
    /// is taken over instead of ignored, so no entry is silently lost.
    fn lock(&self) -> MutexGuard<'_, VecDeque<LogEntry>> {
        let entries = match self.entries.lock() {
            Ok(entries) => entries,
            Err(poisoned) => poisoned.into_inner(),
        };
        return entries;
    }
}

/// Returns the current local time of day as `HH:MM:SS`.
///
/// The offset, including the daylight saving rules of the system, comes from
/// [`chrono::Local`]: the standard library only reports the instant, and the
/// `time` crate refuses to report a local offset while worker threads run,
/// which this application always has.
fn current_timestamp() -> String {
    return Local::now().format("%H:%M:%S").to_string();
}
