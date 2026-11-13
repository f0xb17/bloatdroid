//! Application state and key handling for the terminal interface.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::discover::AndroidDevice;

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

    /// Returns the panel that follows this one in focus order.
    pub fn next(self) -> Self {
        let next_panel = match self {
            Panel::DeviceSelection => Panel::RemovePackages,
            Panel::RemovePackages => Panel::Operations,
            Panel::Operations => Panel::StandbyCrates,
            Panel::StandbyCrates => Panel::DeviceSelection,
        };
        return next_panel;
    }

    /// Returns the panel that precedes this one in focus order.
    pub fn previous(self) -> Self {
        let previous_panel = match self {
            Panel::DeviceSelection => Panel::StandbyCrates,
            Panel::StandbyCrates => Panel::Operations,
            Panel::Operations => Panel::RemovePackages,
            Panel::RemovePackages => Panel::DeviceSelection,
        };
        return previous_panel;
    }
}

/// The application state that the event handling and the rendering share.
#[non_exhaustive]
pub struct App {
    focused: Panel,
    running: bool,
    devices: Vec<AndroidDevice>,
    selected: Option<usize>,
    overlay_open: bool,
    discovery_error: Option<String>,
    refresh: RefreshWorker,
    redraw: bool,
}

impl App {
    /// Creates the application with the first panel focused.
    pub(crate) fn new(refresh: RefreshWorker) -> Self {
        let app = Self {
            focused: Panel::DeviceSelection,
            running: true,
            devices: Vec::new(),
            selected: None,
            overlay_open: false,
            discovery_error: None,
            refresh,
            redraw: true,
        };
        return app;
    }

    /// Returns the panel that currently has focus.
    pub fn focused(&self) -> Panel {
        return self.focused;
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

    /// Handles a key press.
    ///
    /// The global shortcuts are handled here, so they stay available no matter
    /// which panel has focus. Everything else goes to the overlay while it is
    /// open, and to the focused panel otherwise.
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
    fn apply_outcome(&mut self, outcome: ScanOutcome) {
        if self.discovery_error != outcome.error {
            self.discovery_error = outcome.error;
            self.redraw = true;
        }

        if self.devices == outcome.devices {
            return;
        }

        let previous_identifier = self.selected_identifier();
        let previous_index = self.selected;
        self.devices = outcome.devices;
        self.selected =
            self.reconciled_selection(previous_identifier, previous_index);
        self.redraw = true;
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

    /// Passes a key press to the focused panel.
    fn handle_panel_key(&mut self, key: KeyEvent) {
        if self.focused == Panel::DeviceSelection {
            self.handle_device_selection_key(key);
        }
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
