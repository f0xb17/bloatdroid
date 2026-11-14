//! The interface between the panel and a control section.

use crossterm::event::KeyCode;

use super::entry::ControlEntry;
use super::section::Section;
use super::state::SectionState;
use super::target::ControlTarget;

/// What the Standby Crates panel needs from a control section.
///
/// The panel expands and collapses sections, forwards the keys that concern the
/// selected entry, and asks each section to apply its finished background work.
/// Everything else — the entries, the values, and the ADB commands — stays
/// inside the section.
pub trait ControlSection {
    /// Returns the section this value belongs to.
    fn section(&self) -> Section;

    /// Returns the expandable state of the section.
    fn state(&self) -> &SectionState;

    /// Returns the expandable state for expanding or collapsing the section.
    fn state_mut(&mut self) -> &mut SectionState;

    /// Adopts the device the section operates on.
    ///
    /// `None` means that no device is selected. Returns `true` when the
    /// section changed.
    fn set_target(&mut self, _target: Option<&ControlTarget>) -> bool {
        return false;
    }

    /// Handles a key press for the entry that holds the cursor.
    ///
    /// Returns `true` when the section changed.
    fn handle_entry_key(
        &mut self,
        _key: KeyCode,
        _entry: &ControlEntry,
    ) -> bool {
        return false;
    }

    /// Applies the background work that finished since the last call.
    ///
    /// Returns `true` when the section changed.
    fn poll(&mut self) -> bool {
        return false;
    }
}
