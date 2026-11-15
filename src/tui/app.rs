//! Application state and key handling for the terminal interface.

use std::io;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::adb::AdbClient;
use crate::control::ControlPanel;
use crate::control::ControlTarget;
use crate::discover::AndroidDevice;
use crate::discover::DeviceScanner;
use crate::discover::REFRESH_INTERVAL;
use crate::discover::apps::ApplicationFilter;
use crate::log::LogLevel;
use crate::log::OperationLog;

use super::operations::OperationsPanel;
use super::refresh::RefreshWorker;
use super::refresh::ScanOutcome;

/// One of the four panels shown in the terminal interface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Panel {
    /// Selects the Android device to operate on.
    DeviceSelection,
    /// Holds the packages that are queued for removal.
    RemovePackages,
    /// Shows the operations that are available or in progress.
    Operations,
    /// Holds the standby crates of packages.
    StandbyCrates,
}

impl Panel {
    /// Returns the title that is shown in the panel border.
    pub fn title(self) -> &'static str {
        let title = match self {
            Panel::DeviceSelection => "DEVICE SELECTION",
            Panel::RemovePackages => "Remove Packages",
            Panel::Operations => "Operations",
            Panel::StandbyCrates => "Standby Crates",
        };
        return title;
    }
}

/// One target of the global focus navigation.
///
/// The Device Selection panel holds two of them: the device list and the system
/// application checkbox below it. They are separate targets, not separate
/// panels: both stay inside the one bordered panel that the two share.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Focus {
    /// Selects the Android device to operate on.
    DeviceSelection,
    /// Decides whether system applications are part of every application list.
    DeviceSelectionFilter,
    /// Holds the packages that are queued for removal.
    RemovePackages,
    /// Holds the expandable sections of the Standby Crates panel.
    StandbyCrates,
    /// Shows what the operations did.
    Operations,
}

impl Focus {
    /// Returns the focus target that follows this one.
    pub fn next(self) -> Self {
        let next = match self {
            Focus::DeviceSelection => Focus::DeviceSelectionFilter,
            Focus::DeviceSelectionFilter => Focus::RemovePackages,
            Focus::RemovePackages => Focus::StandbyCrates,
            Focus::StandbyCrates => Focus::Operations,
            Focus::Operations => Focus::DeviceSelection,
        };
        return next;
    }

    /// Returns the focus target that precedes this one.
    pub fn previous(self) -> Self {
        let previous = match self {
            Focus::DeviceSelection => Focus::Operations,
            Focus::DeviceSelectionFilter => Focus::DeviceSelection,
            Focus::RemovePackages => Focus::DeviceSelectionFilter,
            Focus::StandbyCrates => Focus::RemovePackages,
            Focus::Operations => Focus::StandbyCrates,
        };
        return previous;
    }

    /// Returns the panel that shows this focus target.
    ///
    /// Both Device Selection targets share one panel, so the panel looks focused
    /// whichever of the two holds the focus.
    pub fn panel(self) -> Panel {
        let panel = match self {
            Focus::DeviceSelection => Panel::DeviceSelection,
            Focus::DeviceSelectionFilter => Panel::DeviceSelection,
            Focus::RemovePackages => Panel::RemovePackages,
            Focus::StandbyCrates => Panel::StandbyCrates,
            Focus::Operations => Panel::Operations,
        };
        return panel;
    }
}

/// The application state that the event handling and the rendering share.
#[non_exhaustive]
pub struct App {
    focused: Focus,
    running: bool,
    devices: Vec<AndroidDevice>,
    selected: Option<usize>,
    overlay_open: bool,
    discovery_error: Option<String>,
    filter: ApplicationFilter,
    control: ControlPanel,
    log: Arc<OperationLog>,
    operations: OperationsPanel,
    refresh: RefreshWorker,
    redraw: bool,
}

impl App {
    /// Creates the application with the device list of the Device Selection
    /// panel focused.
    ///
    /// ADB runs through the given client, so the whole application can be driven
    /// without a device. Device discovery and the control panel start their own
    /// background workers here.
    pub fn new(client: AdbClient) -> io::Result<Self> {
        let scanner = DeviceScanner::new(client.clone());
        let refresh = RefreshWorker::spawn(scanner, REFRESH_INTERVAL)?;
        let control = ControlPanel::new(client)?;

        let log = control.log();
        let operations = OperationsPanel::new(Arc::clone(&log));
        let app = Self {
            focused: Focus::DeviceSelection,
            running: true,
            devices: Vec::new(),
            selected: None,
            overlay_open: false,
            discovery_error: None,
            filter: ApplicationFilter::new(),
            control,
            log,
            operations,
            refresh,
            redraw: true,
        };
        return Ok(app);
    }

    /// Returns the focus target that currently has focus.
    pub fn focused(&self) -> Focus {
        return self.focused;
    }

    /// Returns `true` when the device list of the Device Selection panel has
    /// focus.
    pub fn device_list_is_focused(&self) -> bool {
        return self.focused == Focus::DeviceSelection;
    }

    /// Returns `true` when the system application checkbox of the Device
    /// Selection panel has focus.
    pub fn filter_is_focused(&self) -> bool {
        return self.focused == Focus::DeviceSelectionFilter;
    }

    /// Returns `true` when the sections of the Standby Crates panel have
    /// focus.
    pub fn crates_are_focused(&self) -> bool {
        return self.focused == Focus::StandbyCrates;
    }

    /// Returns `true` when system applications are part of every application
    /// list.
    pub fn show_system(&self) -> bool {
        return self.filter.show_system();
    }

    /// Returns `true` for as long as the application should keep running.
    pub fn is_running(&self) -> bool {
        return self.running;
    }

    /// Returns the discovered devices, sorted by identifier.
    pub fn devices(&self) -> &[AndroidDevice] {
        return &self.devices;
    }

    /// Returns the index of the selected device.
    pub fn selected_index(&self) -> Option<usize> {
        return self.selected;
    }

    /// Returns the selected device.
    pub fn selected_device(&self) -> Option<&AndroidDevice> {
        let index = self.selected?;
        return self.devices.get(index);
    }

    /// Returns `true` when the device information overlay is open.
    pub fn overlay_is_open(&self) -> bool {
        return self.overlay_open;
    }

    /// Returns the error of the latest failed scan, if it failed.
    pub fn discovery_error(&self) -> Option<&str> {
        return self.discovery_error.as_deref();
    }

    /// Returns the control panel shown in the Standby Crates panel.
    pub fn control(&self) -> &ControlPanel {
        return &self.control;
    }

    /// Returns the Operations panel.
    pub fn operations(&self) -> &OperationsPanel {
        return &self.operations;
    }

    /// Tells the Operations panel how much room its rows have.
    ///
    /// The renderer calls this with the width and height it drew the panel in,
    /// so the log layout and the scrolling follow the terminal size.
    pub fn set_operations_viewport(&mut self, width: usize, height: usize) {
        if self.operations.set_viewport(width, height) {
            self.redraw = true;
        }
    }

    /// Tells the control panel how many rows its sections have.
    ///
    /// The renderer calls this with the height the sections have room for,
    /// below the control strip, so the panel scrolls to keep the cursor visible
    /// whatever the terminal size is.
    pub fn set_control_viewport(&mut self, height: usize) {
        if self.control.set_viewport(height) {
            self.redraw = true;
        }
    }

    /// Returns `true` when the interface has to be drawn again.
    pub fn needs_redraw(&self) -> bool {
        return self.redraw;
    }

    /// Marks the current state as drawn.
    pub fn clear_redraw(&mut self) {
        self.redraw = false;
    }

    /// Applies every discovery result that arrived since the last check.
    pub fn poll_discovery(&mut self) {
        let outcomes = self.refresh.drain();
        for outcome in outcomes {
            self.apply_outcome(outcome);
        }
    }

    /// Applies the control work that finished in the background.
    pub fn poll_control(&mut self) {
        if self.control.poll() {
            self.redraw = true;
        }
    }

    /// Applies the log entries that were recorded since the last check.
    pub fn poll_operations(&mut self) {
        if self.operations.poll() {
            self.redraw = true;
        }
    }

    /// Handles a key press.
    ///
    /// The global shortcuts are handled here, so they stay available no matter
    /// which panel or which control has focus. Everything else goes to the
    /// device information overlay while it is open, and to the focused part of
    /// the interface otherwise.
    pub fn handle_key(&mut self, key: KeyEvent) {
        if self.overlay_open {
            self.handle_overlay_key(key);
            return;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.request_quit();
            }
            KeyCode::Char('c')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.request_quit();
            }
            KeyCode::Tab => {
                self.focus_next();
            }
            KeyCode::BackTab => {
                self.focus_previous();
            }
            _ => {
                self.handle_panel_key(key);
            }
        }
    }

    /// Stops the application the next time the event loop checks its state.
    pub fn request_quit(&mut self) {
        self.running = false;
    }

    /// Applies a discovery result.
    ///
    /// Only a change is reported: a periodic pass that finds the same devices
    /// and no error says nothing.
    fn apply_outcome(&mut self, outcome: ScanOutcome) {
        if self.discovery_error != outcome.error {
            match &outcome.error {
                Some(message) => {
                    let message = format!("Device discovery failed: {message}");
                    self.log.record(LogLevel::Error, message);
                }
                None => {
                    let message = String::from("Device discovery recovered.");
                    self.log.record(LogLevel::Info, message);
                }
            }
            self.discovery_error = outcome.error;
            self.redraw = true;
        }

        if self.devices == outcome.devices {
            return;
        }

        let previous_identifier = self.selected_identifier();
        let previous_index = self.selected;
        let previous = std::mem::replace(&mut self.devices, outcome.devices);
        self.selected =
            self.reconciled_selection(previous_identifier, previous_index);
        self.log_device_changes(&previous);
        self.redraw = true;
        self.sync_control_target();
    }

    /// Logs the devices that connected or disconnected since the last scan.
    fn log_device_changes(&mut self, previous: &[AndroidDevice]) {
        for device in &self.devices {
            if has_identifier(previous, device.identifier.as_str()) {
                continue;
            }
            let message = format!("Device {} connected.", device.identifier);
            self.log.record(LogLevel::Info, message);
        }

        for device in previous {
            if has_identifier(&self.devices, device.identifier.as_str()) {
                continue;
            }
            let message = format!("Device {} disconnected.", device.identifier);
            self.log.record(LogLevel::Warning, message);
        }

        if self.devices.is_empty() && !previous.is_empty() {
            let message = String::from("No Android devices are connected.");
            self.log.record(LogLevel::Warning, message);
        }
    }

    /// Returns the identifier of the selected device.
    fn selected_identifier(&self) -> Option<String> {
        let device = self.selected_device()?;
        return Some(device.identifier.clone());
    }

    /// Returns the index of the device with the given identifier, if present.
    fn kept_index(&self, identifier: Option<String>) -> Option<usize> {
        let identifier = identifier?;
        return self.index_of(identifier.as_str());
    }

    /// Keeps the selected device when it is still present, otherwise selects
    /// the device at the previous position, or clears the selection.
    fn reconciled_selection(
        &self,
        identifier: Option<String>,
        previous_index: Option<usize>,
    ) -> Option<usize> {
        if let Some(index) = self.kept_index(identifier) {
            return Some(index);
        }
        if self.devices.is_empty() {
            return None;
        }

        let requested = previous_index.unwrap_or_default();
        let last = self.devices.len() - 1;
        return Some(requested.min(last));
    }

    /// Returns the index of a device with the given identifier.
    fn index_of(&self, identifier: &str) -> Option<usize> {
        for (index, device) in self.devices.iter().enumerate() {
            if device.identifier == identifier {
                return Some(index);
            }
        }
        return None;
    }

    /// Moves the focus to the next panel.
    fn focus_next(&mut self) {
        self.focused = self.focused.next();
        self.redraw = true;
    }

    /// Moves the focus to the previous panel.
    fn focus_previous(&mut self) {
        self.focused = self.focused.previous();
        self.redraw = true;
    }

    /// Handles a key press while the overlay is open.
    ///
    /// `ESC` closes the overlay instead of ending the application, the global
    /// exit shortcuts still work, and every other key is consumed.
    fn handle_overlay_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.overlay_open = false;
                self.redraw = true;
            }
            KeyCode::Char('q') => {
                self.request_quit();
            }
            KeyCode::Char('c')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.request_quit();
            }
            _ => {}
        }
    }

    /// Passes a key press to the focused part of the interface.
    fn handle_panel_key(&mut self, key: KeyEvent) {
        match self.focused {
            Focus::DeviceSelection => {
                self.handle_device_selection_key(key);
                self.sync_control_target();
            }
            Focus::DeviceSelectionFilter => {
                self.handle_filter_key(key);
            }
            Focus::RemovePackages => {}
            Focus::StandbyCrates => {
                self.handle_control_key(key);
            }
            Focus::Operations => {
                if self.operations.handle_key(key.code) {
                    self.redraw = true;
                }
            }
        }
    }

    /// Handles a key press while the system application checkbox has focus.
    ///
    /// Only SPACE reacts. The keys of the device list never reach the checkbox
    /// from here, so it can neither move the device selection nor open the
    /// device information overlay.
    fn handle_filter_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char(' ') {
            self.toggle_filter();
        }
    }

    /// Toggles the global system application setting.
    ///
    /// The setting decides which applications every list shows, so the control
    /// panel reads the package inventory of the selected device again.
    fn toggle_filter(&mut self) {
        self.filter.toggle();

        let message = if self.filter.show_system() {
            String::from("Showing system applications.")
        } else {
            String::from("Hiding system applications.")
        };
        self.log.record(LogLevel::Info, message);

        self.control.set_filter(self.filter);
        self.redraw = true;
    }

    /// Handles a key press for the Standby Crates panel.
    fn handle_control_key(&mut self, key: KeyEvent) {
        if self.control.handle_key(key.code) {
            self.redraw = true;
        }
    }

    /// Keeps the control panel pointed at the selected device.
    fn sync_control_target(&mut self) {
        let target = self.control_target();
        if self.control.set_target(target) {
            self.redraw = true;
        }
    }

    /// Returns the control target of the selected device.
    fn control_target(&self) -> Option<ControlTarget> {
        let device = self.selected_device()?;
        let target = ControlTarget::new(
            device.identifier.clone(),
            device.sdk_version.clone(),
        );
        return Some(target);
    }

    /// Handles a key press for the device selection panel.
    fn handle_device_selection_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => {
                self.select_previous();
            }
            KeyCode::Down => {
                self.select_next();
            }
            KeyCode::Char(' ') => {
                self.open_overlay();
            }
            _ => {}
        }
    }

    /// Selects the next device, wrapping around at the end.
    fn select_next(&mut self) {
        if self.devices.is_empty() {
            return;
        }

        let current = self.selected.unwrap_or_default();
        let last = self.devices.len() - 1;
        let next = if current >= last {
            0
        } else {
            current + 1
        };
        self.selected = Some(next);
        self.redraw = true;
        self.log_selected_device();
    }

    /// Selects the previous device, wrapping around at the start.
    fn select_previous(&mut self) {
        if self.devices.is_empty() {
            return;
        }

        let current = self.selected.unwrap_or_default();
        let previous = if current == 0 {
            self.devices.len() - 1
        } else {
            current - 1
        };
        self.selected = Some(previous);
        self.redraw = true;
        self.log_selected_device();
    }

    /// Logs the device that the user selected.
    fn log_selected_device(&self) {
        let device = match self.selected_device() {
            Some(device) => device,
            None => return,
        };
        let message = format!("Selected device {}.", device.identifier);
        self.log.record(LogLevel::Info, message);
    }

    /// Opens the device information overlay for the selected device.
    fn open_overlay(&mut self) {
        if self.selected_device().is_none() {
            return;
        }

        self.overlay_open = true;
        self.redraw = true;
    }
}

/// Returns `true` when the devices hold the given identifier.
fn has_identifier(devices: &[AndroidDevice], identifier: &str) -> bool {
    for device in devices {
        if device.identifier == identifier {
            return true;
        }
    }
    return false;
}
