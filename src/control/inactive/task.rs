//! The inactive state work that runs on the control worker thread.

use crate::adb::AdbClient;
use crate::control::common::ControlEntry;
use crate::control::common::WorkerTask;

use super::commands::get_inactive;
use super::commands::set_inactive;
use super::scan::read_inactive;
use super::state::InactiveState;

/// An inactive state operation that runs on the worker thread.
#[non_exhaustive]
pub struct InactiveTask {
    client: AdbClient,
    serial: String,
    operation: InactiveOperation,
}

/// The operation an [`InactiveTask`] performs.
enum InactiveOperation {
    /// Read every application and its state.
    Load,
    /// Set one application and verify the result.
    Change {
        /// The package to change.
        package: String,
        /// The state to apply.
        state: InactiveState,
    },
}

/// The result of an [`InactiveTask`].
#[non_exhaustive]
pub enum InactiveOutcome {
    /// The applications and their states were read.
    Loaded {
        /// The device the snapshot belongs to.
        serial: String,
        /// The applications, sorted by package identifier.
        entries: Vec<ControlEntry>,
        /// How many applications could not be read.
        unreadable: usize,
    },
    /// The applications could not be read.
    LoadFailed {
        /// The device the attempt was made on.
        serial: String,
        /// The failure, as shown to the user.
        message: String,
    },
    /// A state was applied and read back.
    Changed {
        /// The device the change was made on.
        serial: String,
        /// The package that was changed.
        package: String,
        /// The state the device reports after the change.
        state: InactiveState,
    },
    /// A state change failed or was not applied.
    ChangeFailed {
        /// The device the attempt was made on.
        serial: String,
        /// The package that was meant to change.
        package: String,
        /// The failure, as shown to the user.
        message: String,
    },
}

impl InactiveTask {
    /// Builds a task that reads every application and its state.
    pub fn load(client: AdbClient, serial: String) -> Self {
        let task = Self {
            client,
            serial,
            operation: InactiveOperation::Load,
        };
        return task;
    }

    /// Builds a task that sets the state of one package.
    pub fn change(
        client: AdbClient,
        serial: String,
        package: String,
        state: InactiveState,
    ) -> Self {
        let operation = InactiveOperation::Change {
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

impl InactiveOutcome {
    /// Returns the device the outcome belongs to.
    pub fn serial(&self) -> &str {
        let serial = match self {
            InactiveOutcome::Loaded {
                serial,
                ..
            } => serial,
            InactiveOutcome::LoadFailed {
                serial,
                ..
            } => serial,
            InactiveOutcome::Changed {
                serial,
                ..
            } => serial,
            InactiveOutcome::ChangeFailed {
                serial,
                ..
            } => serial,
        };
        return serial.as_str();
    }
}

impl WorkerTask for InactiveTask {
    type Response = InactiveOutcome;

    fn execute(self) -> InactiveOutcome {
        match self.operation {
            InactiveOperation::Load => {
                return load(&self.client, &self.serial);
            }
            InactiveOperation::Change {
                package,
                state,
            } => {
                return change(&self.client, &self.serial, &package, state);
            }
        }
    }
}

/// Reads every application, reporting a failure as an outcome.
fn load(client: &AdbClient, serial: &str) -> InactiveOutcome {
    let snapshot = match read_inactive(client, serial) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            let outcome = InactiveOutcome::LoadFailed {
                serial: String::from(serial),
                message: error.to_string(),
            };
            return outcome;
        }
    };

    let outcome = InactiveOutcome::Loaded {
        serial: String::from(serial),
        entries: snapshot.entries,
        unreadable: snapshot.unreadable,
    };
    return outcome;
}

/// Applies one state and verifies it by reading it back.
fn change(
    client: &AdbClient,
    serial: &str,
    package: &str,
    state: InactiveState,
) -> InactiveOutcome {
    match set_inactive(client, serial, package, state) {
        Ok(()) => {}
        Err(error) => {
            let message = format!("the device rejected the change: {error}");
            return failure(serial, package, message);
        }
    }

    let reported = match get_inactive(client, serial, package) {
        Ok(reported) => reported,
        Err(error) => {
            let message = format!("could not verify the change: {error}");
            return failure(serial, package, message);
        }
    };
    let reported = match reported {
        Some(reported) => reported,
        None => {
            let message =
                String::from("the state the device reports could not be read");
            return failure(serial, package, message);
        }
    };

    if reported != state {
        let message = format!("the device kept `{}`", reported.name());
        return failure(serial, package, message);
    }

    let outcome = InactiveOutcome::Changed {
        serial: String::from(serial),
        package: String::from(package),
        state: reported,
    };
    return outcome;
}

/// Builds a failed change outcome.
fn failure(serial: &str, package: &str, message: String) -> InactiveOutcome {
    let outcome = InactiveOutcome::ChangeFailed {
        serial: String::from(serial),
        package: String::from(package),
        message,
    };
    return outcome;
}
