//! The Standby Crates panel: four expandable sections and one cursor.
//!
//! The panel expands and collapses sections, moves the cursor through the
//! headers and entries that are currently visible, and forwards a key press to
//! the section that owns the selected entry. It does not know what any section
//! actually does.
//!
//! The panel also owns what the sections share: the package inventory of the
//! selected device, which is read once with the global system application filter
//! and handed to every section, and the scroll position, which keeps the cursor
//! inside the visible area of the terminal.
//!
//! The system application filter is application-wide state, so the panel never
//! changes it on its own. It applies the filter it is handed and reads the
//! inventory again whenever that filter differs from the one the inventory was
//! read with.

use std::io;
use std::sync::Arc;

use crossterm::event::KeyCode;

use crate::adb::AdbClient;
use crate::discover::apps::ApplicationFilter;
use crate::discover::apps::Inventory;
use crate::log::LogLevel;
use crate::log::OperationLog;

use super::appops::AppOpsSection;
use super::buckets::BucketsSection;
use super::common::ControlEntry;
use super::common::ControlLine;
use super::common::ControlSection;
use super::common::ControlTarget;
use super::common::Section;
use super::common::SectionState;
use super::common::Worker;
use super::common::inventory::InventoryOutcome;
use super::common::inventory::InventoryTask;
use super::doze::DozeSection;
use super::inactive::InactiveSection;

/// The message shown for an expanded section without entries.
const NO_ENTRIES: &str = "no applications";

/// The name of the thread that reads the installed package list.
const INVENTORY_THREAD: &str = "package-inventory";

/// Shown when the inventory thread is no longer running.
const INVENTORY_STATUS: &str = "the package inventory worker has stopped";

/// One position the cursor can occupy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Slot {
    /// The header of a section.
    Header(Section),
    /// One entry of an expanded section.
    Entry {
        /// The section the entry belongs to.
        section: Section,
        /// The position of the entry inside the section.
        entry: usize,
    },
}

/// The four control sections shown in the Standby Crates panel.
#[non_exhaustive]
pub struct ControlPanel {
    sections: Vec<Box<dyn ControlSection>>,
    cursor: Slot,
    target: Option<ControlTarget>,
    filter: ApplicationFilter,
    client: AdbClient,
    inventory: Inventory,
    inventory_requested: bool,
    revision: u64,
    inventory_worker: Worker<InventoryTask>,
    viewport: Option<usize>,
    offset: usize,
    log: Arc<OperationLog>,
}

impl ControlPanel {
    /// Creates the panel with every section collapsed.
    pub fn new(client: AdbClient) -> io::Result<Self> {
        let log = Arc::new(OperationLog::new());
        let sections: Vec<Box<dyn ControlSection>> = vec![
            Box::new(BucketsSection::new(client.clone(), Arc::clone(&log))?),
            Box::new(AppOpsSection::new(client.clone(), Arc::clone(&log))?),
            Box::new(InactiveSection::new(client.clone(), Arc::clone(&log))?),
            Box::new(DozeSection::new(client.clone(), Arc::clone(&log))?),
        ];
        let inventory_worker = Worker::spawn(INVENTORY_THREAD)?;

        let panel = Self {
            sections,
            cursor: Slot::Header(Section::Buckets),
            target: None,
            filter: ApplicationFilter::new(),
            client,
            inventory: Inventory::Absent,
            inventory_requested: false,
            revision: 0,
            inventory_worker,
            viewport: None,
            offset: 0,
            log,
        };
        return Ok(panel);
    }

    /// Returns the Operations Log that the sections report to.
    pub fn log(&self) -> Arc<OperationLog> {
        return Arc::clone(&self.log);
    }

    /// Points the panel at the device that the sections operate on.
    ///
    /// The inventory of the previous device is dropped immediately, so nothing
    /// from an old device stays on screen or is reused. Returns `true` when the
    /// panel changed.
    pub fn set_target(&mut self, target: Option<ControlTarget>) -> bool {
        if self.target == target {
            return false;
        }

        let mut changed = false;
        for section in &mut self.sections {
            if section.set_target(target.as_ref()) {
                changed = true;
            }
        }

        self.target = target;
        // A read that was started for the previous device is outdated, even
        // when the user comes back to the same device later.
        self.next_revision();
        self.inventory_requested = false;
        if self.publish_inventory(Inventory::Absent) {
            changed = true;
        }
        if self.normalize_cursor() {
            changed = true;
        }
        if self.scroll_to_cursor() {
            changed = true;
        }
        return changed;
    }

    /// Reads the installed packages of the selected device again.
    ///
    /// A filter change and a refresh on request share this path. Every section
    /// rescans its own state against the new list. Returns `true` when the panel
    /// changed.
    pub fn refresh_inventory(&mut self) -> bool {
        self.inventory_requested = false;
        return self.request_inventory();
    }

    /// Applies the global system application setting.
    ///
    /// The setting decides which applications every section lists, so a change
    /// reads the package inventory again with the new setting. The setting
    /// belongs to the application: the panel never changes it on its own, it
    /// only applies the value it is handed. Returns `true` when the panel
    /// changed.
    pub fn set_filter(&mut self, filter: ApplicationFilter) -> bool {
        if self.filter == filter {
            return false;
        }
        self.filter = filter;
        return self.request_inventory();
    }

    /// Tells the panel how many lines the sections have room for.
    ///
    /// The renderer calls this with the rows of the Standby Crates panel that
    /// the control strip does not take, so the scroll position follows the
    /// terminal size. Returns `true` when the visible lines changed.
    pub fn set_viewport(&mut self, height: usize) -> bool {
        let previous = self.viewport;
        self.viewport = Some(height);
        let moved = self.scroll_to_cursor();
        return previous != Some(height) || moved;
    }

    /// Handles a key press while the sections of the panel have focus.
    ///
    /// `SPACE` expands or collapses the focused section, and the arrows move
    /// the cursor and change the value of the selected entry. No key from here
    /// changes the system application setting, which belongs to the
    /// application. Returns `true` when the panel changed.
    pub fn handle_key(&mut self, key: KeyCode) -> bool {
        let mut changed = self.normalize_cursor();

        let handled = match key {
            KeyCode::Up => self.move_up(),
            KeyCode::Down => self.move_down(),
            KeyCode::Char(' ') => self.toggle_focused_section(),
            KeyCode::Left | KeyCode::Right => self.handle_entry_key(key),
            _ => false,
        };

        if handled {
            changed = true;
        }
        if self.scroll_to_cursor() {
            changed = true;
        }
        return changed;
    }

    /// Applies the background work that finished since the last call.
    ///
    /// The inventory is read first, so a section that starts scanning in the
    /// same call already works with the package list of the selected device.
    /// Returns `true` when the panel changed.
    pub fn poll(&mut self) -> bool {
        let selected = self.selected_identifier();

        let mut changed = false;
        if self.needs_inventory() && self.request_inventory() {
            changed = true;
        }

        let outcomes = self.inventory_worker.drain();
        for outcome in outcomes {
            if let Some(inventory) = self.inventory_after(outcome)
                && self.publish_inventory(inventory)
            {
                changed = true;
            }
        }

        for section in &mut self.sections {
            if section.poll() {
                changed = true;
            }
        }

        if changed {
            self.restore_cursor(selected);
        }
        if self.scroll_to_cursor() {
            changed = true;
        }
        return changed;
    }

    /// Returns the lines that belong to the visible area of the panel.
    ///
    /// Nothing outside the viewport is built into a widget: the renderer draws
    /// at most as many rows as the terminal has room for. `content_focused`
    /// decides whether the cursor mark is shown: the sections are a focus
    /// target of their own, so their mark disappears while another part of the
    /// interface has the focus. The scroll position never depends on it.
    pub fn lines(&self, content_focused: bool) -> Vec<ControlLine> {
        let lines = self.full_lines();

        let mut visible = match self.viewport {
            Some(height) if height > 0 => self.visible_lines(&lines, height),
            Some(_) => Vec::new(),
            None => lines,
        };
        if !content_focused {
            for line in &mut visible {
                line.release_focus();
            }
        }
        return visible;
    }

    /// Returns the part of the lines that a viewport of the given height shows.
    fn visible_lines(
        &self,
        lines: &[ControlLine],
        height: usize,
    ) -> Vec<ControlLine> {
        let start = self.offset.min(lines.len());
        let end = lines.len().min(start.saturating_add(height));

        let mut visible = Vec::new();
        for index in start..end {
            if let Some(line) = lines.get(index) {
                visible.push(line.clone());
            }
        }
        return visible;
    }

    /// Returns every line of the panel, including the ones that are scrolled
    /// out of view.
    fn full_lines(&self) -> Vec<ControlLine> {
        let mut lines = Vec::new();

        for control in &self.sections {
            let state = control.state();
            let expanded = state.is_expanded();
            let focused = self.cursor == Slot::Header(state.section());
            let header =
                ControlLine::header(state.section().title(), expanded, focused);
            lines.push(header);

            if !expanded {
                continue;
            }
            self.push_entries(&mut lines, state);
            self.push_status(&mut lines, state);
        }

        return lines;
    }

    /// Appends the entry lines of an expanded section.
    fn push_entries(&self, lines: &mut Vec<ControlLine>, state: &SectionState) {
        for index in 0..state.entries().len() {
            let entry = match state.entry(index) {
                Some(entry) => entry,
                None => continue,
            };
            let slot = Slot::Entry {
                section: state.section(),
                entry: index,
            };
            lines.push(ControlLine::entry(entry, self.cursor == slot));
        }
    }

    /// Appends the status line of an expanded section.
    fn push_status(&self, lines: &mut Vec<ControlLine>, state: &SectionState) {
        let status = match state.status() {
            Some(status) => status,
            None => {
                if !state.entries().is_empty() {
                    return;
                }
                NO_ENTRIES
            }
        };
        lines.push(ControlLine::status(status));
    }

    /// Scrolls the visible area just enough to keep the cursor inside it.
    ///
    /// Returns `true` when the visible lines moved.
    fn scroll_to_cursor(&mut self) -> bool {
        let height = match self.viewport {
            Some(height) if height > 0 => height,
            _ => {
                let moved = self.offset != 0;
                self.offset = 0;
                return moved;
            }
        };

        let lines = self.full_lines();
        let previous = self.offset;
        let focused = Self::focused_line(&lines).unwrap_or_default();

        if focused < self.offset {
            self.offset = focused;
        } else if focused >= self.offset.saturating_add(height) {
            self.offset = focused.saturating_add(1).saturating_sub(height);
        }

        let maximum = lines.len().saturating_sub(height);
        if self.offset > maximum {
            self.offset = maximum;
        }
        return previous != self.offset;
    }

    /// Returns the position of the line that holds the cursor.
    fn focused_line(lines: &[ControlLine]) -> Option<usize> {
        for (index, line) in lines.iter().enumerate() {
            if line.is_focused() {
                return Some(index);
            }
        }
        return None;
    }

    /// Starts reading the package list when the selected device has none yet.
    fn needs_inventory(&self) -> bool {
        if self.inventory_requested || self.target.is_none() {
            return false;
        }
        return matches!(self.inventory, Inventory::Absent);
    }

    /// Hands an inventory task to the inventory worker.
    ///
    /// Only the first load of a device is announced: a later load is part of a
    /// refresh that the user asked for, and its result is reported anyway.
    fn request_inventory(&mut self) -> bool {
        let serial = match &self.target {
            Some(target) => target.serial.clone(),
            None => return false,
        };

        if matches!(self.inventory, Inventory::Absent) {
            let message = String::from("Loading the application inventory.");
            self.log.record(LogLevel::Info, message);
        }

        self.inventory_requested = true;
        let revision = self.next_revision();
        let task = InventoryTask::load(
            self.client.clone(),
            serial,
            self.filter,
            revision,
        );
        if self.inventory_worker.submit(task).is_err() {
            let message = String::from(INVENTORY_STATUS);
            return self.publish_inventory(Inventory::Failed(message));
        }
        return self.publish_inventory(Inventory::Loading);
    }

    /// Bumps and returns the revision of the newest inventory request.
    fn next_revision(&mut self) -> u64 {
        self.revision += 1;
        return self.revision;
    }

    /// Turns an inventory outcome into the inventory to publish.
    ///
    /// The outcome of a device that is no longer selected, and the outcome of a
    /// request that a newer device or filter change overtook, are dropped, so an
    /// answer that no longer fits the panel never replaces what is shown.
    fn inventory_after(&self, outcome: InventoryOutcome) -> Option<Inventory> {
        if outcome.revision() != self.revision {
            return None;
        }
        let serial = self.target.as_ref()?.serial.as_str();
        if outcome.serial() != serial {
            return None;
        }

        let inventory = match outcome {
            InventoryOutcome::Loaded {
                packages,
                ..
            } => Inventory::Loaded(packages),
            InventoryOutcome::Failed {
                message,
                ..
            } => Inventory::Failed(message),
        };
        return Some(inventory);
    }

    /// Hands one inventory to every section and remembers it.
    ///
    /// A loaded or failed inventory is an answer to a load that the user can
    /// see the result of, so it is logged; a pending and an absent inventory
    /// say nothing on their own.
    fn publish_inventory(&mut self, inventory: Inventory) -> bool {
        let mut changed = false;
        for section in &mut self.sections {
            if section.set_inventory(inventory.clone()) {
                changed = true;
            }
        }
        self.inventory = inventory;

        match &self.inventory {
            Inventory::Loaded(packages) => {
                let message = found_applications(packages.len());
                self.log.record(LogLevel::Success, message);
            }
            Inventory::Failed(message) => {
                let message = format!(
                    "Could not load the application inventory: {message}"
                );
                self.log.record(LogLevel::Error, message);
            }
            Inventory::Absent | Inventory::Loading => {}
        }
        return changed;
    }

    /// Moves the cursor one visible line up.
    fn move_up(&mut self) -> bool {
        let slots = self.slots();
        let current = match Self::slot_index(&slots, self.cursor) {
            Some(current) => current,
            None => return false,
        };
        if current == 0 {
            return false;
        }

        let target = match slots.get(current - 1) {
            Some(slot) => *slot,
            None => return false,
        };
        return self.focus(target);
    }

    /// Moves the cursor one visible line down.
    fn move_down(&mut self) -> bool {
        let slots = self.slots();
        let current = match Self::slot_index(&slots, self.cursor) {
            Some(current) => current,
            None => return false,
        };

        let target = match slots.get(current + 1) {
            Some(slot) => *slot,
            None => return false,
        };
        return self.focus(target);
    }

    /// Expands or collapses the section that holds the cursor.
    fn toggle_focused_section(&mut self) -> bool {
        let section = match self.cursor_section() {
            Some(section) => section,
            None => return false,
        };
        let control = match self.find_mut(section) {
            Some(control) => control,
            None => return false,
        };
        let expanded = control.state_mut().toggle_expanded();

        if !expanded && self.cursor != Slot::Header(section) {
            self.cursor = Slot::Header(section);
        }
        return true;
    }

    /// Forwards a key press to the section of the selected entry.
    fn handle_entry_key(&mut self, key: KeyCode) -> bool {
        let (section, index) = match self.cursor {
            Slot::Entry {
                section,
                entry,
            } => (section, entry),
            Slot::Header(_) => return false,
        };

        let entry = match self.entry_copy(section, index) {
            Some(entry) => entry,
            None => return false,
        };
        let control = match self.find_mut(section) {
            Some(control) => control,
            None => return false,
        };
        return control.handle_entry_key(key, &entry);
    }

    /// Returns every position that is currently visible, in display order.
    fn slots(&self) -> Vec<Slot> {
        let mut slots = Vec::new();

        for control in &self.sections {
            let state = control.state();
            slots.push(Slot::Header(state.section()));

            if !state.is_expanded() {
                continue;
            }
            for index in 0..state.entries().len() {
                slots.push(Slot::Entry {
                    section: state.section(),
                    entry: index,
                });
            }
        }
        return slots;
    }

    /// Moves the cursor back to its section header when its position is no
    /// longer visible.
    fn normalize_cursor(&mut self) -> bool {
        if Self::slot_index(&self.slots(), self.cursor).is_some() {
            return false;
        }

        let section = match self.cursor_section() {
            Some(section) => section,
            None => return false,
        };
        return self.focus(Slot::Header(section));
    }

    /// Keeps the cursor on the same entry after the entries changed.
    fn restore_cursor(&mut self, selected: Option<String>) -> bool {
        let (section, index) = match self.cursor {
            Slot::Entry {
                section,
                entry,
            } => (section, entry),
            Slot::Header(_) => return false,
        };

        let count = self.entry_count(section);
        if count == 0 {
            return self.focus(Slot::Header(section));
        }

        if let Some(identifier) = selected
            && let Some(found) = self.entry_index(section, &identifier)
        {
            return self.focus(Slot::Entry {
                section,
                entry: found,
            });
        }

        let clamped = index.min(count - 1);
        return self.focus(Slot::Entry {
            section,
            entry: clamped,
        });
    }

    /// Returns the identifier of the entry that holds the cursor.
    fn selected_identifier(&self) -> Option<String> {
        let (section, index) = match self.cursor {
            Slot::Entry {
                section,
                entry,
            } => (section, entry),
            Slot::Header(_) => return None,
        };

        let control = self.find(section)?;
        let entry = control.state().entry(index)?;
        return Some(entry.identifier.clone());
    }

    /// Returns the section the cursor belongs to, if it belongs to one.
    fn cursor_section(&self) -> Option<Section> {
        let section = match self.cursor {
            Slot::Header(section) => section,
            Slot::Entry {
                section,
                ..
            } => section,
        };
        return Some(section);
    }

    /// Returns a copy of one entry of a section.
    fn entry_copy(
        &self,
        section: Section,
        index: usize,
    ) -> Option<ControlEntry> {
        let control = self.find(section)?;
        let entry = control.state().entry(index)?;
        return Some(entry.clone());
    }

    /// Returns how many entries a section has.
    fn entry_count(&self, section: Section) -> usize {
        let control = match self.find(section) {
            Some(control) => control,
            None => return 0,
        };
        return control.state().entries().len();
    }

    /// Returns the position of an entry, identified by its identifier.
    fn entry_index(&self, section: Section, identifier: &str) -> Option<usize> {
        let control = self.find(section)?;

        for (index, entry) in control.state().entries().iter().enumerate() {
            if entry.identifier == identifier {
                return Some(index);
            }
        }
        return None;
    }

    /// Puts the cursor on a position and reports whether it moved.
    fn focus(&mut self, slot: Slot) -> bool {
        if self.cursor == slot {
            return false;
        }
        self.cursor = slot;
        return true;
    }

    /// Returns the section with the given identity.
    fn find(&self, section: Section) -> Option<&dyn ControlSection> {
        for control in &self.sections {
            if control.section() == section {
                return Some(control.as_ref());
            }
        }
        return None;
    }

    /// Returns the section with the given identity for modification.
    fn find_mut(
        &mut self,
        section: Section,
    ) -> Option<&mut dyn ControlSection> {
        for control in &mut self.sections {
            if control.section() == section {
                return Some(control.as_mut());
            }
        }
        return None;
    }

    /// Returns the position of a cursor value inside the visible positions.
    fn slot_index(slots: &[Slot], cursor: Slot) -> Option<usize> {
        for (index, slot) in slots.iter().enumerate() {
            if *slot == cursor {
                return Some(index);
            }
        }
        return None;
    }
}

/// Returns the message that reports how many applications were found.
fn found_applications(count: usize) -> String {
    if count == 1 {
        return String::from("Found 1 application.");
    }
    return format!("Found {count} applications.");
}
