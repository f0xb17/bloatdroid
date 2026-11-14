//! The AppOps work that runs on the control worker thread.

use crate::adb::AdbClient;
use crate::control::common::ControlEntry;
use crate::control::common::WorkerTask;

use super::commands::get_mode;
use super::commands::set_mode;
use super::mode::AppOpMode;
use super::scan::read_appops;

/// An AppOps operation that runs on the worker thread.
#[non_exhaustive]
pub struct AppOpsTask {
    client: AdbClient,
    serial: String,
    operation: AppOpsOperation,
}

/// The operation an [`AppOpsTask`] performs.
enum AppOpsOperation {
    /// Read every application and its mode.
    Load,
    /// Set one application and verify the result.
    Change {
        /// The package to change.
        package: String,
        /// The mode to apply.
        mode: AppOpMode,
    },
}

/// The result of an [`AppOpsTask`].
#[non_exhaustive]
pub enum AppOpsOutcome {
    /// The applications and their modes were read.
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
    /// A mode was applied and read back.
    Changed {
        /// The device the change was made on.
        serial: String,
        /// The package that was changed.
        package: String,
        /// The mode the device reports after the change.
        mode: AppOpMode,
    },
    /// A mode change failed or was not applied.
    ChangeFailed {
        /// The device the attempt was made on.
        serial: String,
        /// The package that was meant to change.
        package: String,
        /// The failure, as shown to the user.
        message: String,
    },
}

impl AppOpsTask {
    /// Builds a task that reads every application and its mode.
    pub fn load(client: AdbClient, serial: String) -> Self {
        let task = Self {
            client,
            serial,
            operation: AppOpsOperation::Load,
        };
        return task;
    }

    /// Builds a task that sets the mode of one package.
    pub fn change(
        client: AdbClient,
        serial: String,
        package: String,
        mode: AppOpMode,
    ) -> Self {
        let operation = AppOpsOperation::Change {
            package,
            mode,
        };
        let task = Self {
            client,
            serial,
            operation,
        };
        return task;
    }
}

impl AppOpsOutcome {
    /// Returns the device the outcome belongs to.
    pub fn serial(&self) -> &str {
        let serial = match self {
            AppOpsOutcome::Loaded {
                serial,
                ..
            } => serial,
            AppOpsOutcome::LoadFailed {
                serial,
                ..
            } => serial,
            AppOpsOutcome::Changed {
                serial,
                ..
            } => serial,
            AppOpsOutcome::ChangeFailed {
                serial,
                ..
            } => serial,
        };
        return serial.as_str();
    }
}

impl WorkerTask for AppOpsTask {
    type Response = AppOpsOutcome;

    fn execute(self) -> AppOpsOutcome {
        match self.operation {
            AppOpsOperation::Load => {
                return load(&self.client, &self.serial);
            }
            AppOpsOperation::Change {
                package,
                mode,
            } => {
                return change(&self.client, &self.serial, &package, mode);
            }
        }
    }
}

/// Reads every application, reporting a failure as an outcome.
fn load(client: &AdbClient, serial: &str) -> AppOpsOutcome {
    let snapshot = match read_appops(client, serial) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            let outcome = AppOpsOutcome::LoadFailed {
                serial: String::from(serial),
                message: error.to_string(),
            };
            return outcome;
        }
    };

    let outcome = AppOpsOutcome::Loaded {
        serial: String::from(serial),
        entries: snapshot.entries,
        unreadable: snapshot.unreadable,
    };
    return outcome;
}

/// Applies one mode and verifies it by reading it back.
fn change(
    client: &AdbClient,
    serial: &str,
    package: &str,
    mode: AppOpMode,
) -> AppOpsOutcome {
    match set_mode(client, serial, package, mode) {
        Ok(()) => {}
        Err(error) => {
            let message = format!("the device rejected the change: {error}");
            return failure(serial, package, message);
        }
    }

    let reported = match get_mode(client, serial, package) {
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
                String::from("the mode the device reports could not be read");
            return failure(serial, package, message);
        }
    };

    if reported != mode {
        let message = format!("the device kept `{}`", reported.name());
        return failure(serial, package, message);
    }

    let outcome = AppOpsOutcome::Changed {
        serial: String::from(serial),
        package: String::from(package),
        mode: reported,
    };
    return outcome;
}

/// Builds a failed change outcome.
fn failure(serial: &str, package: &str, message: String) -> AppOpsOutcome {
    let outcome = AppOpsOutcome::ChangeFailed {
        serial: String::from(serial),
        package: String::from(package),
        message,
    };
    return outcome;
}
