//! Construction of the ADB commands that removal methods run.
//!
//! A command is a value: it holds the argument list of one ADB invocation and
//! nothing else. Building it never runs ADB, so a caller or a test can inspect
//! exactly what a method would pass to the device.
//!
//! Every argument stays its own value. The device selector is added here instead
//! of being spliced into a command string, and a package name or serial number
//! that contains spaces or shell characters stays one argument, so it can never
//! be interpreted as shell input.
//!
//! The module knows the shape of a command, not the commands of the removal
//! methods: each method contributes its own arguments through
//! [`crate::remove::execution`].

use crate::discover::AndroidDevice;

use super::package::RemovalPackage;
use super::user::AndroidUser;

/// The ADB argument that selects the device of a command.
const DEVICE_ARGUMENT: &str = "-s";

/// The method that removes a package.
///
/// The two methods are deliberately separate, because they differ in what they
/// change and in what they require from the device. A user-level uninstall
/// removes the application for the current Android user without claiming that its
/// APK is gone from the system image; a system package removal changes the
/// system image and needs its own permissions and safety rules. No method is
/// assumed to support every package or every device.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum RemovalMethod {
    /// Removes the application for the current Android user.
    UserUninstall,
    /// Removes a system package from the device.
    SystemPackage,
}

impl RemovalMethod {
    /// Returns the name that the interface and the Operations Log show.
    pub fn name(self) -> &'static str {
        let name = match self {
            RemovalMethod::UserUninstall => "user uninstall",
            RemovalMethod::SystemPackage => "system package removal",
        };
        return name;
    }
}

/// One ADB invocation that a removal method would run.
///
/// The arguments are the complete list for the ADB client and begin with the
/// device selector, so every command is bound to exactly one device. The value
/// describes the invocation only; running it belongs to the execution layer.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct RemovalCommand {
    arguments: Vec<String>,
}

impl RemovalCommand {
    /// Builds the command of one device from the arguments of its operation.
    ///
    /// The serial number becomes its own `-s` argument, and every operation
    /// argument is copied as a value of its own, so nothing is concatenated into
    /// a command string.
    pub fn for_device(device: &AndroidDevice, arguments: &[&str]) -> Self {
        let mut command_arguments = Vec::new();
        command_arguments.push(String::from(DEVICE_ARGUMENT));
        command_arguments.push(device.identifier.clone());
        for argument in arguments {
            command_arguments.push(String::from(*argument));
        }

        let command = Self {
            arguments: command_arguments,
        };
        return command;
    }

    /// Returns the complete argument list for ADB, device selector included.
    pub fn arguments(&self) -> &[String] {
        return &self.arguments;
    }

    /// Returns the arguments in the form the ADB client takes them.
    ///
    /// The client receives every argument separately, so no part of the command
    /// is parsed by a shell.
    pub fn as_adb_arguments(&self) -> Vec<&str> {
        let mut arguments = Vec::new();
        for argument in &self.arguments {
            arguments.push(argument.as_str());
        }
        return arguments;
    }
}

/// Builds the command that asks the device for its active Android user.
pub fn current_user_command(device: &AndroidDevice) -> RemovalCommand {
    let arguments = ["shell", "am", "get-current-user"];
    return RemovalCommand::for_device(device, &arguments);
}

/// Builds the command that uninstalls a package for one Android user.
///
/// The user is always named explicitly, so the Package Manager never falls back
/// to a user that was not asked for. The package identifier stays one argument.
pub fn uninstall_command(
    device: &AndroidDevice,
    package: &RemovalPackage,
    user: AndroidUser,
) -> RemovalCommand {
    let user_id = user.argument();
    let arguments = [
        "shell",
        "pm",
        "uninstall",
        "--user",
        user_id.as_str(),
        package.identifier(),
    ];
    return RemovalCommand::for_device(device, &arguments);
}

/// Builds the command that lists a package when it is installed for one user.
///
/// The Package Manager prints `package:<identifier>` for every match, so the
/// answer tells whether the package is installed for that user. The identifier
/// is passed as one argument and compared exactly when the answer is read.
pub fn installed_command(
    device: &AndroidDevice,
    package: &RemovalPackage,
    user: AndroidUser,
) -> RemovalCommand {
    let user_id = user.argument();
    let arguments = [
        "shell",
        "pm",
        "list",
        "packages",
        "--user",
        user_id.as_str(),
        package.identifier(),
    ];
    return RemovalCommand::for_device(device, &arguments);
}
