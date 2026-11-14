//! The persistent control strip at the bottom of the Standby Crates panel.
//!
//! The strip is not part of the scrollable content: it stays where it is while
//! the sections above it scroll, and it holds the controls that apply to all
//! four sections at once. It keeps no package list of its own, so the filtering
//! stays in one place: the panel reads the settings of the strip and asks the
//! device for the matching package inventory.

use crate::control::common::ControlLine;

/// The label of the checkbox that decides whether system applications show.
const SYSTEM_APPS_LABEL: &str = "Show System Apps";

/// The persistent control strip of the Standby Crates panel.
#[non_exhaustive]
pub struct Overlay {
    show_system: bool,
}

impl Overlay {
    /// Creates the strip with every control at its default.
    ///
    /// System applications are hidden by default, so a new panel lists only
    /// the applications a user installed.
    pub(crate) fn new() -> Self {
        let overlay = Self {
            show_system: false,
        };
        return overlay;
    }

    /// Returns `true` when system applications are part of the inventory.
    pub fn show_system(&self) -> bool {
        return self.show_system;
    }

    /// Toggles the focused control.
    ///
    /// Returns `true` when a setting changed, so the caller can refresh what
    /// the setting influences.
    pub fn toggle(&mut self) -> bool {
        self.show_system = !self.show_system;
        return true;
    }

    /// Returns the lines of the strip.
    ///
    /// `focused` marks the control that holds the cursor of the panel, which
    /// is what the terminal layer uses to make the focus visible.
    pub fn lines(&self, focused: bool) -> Vec<ControlLine> {
        let line =
            ControlLine::checkbox(SYSTEM_APPS_LABEL, self.show_system, focused);
        return vec![line];
    }
}
