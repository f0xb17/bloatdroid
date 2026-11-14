//! Control areas for Android application background behaviour.
//!
//! Every control area is an independent module with its own state and its own
//! ADB operations:
//!
//! - [`buckets`] manages Android standby buckets.
//! - [`appops`] manages AppOps background execution restrictions.
//! - [`inactive`] manages the Android application inactive state.
//! - [`doze`] manages the Doze whitelist.
//!
//! [`common`] holds what the areas share: the expandable section state, the
//! background worker that keeps ADB off the interface thread, the
//! renderer-independent panel lines, the package inventory of the selected
//! device, and the interface between the panel and a section. [`ControlPanel`]
//! composes the four sections for the Standby Crates panel, reads the installed
//! package list once per device and hands it to every section, keeps the cursor
//! inside the visible area of the terminal, and owns the control strip that
//! stays at the bottom of the panel. It is the only type the terminal interface
//! needs.

pub mod appops;
pub mod buckets;
pub mod common;
pub mod doze;
pub mod inactive;

mod overlay;
mod panel;

pub use common::ControlEntry;
pub use common::ControlLine;
pub use common::ControlSection;
pub use common::ControlTarget;
pub use common::Section;
pub use common::SectionState;
pub use panel::ControlPanel;
