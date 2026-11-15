//! The user-level uninstall method.
//!
//! The strategy removes an application for the active Android user with the
//! Package Manager. It never touches the system image: a preinstalled system
//! application loses its installation for that user, and its APK stays where
//! the system image holds it.
//!
//! The active user is read from the device first, so user `0` is never assumed.
//! The package has to be installed for that user before anything is uninstalled,
//! and the result is verified with an independent listing afterwards. A
//! successful command is not reported as a removal until the device confirms
//! that the package is gone.

use crate::adb::AdbClient;
use crate::adb::AdbError;
use crate::log::LogLevel;
use crate::log::OperationLog;

use super::commands::RemovalCommand;
use super::commands::RemovalMethod;
use super::commands::installed_command;
use super::commands::uninstall_command;
use super::execution::RemovalStrategy;
use super::result::RemovalError;
use super::result::RemovalOutcome;
use super::user::AndroidUser;
use super::user::current_user;
use super::validation::RemovalRequest;

/// The answer of the Package Manager to one uninstall command.
enum UninstallAnswer {
    /// The Package Manager accepted the uninstall.
    Succeeded,
    /// The Package Manager refused the uninstall, with its reason.
    Rejected(String),
}

/// Whether a package is installed for one user.
enum Installation {
    /// The package is installed for the user.
    Installed,
    /// The package is not installed for the user.
    Absent,
    /// The answer did not say whether the package is installed.
    Unknown,
}

/// Removes an application for the active Android user.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct UserUninstall {
    client: AdbClient,
}

impl UserUninstall {
    /// Creates the strategy for one ADB client.
    pub fn new(client: AdbClient) -> Self {
        let strategy = Self {
            client,
        };
        return strategy;
    }

    /// Runs the user-level uninstall of the request's package.
    fn uninstall(
        &self,
        request: &RemovalRequest,
        log: &OperationLog,
    ) -> RemovalOutcome {
        let package = request.package().identifier();
        let preparing = format!("Preparing user-level uninstall of {package}.");
        log.record(LogLevel::Info, preparing);

        let user = match self.active_user(request, log) {
            Ok(user) => user,
            Err(error) => return RemovalOutcome::NotRemoved(error),
        };
        log.record(
            LogLevel::Info,
            format!("Target Android user: {}.", user.id()),
        );

        match self.check_installed(request, user, log) {
            Err(error) => return RemovalOutcome::NotRemoved(error),
            Ok(false) => {
                let message = format!(
                    "Package {package} is not installed for user {}.",
                    user.id()
                );
                log.record(LogLevel::Info, message);
                return RemovalOutcome::NotInstalled;
            }
            Ok(true) => {}
        }

        log.record(
            LogLevel::Info,
            format!("Uninstalling {package} for user {}.", user.id()),
        );
        if let Some(error) = self.send_uninstall(request, user, log) {
            return RemovalOutcome::NotRemoved(error);
        }

        return self.finish(request, user, log);
    }

    /// Reads the active user, logging the failure when the device does not
    /// report one.
    fn active_user(
        &self,
        request: &RemovalRequest,
        log: &OperationLog,
    ) -> Result<AndroidUser, RemovalError> {
        let user = current_user(&self.client, request.device());
        if let Err(error) = &user {
            let message = format!(
                "Could not determine the current Android user: {error}."
            );
            log.record(LogLevel::Error, message);
        }
        return user;
    }

    /// Reports whether the package is installed for the user.
    fn check_installed(
        &self,
        request: &RemovalRequest,
        user: AndroidUser,
        log: &OperationLog,
    ) -> Result<bool, RemovalError> {
        let installed = self.installation(request, user);
        if let Err(error) = &installed {
            let message = format!(
                "Could not check whether {} is installed for user {}: {error}.",
                request.package().identifier(),
                user.id()
            );
            log.record(LogLevel::Error, message);
        }
        return installed;
    }

    /// Sends the uninstall command and returns the error it reported, if any.
    fn send_uninstall(
        &self,
        request: &RemovalRequest,
        user: AndroidUser,
        log: &OperationLog,
    ) -> Option<RemovalError> {
        let package = request.package().identifier();
        let command =
            uninstall_command(request.device(), request.package(), user);

        let output = match self.run(&command) {
            Ok(output) => output,
            Err(error) => {
                let error = adb_error(error);
                log.record(
                    LogLevel::Error,
                    format!(
                        "Uninstalling {package} for user {} failed: {error}.",
                        user.id()
                    ),
                );
                return Some(error);
            }
        };

        match parse_uninstall_answer(output.as_str()) {
            UninstallAnswer::Succeeded => return None,
            UninstallAnswer::Rejected(reason) => {
                let error = rejection_error(reason.as_str());
                let message =
                    rejection_message(package, user, reason.as_str(), &error);
                log.record(LogLevel::Error, message);
                return Some(error);
            }
        }
    }

    /// Verifies that the package is gone and returns the outcome.
    fn finish(
        &self,
        request: &RemovalRequest,
        user: AndroidUser,
        log: &OperationLog,
    ) -> RemovalOutcome {
        let package = request.package().identifier();
        let installed = match self.installation(request, user) {
            Ok(installed) => installed,
            Err(error) => {
                let message = format!(
                    "Could not verify the user-level uninstall of {package} for user {}: {error}.",
                    user.id()
                );
                log.record(LogLevel::Error, message);
                return RemovalOutcome::NotRemoved(error);
            }
        };

        if installed {
            let message = format!(
                "Package {package} is still installed for user {} after the user-level uninstall.",
                user.id()
            );
            log.record(LogLevel::Error, message);
            return RemovalOutcome::StillPresent;
        }

        let message = format!(
            "User-level uninstall of {package} for user {} succeeded and was verified.",
            user.id()
        );
        log.record(LogLevel::Success, message);
        return RemovalOutcome::Removed;
    }

    /// Lists the package for the user and reports whether it is installed.
    ///
    /// An answer that is neither empty nor a package listing is ambiguous, so it
    /// is reported as a verification failure instead of being read as absence.
    fn installation(
        &self,
        request: &RemovalRequest,
        user: AndroidUser,
    ) -> Result<bool, RemovalError> {
        let command =
            installed_command(request.device(), request.package(), user);
        let output = match self.run(&command) {
            Ok(output) => output,
            Err(error) => return Err(adb_error(error)),
        };

        match parse_installation(
            output.as_str(),
            request.package().identifier(),
        ) {
            Installation::Installed => return Ok(true),
            Installation::Absent => return Ok(false),
            Installation::Unknown => {
                let message = format!(
                    "the device did not report whether {} is installed for user {}",
                    request.package().identifier(),
                    user.id()
                );
                return Err(RemovalError::VerificationFailed(message));
            }
        }
    }

    /// Runs one command and returns its output.
    fn run(&self, command: &RemovalCommand) -> Result<String, AdbError> {
        let arguments = command.as_adb_arguments();
        return self.client.run(&arguments);
    }
}

impl RemovalStrategy for UserUninstall {
    fn method(&self) -> RemovalMethod {
        return RemovalMethod::UserUninstall;
    }

    fn run(
        &self,
        request: &RemovalRequest,
        log: &OperationLog,
    ) -> RemovalOutcome {
        return self.uninstall(request, log);
    }
}

/// Reads the Package Manager answer of one uninstall command.
///
/// The Package Manager prints `Success` when it accepted the command and
/// `Failure [<reason>]` when it did not. It does not always exit unsuccessfully
/// when it refuses, so the text is what decides, and anything that is not the
/// success text is a rejection.
fn parse_uninstall_answer(output: &str) -> UninstallAnswer {
    let text = output.trim();
    if text.eq_ignore_ascii_case("Success") {
        return UninstallAnswer::Succeeded;
    }
    if text.is_empty() {
        return UninstallAnswer::Rejected(String::from("the answer was empty"));
    }

    if let Some(rest) = text.strip_prefix("Failure") {
        let reason = rest.trim().trim_matches(&['[', ']'][..]).trim();
        if reason.is_empty() {
            return UninstallAnswer::Rejected(String::from(
                "no reason was reported",
            ));
        }
        return UninstallAnswer::Rejected(String::from(reason));
    }

    return UninstallAnswer::Rejected(String::from(text));
}

/// Reads a package listing of one installation check.
///
/// The identifier is compared exactly, so a longer package name that contains
/// the target as a prefix is not mistaken for it.
fn parse_installation(output: &str, package: &str) -> Installation {
    let mut recognized = true;

    for line in output.lines() {
        let text = line.trim();
        if text.is_empty() {
            continue;
        }
        let name = match text.strip_prefix("package:") {
            Some(name) => name.trim(),
            None => {
                recognized = false;
                continue;
            }
        };
        if name == package {
            return Installation::Installed;
        }
    }

    if !recognized {
        return Installation::Unknown;
    }
    return Installation::Absent;
}

/// Turns a Package Manager rejection into an error.
fn rejection_error(reason: &str) -> RemovalError {
    let lowered = reason.to_ascii_lowercase();
    if mentions_permission(lowered.as_str()) {
        return RemovalError::PermissionDenied;
    }
    if mentions_unsupported(lowered.as_str()) {
        return RemovalError::UnsupportedMethod;
    }
    return RemovalError::Rejected {
        reason: String::from(reason),
    };
}

/// Turns an ADB failure into an error, keeping a permission refusal apart.
fn adb_error(error: AdbError) -> RemovalError {
    if let AdbError::CommandFailed {
        stderr,
        ..
    } = &error
    {
        let lowered = stderr.to_ascii_lowercase();
        if mentions_permission(lowered.as_str()) {
            return RemovalError::PermissionDenied;
        }
    }
    return RemovalError::Adb(error);
}

/// Reports whether the text describes a permission refusal.
fn mentions_permission(text: &str) -> bool {
    return text.contains("permission")
        || text.contains("securityexception")
        || text.contains("user_restricted")
        || text.contains("not allowed");
}

/// Reports whether the text describes an unsupported operation.
fn mentions_unsupported(text: &str) -> bool {
    return text.contains("not supported") || text.contains("unsupported");
}

/// Returns the log message of one rejected uninstall.
fn rejection_message(
    package: &str,
    user: AndroidUser,
    reason: &str,
    error: &RemovalError,
) -> String {
    let message = match error {
        RemovalError::PermissionDenied => format!(
            "Package Manager denied permission for the user-level uninstall of {package}."
        ),
        RemovalError::UnsupportedMethod => format!(
            "Package Manager does not support the user-level uninstall of {package}: {reason}."
        ),
        _ => format!(
            "Package Manager rejected the user-level uninstall of {package} for user {}: {reason}.",
            user.id()
        ),
    };
    return message;
}
