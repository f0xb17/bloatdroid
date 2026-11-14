//! The Doze Whitelist section.

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

use super::state::WhitelistState;
use super::task::DozeOutcome;
use super::task::DozeTask;

/// The name of the thread that runs the whitelist commands.
const WORKER_THREAD: &str = "doze-whitelist";

/// Shown while the application list is being read.
const LOADING_STATUS: &str = "loading applications";

/// Shown when the worker thread is no longer running.
const WORKER_STATUS: &str = "the Doze whitelist worker has stopped";

/// A whitelist change that waits for the device to confirm it.
struct PendingChange {
    package: String,
    previous: String,
}

/// The Doze Whitelist section.
#[non_exhaustive]
pub struct DozeSection {
    state: SectionState,
    client: AdbClient,
    worker: Worker<DozeTask>,
    serial: Option<String>,
    inventory: Inventory,
    pending: Option<PendingChange>,
    loading: bool,
    loaded: bool,
}

impl DozeSection {
    /// Creates a collapsed section that runs ADB through the given client.
    pub(crate) fn new(client: AdbClient) -> io::Result<Self> {
        let worker = Worker::spawn(WORKER_THREAD)?;
        let section = Self {
            state: SectionState::new(Section::Doze),
            client,
            worker,
            serial: None,
            inventory: Inventory::Absent,
            pending: None,
            loading: false,
            loaded: false,
        };
        return Ok(section);
    }

    /// Moves the selected application between the user whitelist and no
    /// whitelist at all.
    ///
    /// A system whitelist entry has no reachable state, so both arrow keys
    /// leave it exactly as it is.
    fn toggle_state(&mut self, entry: &ControlEntry) -> bool {
        if self.pending.is_some() {
            return false;
        }

        let serial = match self.serial.clone() {
            Some(serial) => serial,
            None => return false,
        };
        let state = match WhitelistState::from_name(&entry.value) {
            Some(state) => state,
            None => return false,
        };
        let target = match state.toggled() {
            Some(target) => target,
            None => return false,
        };

        let task = DozeTask::change(
            self.client.clone(),
            serial,
            entry.identifier.clone(),
            target,
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
        if self.state.set_entry_value(&entry.identifier, target.name()) {
            changed = true;
        }
        return changed;
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
        let task = DozeTask::load(self.client.clone(), serial, packages);
        if self.worker.submit(task).is_err() {
            self.loading = false;
            return self.state.set_status(Some(String::from(WORKER_STATUS)));
        }
        return self.state.set_status(Some(String::from(LOADING_STATUS)));
    }

    /// Applies one finished task.
    fn apply_outcome(&mut self, outcome: DozeOutcome) -> bool {
        if self.serial.as_deref() != Some(outcome.serial()) {
            return false;
        }

        match outcome {
            DozeOutcome::Loaded {
                entries,
                ..
            } => {
                return self.apply_snapshot(entries);
            }
            DozeOutcome::LoadFailed {
                message,
                ..
            } => {
                self.loading = false;
                self.loaded = true;
                return self.state.set_status(Some(message));
            }
            DozeOutcome::Changed {
                package,
                state,
                ..
            } => {
                return self.apply_change(&package, state);
            }
            DozeOutcome::ChangeFailed {
                package,
                message,
                ..
            } => {
                return self.reject_change(&package, message);
            }
        }
    }

    /// Shows a freshly read application list.
    fn apply_snapshot(&mut self, entries: Vec<ControlEntry>) -> bool {
        self.loading = false;
        self.loaded = true;

        let mut changed = self.state.replace_entries(entries);
        if self.state.set_status(None) {
            changed = true;
        }
        return changed;
    }

    /// Shows a verified whitelist change.
    fn apply_change(&mut self, package: &str, state: WhitelistState) -> bool {
        self.pending = None;

        let mut changed = self.state.set_entry_value(package, state.name());
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

impl ControlSection for DozeSection {
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
        match key {
            KeyCode::Left | KeyCode::Right => {
                return self.toggle_state(entry);
            }
            _ => return false,
        }
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
