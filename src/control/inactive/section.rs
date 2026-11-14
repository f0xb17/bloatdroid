//! The App Inactive section.

use std::io;

use crossterm::event::KeyCode;

use crate::adb::AdbClient;
use crate::control::common::ControlEntry;
use crate::control::common::ControlSection;
use crate::control::common::ControlTarget;
use crate::control::common::Section;
use crate::control::common::SectionState;
use crate::control::common::Worker;

use super::state::InactiveState;
use super::task::InactiveOutcome;
use super::task::InactiveTask;

/// The name of the thread that runs the inactive state commands.
const WORKER_THREAD: &str = "app-inactive";

/// Shown while the application list is being read.
const LOADING_STATUS: &str = "loading applications";

/// Shown when the worker thread is no longer running.
const WORKER_STATUS: &str = "the App Inactive worker has stopped";

/// A state change that waits for the device to confirm it.
struct PendingChange {
    package: String,
    previous: String,
}

/// The App Inactive section.
#[non_exhaustive]
pub struct InactiveSection {
    state: SectionState,
    client: AdbClient,
    worker: Worker<InactiveTask>,
    serial: Option<String>,
    pending: Option<PendingChange>,
    loading: bool,
    loaded: bool,
}

impl InactiveSection {
    /// Creates a collapsed section that runs ADB through the given client.
    pub(crate) fn new(client: AdbClient) -> io::Result<Self> {
        let worker = Worker::spawn(WORKER_THREAD)?;
        let section = Self {
            state: SectionState::new(Section::Inactive),
            client,
            worker,
            serial: None,
            pending: None,
            loading: false,
            loaded: false,
        };
        return Ok(section);
    }

    /// Toggles the inactive state of the selected application.
    ///
    /// An application whose state the device did not report cannot be
    /// toggled, because the opposite of an unknown state is unknown as well.
    fn toggle_state(&mut self, entry: &ControlEntry) -> bool {
        if self.pending.is_some() {
            return false;
        }

        let serial = match self.serial.clone() {
            Some(serial) => serial,
            None => return false,
        };
        let state = match InactiveState::from_name(&entry.value) {
            Some(state) => state.toggled(),
            None => return false,
        };

        let task = InactiveTask::change(
            self.client.clone(),
            serial,
            entry.identifier.clone(),
            state,
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
        if self.state.set_entry_value(&entry.identifier, state.name()) {
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

        self.loading = true;
        let task = InactiveTask::load(self.client.clone(), serial);
        if self.worker.submit(task).is_err() {
            self.loading = false;
            return self.state.set_status(Some(String::from(WORKER_STATUS)));
        }
        return self.state.set_status(Some(String::from(LOADING_STATUS)));
    }

    /// Applies one finished task.
    fn apply_outcome(&mut self, outcome: InactiveOutcome) -> bool {
        if self.serial.as_deref() != Some(outcome.serial()) {
            return false;
        }

        match outcome {
            InactiveOutcome::Loaded {
                entries,
                unreadable,
                ..
            } => {
                return self.apply_snapshot(entries, unreadable);
            }
            InactiveOutcome::LoadFailed {
                message,
                ..
            } => {
                self.loading = false;
                self.loaded = true;
                let status = format!("could not list applications: {message}");
                return self.state.set_status(Some(status));
            }
            InactiveOutcome::Changed {
                package,
                state,
                ..
            } => {
                return self.apply_change(&package, state);
            }
            InactiveOutcome::ChangeFailed {
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

    /// Shows a verified state change.
    fn apply_change(&mut self, package: &str, state: InactiveState) -> bool {
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

    /// Returns `true` when the application list has to be read.
    fn needs_load(&self) -> bool {
        if self.loading || self.loaded {
            return false;
        }
        return self.serial.is_some();
    }
}

impl ControlSection for InactiveSection {
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

        let mut changed = self.state.replace_entries(Vec::new());
        if self.state.set_status(None) {
            changed = true;
        }

        if let Some(target) = target {
            self.serial = Some(target.serial.clone());
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
