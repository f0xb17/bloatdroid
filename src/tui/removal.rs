//! The Remove Packages panel: package selection, confirmation, and background
//! removal.
//!
//! The panel lists the application inventory that the control panel already
//! read for the selected device and the global system application setting, so no
//! package list is read twice. Selecting a package never removes it: the user
//! confirms first, and only the confirmation hands the request to the removal
//! worker.
//!
//! The worker runs the user-level uninstall on its own thread, so the event loop
//! keeps drawing and reading input. While one removal is running the panel
//! refuses to start another, so a request cannot be submitted twice.

use std::io;
use std::sync::Arc;

use crossterm::event::KeyCode;

use crate::adb::AdbClient;
use crate::control::common::Worker;
use crate::control::common::WorkerTask;
use crate::discover::AndroidDevice;
use crate::discover::apps::Inventory;
use crate::log::LogLevel;
use crate::log::OperationLog;
use crate::remove::RemovalMethod;
use crate::remove::RemovalOperation;
use crate::remove::RemovalOutcome;
use crate::remove::RemovalPackage;
use crate::remove::UserUninstall;
use crate::remove::validate;

/// The name of the thread that runs the removals.
const REMOVAL_THREAD: &str = "package-removal";

/// Shown when the removal worker is no longer running.
const WORKER_STATUS: &str = "the package removal worker has stopped";

/// Shown when another removal is still running.
const BUSY_STATUS: &str = "An uninstall is already running.";

/// A removal that waits for the user's confirmation.
#[derive(Clone, Debug)]
pub(crate) struct RemovalConfirmation {
    /// The device the package was selected on.
    pub(crate) serial: String,
    /// The package that would be removed.
    pub(crate) package: String,
}

/// A removal that finished on the worker thread.
pub(crate) struct RemovalFinished {
    /// The package the removal targeted.
    pub(crate) package: String,
    /// What the device did.
    pub(crate) outcome: RemovalOutcome,
}

/// Runs one prepared removal on the removal worker thread.
pub(crate) struct RemovalTask {
    operation: RemovalOperation,
    log: Arc<OperationLog>,
}

impl WorkerTask for RemovalTask {
    type Response = RemovalFinished;

    fn execute(self) -> RemovalFinished {
        let package = String::from(self.operation.package().identifier());
        let outcome = self.operation.run(&self.log);
        return RemovalFinished {
            package,
            outcome,
        };
    }
}

/// The packages that can be removed and the removal in progress.
pub(crate) struct RemovePackagesPanel {
    client: AdbClient,
    log: Arc<OperationLog>,
    inventory: Inventory,
    packages: Vec<String>,
    selected: usize,
    confirmation: Option<RemovalConfirmation>,
    pending: Option<String>,
    status: Option<String>,
    refresh_inventory: bool,
    worker: Worker<RemovalTask>,
}

impl RemovePackagesPanel {
    /// Creates the panel and starts its removal worker.
    pub(crate) fn new(
        client: AdbClient,
        log: Arc<OperationLog>,
    ) -> io::Result<Self> {
        let worker = Worker::spawn(REMOVAL_THREAD)?;
        let panel = Self {
            client,
            log,
            inventory: Inventory::Absent,
            packages: Vec::new(),
            selected: 0,
            confirmation: None,
            pending: None,
            status: None,
            refresh_inventory: false,
            worker,
        };
        return Ok(panel);
    }

    /// Adopts the shared application inventory.
    ///
    /// The selection follows the package it was on, so a refresh keeps the user
    /// where they were. Returns `true` when the panel changed.
    pub(crate) fn set_inventory(&mut self, inventory: &Inventory) -> bool {
        if self.inventory == *inventory {
            return false;
        }

        let previous = match self.selected_package() {
            Some(package) => String::from(package),
            None => String::new(),
        };
        self.inventory = inventory.clone();
        self.packages = self.package_names(inventory);
        self.selected = self.index_of(previous.as_str()).unwrap_or_default();
        if self.packages.is_empty() {
            self.selected = 0;
        }
        return true;
    }

    /// Returns the listed packages, sorted by identifier.
    pub(crate) fn packages(&self) -> &[String] {
        return &self.packages;
    }

    /// Returns the position of the selected package.
    pub(crate) fn selected(&self) -> usize {
        return self.selected;
    }

    /// Returns the selected package.
    pub(crate) fn selected_package(&self) -> Option<&str> {
        let package = self.packages.get(self.selected)?;
        return Some(package.as_str());
    }

    /// Returns the message shown while no package can be listed.
    pub(crate) fn placeholder(&self) -> Option<&str> {
        let message = match &self.inventory {
            Inventory::Absent => "No Android device is selected",
            Inventory::Loading => "Loading applications...",
            Inventory::Failed(_) => "Could not load applications",
            Inventory::Loaded(packages) => {
                if packages.is_empty() {
                    "No applications"
                } else {
                    return None;
                }
            }
        };
        return Some(message);
    }

    /// Returns the removal that waits for confirmation.
    pub(crate) fn confirmation(&self) -> Option<&RemovalConfirmation> {
        return self.confirmation.as_ref();
    }

    /// Returns `true` while a removal runs on the worker thread.
    pub(crate) fn is_running(&self) -> bool {
        return self.pending.is_some();
    }

    /// Returns the status line of the panel.
    pub(crate) fn status(&self) -> Option<&str> {
        return self.status.as_deref();
    }

    /// Handles a key press while the panel has focus.
    ///
    /// The arrows move the selection and `ENTER` asks for confirmation. Nothing
    /// is removed from here: the user decides in the confirmation first.
    pub(crate) fn handle_key(
        &mut self,
        key: KeyCode,
        device: Option<&AndroidDevice>,
    ) -> bool {
        if self.confirmation.is_some() {
            return self.handle_confirmation_key(key, device);
        }

        match key {
            KeyCode::Up => return self.move_selection(-1),
            KeyCode::Down => return self.move_selection(1),
            KeyCode::Enter => return self.request_confirmation(device),
            _ => return false,
        }
    }

    /// Applies the removal results that arrived since the last call.
    pub(crate) fn poll(&mut self) -> bool {
        let finished = self.worker.drain();
        if finished.is_empty() {
            return false;
        }

        for result in finished {
            self.pending = None;
            if result.outcome.is_removed() {
                self.refresh_inventory = true;
            }
            self.status =
                Some(format!("{}: {}", result.package, result.outcome));
        }
        return true;
    }

    /// Returns whether the package inventory should be read again, and clears
    /// the request.
    pub(crate) fn take_inventory_refresh(&mut self) -> bool {
        let refresh = self.refresh_inventory;
        self.refresh_inventory = false;
        return refresh;
    }

    /// Moves the selection one package up or down, wrapping around.
    fn move_selection(&mut self, delta: isize) -> bool {
        if self.packages.is_empty() {
            return false;
        }

        let current = self.selected;
        let last = self.packages.len() - 1;
        let next = if delta < 0 {
            if current == 0 {
                last
            } else {
                current - 1
            }
        } else if current >= last {
            0
        } else {
            current + 1
        };
        if next == current {
            return false;
        }

        self.selected = next;
        return true;
    }

    /// Asks the user to confirm the removal of the selected package.
    fn request_confirmation(&mut self, device: Option<&AndroidDevice>) -> bool {
        if self.pending.is_some() {
            self.status = Some(String::from(BUSY_STATUS));
            return true;
        }

        let device = match device {
            Some(device) => device,
            None => {
                self.status =
                    Some(String::from("No Android device is selected."));
                return true;
            }
        };
        let package = match self.selected_package() {
            Some(package) => String::from(package),
            None => return false,
        };

        let confirmation = RemovalConfirmation {
            serial: device.identifier.clone(),
            package,
        };
        self.confirmation = Some(confirmation);
        self.status = None;
        return true;
    }

    /// Handles a key press while the confirmation is open.
    fn handle_confirmation_key(
        &mut self,
        key: KeyCode,
        device: Option<&AndroidDevice>,
    ) -> bool {
        match key {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                return self.confirm(device);
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.confirmation = None;
                self.status = None;
                return true;
            }
            _ => return false,
        }
    }

    /// Validates the confirmed request and hands it to the worker.
    fn confirm(&mut self, device: Option<&AndroidDevice>) -> bool {
        let confirmation = match self.confirmation.take() {
            Some(confirmation) => confirmation,
            None => return false,
        };

        let device = match device {
            Some(device) => device,
            None => {
                self.status =
                    Some(String::from("No Android device is selected."));
                return true;
            }
        };
        if device.identifier != confirmation.serial {
            let message = String::from(
                "The selected device changed, so the uninstall was cancelled.",
            );
            self.log.record(LogLevel::Warning, message.clone());
            self.status = Some(message);
            return true;
        }

        let package = RemovalPackage::new(confirmation.package.as_str());
        let request =
            match validate(Some(device), package, RemovalMethod::UserUninstall)
            {
                Ok(request) => request,
                Err(error) => {
                    let message = format!(
                        "Cannot uninstall {}: {error}",
                        confirmation.package
                    );
                    self.log.record(LogLevel::Error, message.clone());
                    self.status = Some(message);
                    return true;
                }
            };

        let strategy = Box::new(UserUninstall::new(self.client.clone()));
        let operation = match RemovalOperation::new(request, strategy) {
            Ok(operation) => operation,
            Err(error) => {
                self.log.record(LogLevel::Error, error.to_string());
                self.status = Some(error.to_string());
                return true;
            }
        };

        let task = RemovalTask {
            operation,
            log: Arc::clone(&self.log),
        };
        if self.worker.submit(task).is_err() {
            self.status = Some(String::from(WORKER_STATUS));
            return true;
        }

        let message = format!(
            "Uninstalling {} for the current Android user...",
            confirmation.package
        );
        self.status = Some(message);
        self.pending = Some(String::from(confirmation.package.as_str()));
        return true;
    }

    /// Copies the package names of an inventory.
    fn package_names(&self, inventory: &Inventory) -> Vec<String> {
        let mut packages = Vec::new();
        if let Inventory::Loaded(list) = inventory {
            for package in list.iter() {
                packages.push(package.clone());
            }
        }
        return packages;
    }

    /// Returns the position of a package, if it is listed.
    fn index_of(&self, package: &str) -> Option<usize> {
        for (index, listed) in self.packages.iter().enumerate() {
            if listed == package {
                return Some(index);
            }
        }
        return None;
    }
}
