//! Functionality shared by every control area.
//!
//! A control area only describes what is specific to it: how its entries are
//! read, which values it accepts, and what a key press does to one entry. The
//! expandable section state, the background worker, the panel lines, the
//! section interface, the package inventory of the selected device, and the
//! bounded concurrent execution of the per-package queries live here so that
//! all four areas behave the same way. The two-column layout of the application
//! lists lives here as well.

pub mod batch;
pub mod entry;
pub mod interface;
pub mod inventory;
pub mod line;
pub mod row;
pub mod section;
pub mod state;
pub mod target;
pub mod worker;

pub use entry::ControlEntry;
pub use interface::ControlSection;
pub use inventory::Inventory;
pub use inventory::PackageInventory;
pub use line::ControlLine;
pub use row::application_row;
pub use section::Section;
pub use state::SectionState;
pub use target::ControlTarget;
pub use worker::Worker;
pub use worker::WorkerTask;
