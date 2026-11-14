//! The whitelist work that runs on the control worker thread.

use crate::adb::AdbClient;
use crate::control::common::ControlEntry;
use crate::control::common::PackageInventory;
use crate::control::common::WorkerTask;

use super::commands::add_whitelist_entry;
use super::commands::get_whitelist;
use super::commands::remove_whitelist_entry;
use super::parser::parse_whitelist;
use super::scan::read_doze;
use super::state::WhitelistState;

/// A whitelist operation that runs on the worker thread.
#[non_exhaustive]
pub struct DozeTask {
    client: AdbClient,
    serial: String,
    operation: DozeOperation,
}

/// The operation a [`DozeTask`] performs.
enum DozeOperation {
    /// Read the whitelist state of every package of the shared inventory.
    Load {
        /// The applications to classify.
        packages: PackageInventory,
    },
    /// Change one application and verify the result.
    Change {
        /// The package to change.
        package: String,
        /// The whitelist state to apply.
        state: WhitelistState,
    },
}

/// The result of a [`DozeTask`].
#[non_exhaustive]
pub enum DozeOutcome {
    /// The applications and their whitelist states were read.
    Loaded {
        /// The device the snapshot belongs to.
        serial: String,
        /// The applications, sorted by package identifier.
        entries: Vec<ControlEntry>,
    },
    /// The applications or their whitelist could not be read.
    LoadFailed {
        /// The device the attempt was made on.
        serial: String,
        /// The failure, as shown to the user.
        message: String,
    },
    /// A whitelist state was applied and read back.
    Changed {
        /// The device the change was made on.
        serial: String,
        /// The package that was changed.
        package: String,
        /// The state the device reports after the change.
        state: WhitelistState,
    },
    /// A whitelist change failed or was not applied.
    ChangeFailed {
        /// The device the attempt was made on.
        serial: String,
        /// The package that was meant to change.
        package: String,
        /// The failure, as shown to the user.
        message: String,
    },
}

impl DozeTask {
    /// Builds a task that classifies every package of an inventory.
    pub fn load(
        client: AdbClient,
        serial: String,
        packages: PackageInventory,
    ) -> Self {
        let operation = DozeOperation::Load {
            packages,
        };
        let task = Self {
            client,
            serial,
            operation,
        };
        return task;
    }

    /// Builds a task that sets the whitelist state of one package.
    pub fn change(
        client: AdbClient,
        serial: String,
        package: String,
        state: WhitelistState,
    ) -> Self {
        let operation = DozeOperation::Change {
            package,
            state,
        };
        let task = Self {
            client,
            serial,
            operation,
        };
        return task;
    }
}

impl DozeOutcome {
    /// Returns the device the outcome belongs to.
    pub fn serial(&self) -> &str {
        let serial = match self {
            DozeOutcome::Loaded {
                serial,
                ..
            } => serial,
            DozeOutcome::LoadFailed {
                serial,
                ..
            } => serial,
            DozeOutcome::Changed {
                serial,
                ..
            } => serial,
            DozeOutcome::ChangeFailed {
                serial,
                ..
            } => serial,
        };
        return serial.as_str();
    }
}

impl WorkerTask for DozeTask {
    type Response = DozeOutcome;

    fn execute(self) -> DozeOutcome {
        match self.operation {
            DozeOperation::Load {
                packages,
            } => {
                return load(&self.client, &self.serial, &packages);
            }
            DozeOperation::Change {
                package,
                state,
            } => {
                return change(&self.client, &self.serial, &package, state);
            }
        }
    }
}

/// Classifies every package of an inventory, reporting a failure as an
/// outcome.
fn load(client: &AdbClient, serial: &str, packages: &[String]) -> DozeOutcome {
    let entries = match read_doze(client, serial, packages) {
        Ok(entries) => entries,
        Err(error) => {
            let outcome = DozeOutcome::LoadFailed {
                serial: String::from(serial),
                message: error.to_string(),
            };
            return outcome;
        }
    };

    let outcome = DozeOutcome::Loaded {
        serial: String::from(serial),
        entries,
    };
    return outcome;
}

/// Applies one whitelist state and verifies it by reading the whitelist back.
///
/// The complete whitelist is read again and the package is classified from
/// that answer, so a package that a system whitelist holds can never be
/// reported as removed even if the user entry was dropped.
fn change(
    client: &AdbClient,
    serial: &str,
    package: &str,
    state: WhitelistState,
) -> DozeOutcome {
    let result = match state {
        WhitelistState::User => add_whitelist_entry(client, serial, package),
        WhitelistState::NotWhitelisted => {
            remove_whitelist_entry(client, serial, package)
        }
        WhitelistState::System => {
            let message =
                String::from("system whitelist entries cannot be changed");
            return failure(serial, package, message);
        }
    };
    if let Err(error) = result {
        let message = format!("the device rejected the change: {error}");
        return failure(serial, package, message);
    }

    let output = match get_whitelist(client, serial) {
        Ok(output) => output,
        Err(error) => {
            let message = format!("could not verify the change: {error}");
            return failure(serial, package, message);
        }
    };
    let whitelist = match parse_whitelist(&output) {
        Ok(whitelist) => whitelist,
        Err(error) => {
            let message = format!("could not verify the change: {error}");
            return failure(serial, package, message);
        }
    };

    let reported = whitelist.state_of(package);
    if reported != state {
        let message = format!("the device kept `{}`", reported.name());
        return failure(serial, package, message);
    }

    let outcome = DozeOutcome::Changed {
        serial: String::from(serial),
        package: String::from(package),
        state: reported,
    };
    return outcome;
}

/// Builds a failed change outcome.
fn failure(serial: &str, package: &str, message: String) -> DozeOutcome {
    let outcome = DozeOutcome::ChangeFailed {
        serial: String::from(serial),
        package: String::from(package),
        message,
    };
    return outcome;
}
