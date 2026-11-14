//! Functionality shared by every control area.
//!
//! A control area only describes what is specific to it: how its entries are
//! read, which values it accepts, and what a key press does to one entry. The
//! expandable section state, the background worker, the panel lines, and the
//! section interface live here so that all four areas behave the same way.

pub mod entry;
pub mod interface;
pub mod line;
pub mod section;
pub mod state;
pub mod target;
pub mod worker;

pub use entry::ControlEntry;
pub use interface::ControlSection;
pub use line::ControlLine;
pub use section::Section;
pub use state::SectionState;
pub use target::ControlTarget;
pub use worker::Worker;
pub use worker::WorkerTask;
