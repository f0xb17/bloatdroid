//! The Standby Buckets section.

use std::io;

use crossterm::event::KeyCode;

use crate::adb::AdbClient;
use crate::control::common::ControlEntry;
use crate::control::common::ControlSection;
use crate::control::common::ControlTarget;
use crate::control::common::Inventory;
use crate::control::common::Section;
use crate::control::common::SectionState;
use crate::control::common::Worker;

use super::bucket::BucketSupport;
use super::bucket::StandbyBucket;
use super::task::BucketOutcome;
use super::task::BucketTask;

/// The name of the thread that runs the standby bucket commands.
const WORKER_THREAD: &str = "standby-buckets";

/// Shown while the application list is being read.
const LOADING_STATUS: &str = "loading applications";

/// Shown when the worker thread is no longer running.
const WORKER_STATUS: &str = "the standby bucket worker has stopped";

/// The direction in which a bucket change moves.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Step {
    /// Towards the less active buckets.
    Previous,
    /// Towards the more active buckets.
    Next,
}

/// A bucket change that waits for the device to confirm it.
struct PendingChange {
    package: String,
    previous: String,
}

/// The Standby Buckets section.
#[non_exhaustive]
pub struct BucketsSection {
    state: SectionState,
    client: AdbClient,
    worker: Worker<BucketTask>,
    serial: Option<String>,
    support: BucketSupport,
    inventory: Inventory,
    pending: Option<PendingChange>,
    loading: bool,
    loaded: bool,
}

impl BucketsSection {
    /// Creates a collapsed section that runs ADB through the given client.
    pub(crate) fn new(client: AdbClient) -> io::Result<Self> {
        let worker = Worker::spawn(WORKER_THREAD)?;
        let section = Self {
            state: SectionState::new(Section::Buckets),
            client,
            worker,
            serial: None,
            support: BucketSupport::for_sdk(""),
            inventory: Inventory::Absent,
            pending: None,
            loading: false,
            loaded: false,
        };
        return Ok(section);
    }

    /// Moves the selected application one bucket further.
    fn change_bucket(&mut self, entry: &ControlEntry, step: Step) -> bool {
        if self.pending.is_some() {
            return false;
        }

        let serial = match self.serial.clone() {
            Some(serial) => serial,
            None => return false,
        };
        let bucket = match self.target_bucket(&entry.value, step) {
            Some(bucket) => bucket,
            None => return false,
        };

        let task = BucketTask::change(
            self.client.clone(),
            serial,
            entry.identifier.clone(),
            String::from(bucket.name()),
        );
        if self.worker.submit(task).is_err() {
            return self.state.set_status(Some(String::from(WORKER_STATUS)));
        }

        let pending = PendingChange {
            package: entry.identifier.clone(),
            previous: entry.value.clone(),
        };
        self.pending = Some(pending);

        let mut changed = self.state.set_status(None);
        if self.state.set_entry_value(&entry.identifier, bucket.name()) {
            changed = true;
        }
        return changed;
    }

    /// Returns the bucket that a step leads to from the current value.
    ///
    /// An application whose bucket the device did not report is moved into the
    /// most active bucket, which every device accepts.
    fn target_bucket(
        &self,
        current: &str,
        step: Step,
    ) -> Option<StandbyBucket> {
        let bucket = match StandbyBucket::from_name(current) {
            Some(bucket) => bucket,
            None => return Some(StandbyBucket::Active),
        };

        let target = match step {
            Step::Previous => self.support.previous(bucket),
            Step::Next => self.support.next(bucket),
        };
        return target;
    }

    /// Hands a load task to the worker thread.
    fn request_load(&mut self) -> bool {
        let serial = match self.serial.clone() {
            Some(serial) => serial,
            None => return false,
        };
        let packages = match &self.inventory {
            Inventory::Loaded(packages) => packages.clone(),
            _ => return false,
        };

        self.loading = true;
        let task = BucketTask::load(self.client.clone(), serial, packages);
        if self.worker.submit(task).is_err() {
            self.loading = false;
            return self.state.set_status(Some(String::from(WORKER_STATUS)));
        }
        return self.state.set_status(Some(String::from(LOADING_STATUS)));
    }

    /// Applies one finished task.
    fn apply_outcome(&mut self, outcome: BucketOutcome) -> bool {
        if self.serial.as_deref() != Some(outcome.serial()) {
            return false;
        }

        match outcome {
            BucketOutcome::Loaded {
                entries,
                unreadable,
                ..
            } => {
                return self.apply_snapshot(entries, unreadable);
            }
            BucketOutcome::Changed {
                package,
                bucket,
                ..
            } => {
                return self.apply_change(&package, &bucket);
            }
            BucketOutcome::ChangeFailed {
                package,
                message,
                ..
            } => {
                return self.reject_change(&package, message);
            }
        }
    }

    /// Shows a freshly read application list.
    fn apply_snapshot(
        &mut self,
        entries: Vec<ControlEntry>,
        unreadable: usize,
    ) -> bool {
        self.loading = false;
        self.loaded = true;

        let mut changed = self.state.replace_entries(entries);
        let status = match unreadable {
            0 => None,
            count => Some(format!("{count} applications could not be read")),
        };
        if self.state.set_status(status) {
            changed = true;
        }
        return changed;
    }

    /// Shows a verified bucket change.
    fn apply_change(&mut self, package: &str, bucket: &str) -> bool {
        self.pending = None;

        let mut changed = self.state.set_entry_value(package, bucket);
        if self.state.set_status(None) {
            changed = true;
        }
        return changed;
    }

    /// Reverts an optimistic change that the device did not accept.
    fn reject_change(&mut self, package: &str, message: String) -> bool {
        let mut changed = false;

        if let Some(pending) = self.pending.take()
            && pending.package == package
            && self.state.set_entry_value(&pending.package, &pending.previous)
        {
            changed = true;
        }
        if self.state.set_status(Some(message)) {
            changed = true;
        }
        return changed;
    }

    /// Returns `true` when the application states have to be read.
    ///
    /// The shared package inventory has to be there first: a section never
    /// asks the device for the package list itself.
    fn needs_load(&self) -> bool {
        if self.loading || self.loaded || self.serial.is_none() {
            return false;
        }
        return matches!(self.inventory, Inventory::Loaded(_));
    }
}

impl ControlSection for BucketsSection {
    fn section(&self) -> Section {
        return self.state.section();
    }

    fn state(&self) -> &SectionState {
        return &self.state;
    }

    fn state_mut(&mut self) -> &mut SectionState {
        return &mut self.state;
    }

    fn set_target(&mut self, target: Option<&ControlTarget>) -> bool {
        let same_device = match self.serial.as_deref() {
            Some(current) => match target {
                Some(target) => current == target.serial,
                None => false,
            },
            None => target.is_none(),
        };
        if same_device {
            return false;
        }

        self.serial = None;
        self.support = BucketSupport::for_sdk("");
        self.pending = None;
        self.loading = false;
        self.loaded = false;
        self.inventory = Inventory::Absent;

        let mut changed = self.state.replace_entries(Vec::new());
        if self.state.set_status(None) {
            changed = true;
        }

        if let Some(target) = target {
            self.serial = Some(target.serial.clone());
            self.support = BucketSupport::for_sdk(&target.sdk_version);
        }
        return changed;
    }

    fn set_inventory(&mut self, inventory: Inventory) -> bool {
        self.inventory = inventory;
        self.pending = None;
        self.loading = false;
        self.loaded = false;

        let status = match &self.inventory {
            Inventory::Loading => Some(String::from(LOADING_STATUS)),
            Inventory::Failed(message) => Some(message.clone()),
            _ => None,
        };
        let stale =
            matches!(&self.inventory, Inventory::Absent | Inventory::Failed(_));

        // The entries of a device that is gone or unreadable are dropped. A
        // refresh of the same device keeps them until the new states arrive,
        // so the selected application survives the reload.
        let mut changed = false;
        if stale && self.state.replace_entries(Vec::new()) {
            changed = true;
        }
        if self.state.set_status(status) {
            changed = true;
        }
        return changed;
    }

    fn handle_entry_key(&mut self, key: KeyCode, entry: &ControlEntry) -> bool {
        let step = match key {
            KeyCode::Left => Step::Previous,
            KeyCode::Right => Step::Next,
            _ => return false,
        };
        return self.change_bucket(entry, step);
    }

    fn poll(&mut self) -> bool {
        let outcomes = self.worker.drain();

        let mut changed = false;
        for outcome in outcomes {
            if self.apply_outcome(outcome) {
                changed = true;
            }
        }

        if !self.state.is_expanded() {
            self.loaded = false;
            return changed;
        }
        if !self.needs_load() {
            return changed;
        }
        if self.request_load() {
            changed = true;
        }
        return changed;
    }
}
