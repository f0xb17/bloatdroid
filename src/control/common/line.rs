//! Renderer-independent lines of the control panel.

use super::entry::ControlEntry;

/// The kind of content a panel line carries.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Kind {
    /// The header of a section.
    Header {
        /// Whether the section below the header is expanded.
        expanded: bool,
    },
    /// One entry of an expanded section.
    Entry {
        /// The value that the section currently has for the entry.
        state: String,
    },
    /// An informational message.
    Status,
}

/// One line of the Standby Crates panel.
///
/// Lines describe what to show, not how to style it, so the terminal layer can
/// pick its own colours, markers, and expansion indicators. The text of a
/// header is its title alone: the indicator is a rendering decision that must
/// not influence the title layout. The text of an entry is the package name
/// alone: the state label is kept apart, because the column it occupies
/// depends on the width of the terminal and is therefore a rendering decision
/// too.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ControlLine {
    text: String,
    kind: Kind,
    focused: bool,
}

impl ControlLine {
    /// Builds the header line of a section.
    pub(crate) fn header(title: &str, expanded: bool, focused: bool) -> Self {
        let line = Self {
            text: String::from(title),
            kind: Kind::Header {
                expanded,
            },
            focused,
        };
        return line;
    }

    /// Builds the line of one entry.
    pub(crate) fn entry(entry: &ControlEntry, focused: bool) -> Self {
        let line = Self {
            text: entry.identifier.clone(),
            kind: Kind::Entry {
                state: entry.value.clone(),
            },
            focused,
        };
        return line;
    }

    /// Builds an informational line.
    pub(crate) fn status(text: &str) -> Self {
        let line = Self {
            text: String::from(text),
            kind: Kind::Status,
            focused: false,
        };
        return line;
    }

    /// Returns the text of the line.
    ///
    /// For a header this is the section title, without the indicator. For an
    /// entry this is the package name, without the state label.
    pub fn text(&self) -> &str {
        return self.text.as_str();
    }

    /// Returns the state label of an entry line.
    ///
    /// Every other kind of line yields [`None`].
    pub fn state(&self) -> Option<&str> {
        let state = match &self.kind {
            Kind::Entry {
                state,
            } => Some(state.as_str()),
            _ => None,
        };
        return state;
    }

    /// Returns `true` when the line is the header of a section.
    pub fn is_header(&self) -> bool {
        return matches!(self.kind, Kind::Header { .. });
    }

    /// Returns `true` when the section of this header is expanded.
    ///
    /// Entry and status lines always report `false`.
    pub fn is_expanded(&self) -> bool {
        let expanded = match self.kind {
            Kind::Header {
                expanded,
            } => expanded,
            _ => false,
        };
        return expanded;
    }

    /// Returns `true` when the line is an informational message.
    pub fn is_status(&self) -> bool {
        return self.kind == Kind::Status;
    }

    /// Returns `true` when the cursor is on this line.
    pub fn is_focused(&self) -> bool {
        return self.focused;
    }

    /// Clears the focus mark of the line.
    ///
    /// The cursor of the panel keeps its position while another focus target
    /// holds the focus; the mark is dropped when the lines are handed to the
    /// renderer, so the cursor does not look active from a distance.
    pub(crate) fn release_focus(&mut self) {
        self.focused = false;
    }
}
