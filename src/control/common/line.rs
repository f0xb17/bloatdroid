//! Renderer-independent lines of the control panel.

use super::entry::ControlEntry;

/// The kind of content a panel line carries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    /// The header of a section.
    Header,
    /// One entry of an expanded section.
    Entry,
    /// An informational message.
    Status,
}

/// One line of the Standby Crates panel.
///
/// Lines describe what to show, not how to style it, so the terminal layer can
/// pick its own colours and markers.
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
        let marker = if expanded {
            "▾"
        } else {
            "▸"
        };
        let line = Self {
            text: format!("{title} {marker}"),
            kind: Kind::Header,
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
    pub fn text(&self) -> &str {
        return self.text.as_str();
    }

    /// Returns `true` when the line is the header of a section.
    pub fn is_header(&self) -> bool {
        return self.kind == Kind::Header;
    }

    /// Returns `true` when the line is an informational message.
    pub fn is_status(&self) -> bool {
        return self.kind == Kind::Status;
    }

    /// Returns `true` when the cursor is on this line.
    pub fn is_focused(&self) -> bool {
        return self.focused;
    }
}
