//! The Standby Crates panel: four expandable sections and one cursor.
//!
//! The panel expands and collapses sections, moves the cursor through the
//! headers and entries that are currently visible, and forwards a key press to
//! the section that owns the selected entry. It does not know what any section
//! actually does.

use std::io;

use crossterm::event::KeyCode;

use crate::adb::AdbClient;

use super::appops::AppOpsSection;
use super::buckets::BucketsSection;
use super::common::ControlEntry;
use super::common::ControlLine;
use super::common::ControlSection;
use super::common::ControlTarget;
use super::common::Section;
use super::common::SectionState;

/// The message shown for an expanded section without entries.
const NO_ENTRIES: &str = "no applications";

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
}

impl ControlPanel {
    /// Creates the panel with every section collapsed.
    pub fn new(client: AdbClient) -> io::Result<Self> {
        let sections: Vec<Box<dyn ControlSection>> = vec![
            Box::new(BucketsSection::new(client.clone())?),
            Box::new(AppOpsSection::new(client)?),
            Box::new(SectionState::new(Section::Inactive)),
            Box::new(SectionState::new(Section::Doze)),
        ];

        let panel = Self {
            sections,
            cursor: Slot::Header(Section::Buckets),
            target: None,
        };
        return Ok(panel);
    }

    /// Points the panel at the device that the sections operate on.
    ///
    /// Returns `true` when the panel changed.
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
        if self.normalize_cursor() {
            changed = true;
        }
        return changed;
    }

    /// Handles a key press while the panel has focus.
    ///
    /// `SPACE` expands or collapses the focused section, the arrows move the
    /// cursor and change the value of the selected entry. Returns `true` when
    /// the panel changed.
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
        return changed;
    }

    /// Applies the background work that finished since the last call.
    ///
    /// Returns `true` when the panel changed.
    pub fn poll(&mut self) -> bool {
        let selected = self.selected_identifier();

        let mut changed = false;
        for section in &mut self.sections {
            if section.poll() {
                changed = true;
            }
        }

        if changed {
            self.restore_cursor(selected);
        }
        return changed;
    }

    /// Returns the panel content as renderer-independent lines.
    pub fn lines(&self) -> Vec<ControlLine> {
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
        let section = self.cursor_section();
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

        let header = Slot::Header(self.cursor_section());
        return self.focus(header);
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

    /// Returns the section the cursor belongs to.
    fn cursor_section(&self) -> Section {
        let section = match self.cursor {
            Slot::Header(section) => section,
            Slot::Entry {
                section,
                ..
            } => section,
        };
        return section;
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
