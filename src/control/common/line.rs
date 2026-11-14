//! Renderer-independent lines of the control panel.

use super::entry::ControlEntry;

/// The kind of content a panel line carries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    /// The header of a section.
    Header {
        /// Whether the section below the header is expanded.
        expanded: bool,
    },
    /// One entry of an expanded section.
    Entry,
    /// An informational message.
    Status,
    /// A checkbox of the control strip at the bottom of the panel.
    Checkbox {
        /// Whether the checkbox is ticked.
        checked: bool,
    },
}

/// One line of the Standby Crates panel.
///
/// Lines describe what to show, not how to style it, so the terminal layer can
/// pick its own colours, markers, and expansion indicators. The text of a
/// header is its title alone: the indicator is a rendering decision that must
/// not influence the title layout.
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
            text: format!("{}  ({})", entry.identifier, entry.value),
            kind: Kind::Entry,
            focused,
        };
        return line;
    }

    /// Builds the line of a checkbox.
    pub(crate) fn checkbox(label: &str, checked: bool, focused: bool) -> Self {
        let line = Self {
            text: String::from(label),
            kind: Kind::Checkbox {
                checked,
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
    /// For a header this is the section title, without the indicator.
    pub fn text(&self) -> &str {
        return self.text.as_str();
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

    /// Returns `true` when the line is a checkbox of the control strip.
    pub fn is_checkbox(&self) -> bool {
        return matches!(self.kind, Kind::Checkbox { .. });
    }

    /// Returns `true` when the checkbox of this line is ticked.
    ///
    /// Every other kind of line reports `false`.
    pub fn is_checked(&self) -> bool {
        let checked = match self.kind {
            Kind::Checkbox {
                checked,
            } => checked,
            _ => false,
        };
        return checked;
    }

    /// Returns `true` when the cursor is on this line.
    pub fn is_focused(&self) -> bool {
        return self.focused;
    }
}
