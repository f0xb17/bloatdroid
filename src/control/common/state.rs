//! The expandable state shared by every control section.

use super::entry::ControlEntry;
use super::interface::ControlSection;
use super::section::Section;

/// The expandable state of one control section.
///
/// It holds the entries a section shows, the status message that explains what
/// the section is doing or why it failed, and whether it is expanded. The
/// cursor is not stored here: the panel owns the one cursor that moves through
/// every section.
#[derive(Debug)]
#[non_exhaustive]
pub struct SectionState {
    section: Section,
    expanded: bool,
    entries: Vec<ControlEntry>,
    status: Option<String>,
}

impl SectionState {
    /// Creates a collapsed section without entries.
    pub fn new(section: Section) -> Self {
        let state = Self {
            section,
            expanded: false,
            entries: Vec::new(),
            status: None,
        };
        return state;
    }

    /// Returns the section this state belongs to.
    pub fn section(&self) -> Section {
        return self.section;
    }

    /// Returns `true` when the section is expanded.
    pub fn is_expanded(&self) -> bool {
        return self.expanded;
    }

    /// Returns the entries the section shows while it is expanded.
    pub fn entries(&self) -> &[ControlEntry] {
        return &self.entries;
    }

    /// Returns the entry at the given position.
    pub fn entry(&self, index: usize) -> Option<&ControlEntry> {
        return self.entries.get(index);
    }

    /// Returns the status message of the section.
    pub fn status(&self) -> Option<&str> {
        return self.status.as_deref();
    }

    /// Expands a collapsed section or collapses an expanded one.
    ///
    /// Returns `true` when the section is expanded afterwards.
    pub fn toggle_expanded(&mut self) -> bool {
        self.expanded = !self.expanded;
        return self.expanded;
    }

    /// Replaces the entries and returns `true` when they changed.
    pub fn replace_entries(&mut self, entries: Vec<ControlEntry>) -> bool {
        if self.entries == entries {
            return false;
        }
        self.entries = entries;
        return true;
    }

    /// Sets the value of one entry and returns `true` when it changed.
    pub fn set_entry_value(&mut self, identifier: &str, value: &str) -> bool {
        for entry in &mut self.entries {
            if entry.identifier != identifier {
                continue;
            }
            if entry.value == value {
                return false;
            }
            entry.value = String::from(value);
            return true;
        }
        return false;
    }

    /// Sets the status message and returns `true` when it changed.
    pub fn set_status(&mut self, status: Option<String>) -> bool {
        if self.status == status {
            return false;
        }
        self.status = status;
        return true;
    }
}

impl ControlSection for SectionState {
    fn section(&self) -> Section {
        return self.section;
    }

    fn state(&self) -> &SectionState {
        return self;
    }

    fn state_mut(&mut self) -> &mut SectionState {
        return self;
    }
}
