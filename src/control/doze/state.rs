//! The Device Idle whitelist states and the classification of a device.

use std::collections::HashSet;

/// Where an application stands in the Device Idle whitelist.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum WhitelistState {
    /// The user added the application to the whitelist.
    User,
    /// Android manages the application through a system whitelist.
    System,
    /// Neither whitelist holds the application.
    NotWhitelisted,
}

impl WhitelistState {
    /// Returns the value shown in the application list.
    pub fn name(self) -> &'static str {
        let name = match self {
            WhitelistState::User => "user",
            WhitelistState::System => "system",
            WhitelistState::NotWhitelisted => "none",
        };
        return name;
    }

    /// Reads a state from the value shown in the application list.
    pub fn from_name(name: &str) -> Option<Self> {
        let state = match name {
            "user" => WhitelistState::User,
            "system" => WhitelistState::System,
            "none" => WhitelistState::NotWhitelisted,
            _ => return None,
        };
        return Some(state);
    }

    /// Returns the state that the other arrow keys lead to.
    ///
    /// Only the user whitelist is managed here, so `user` and `none` toggle
    /// into each other and a system entry has no reachable state at all.
    pub fn toggled(self) -> Option<Self> {
        let state = match self {
            WhitelistState::User => WhitelistState::NotWhitelisted,
            WhitelistState::NotWhitelisted => WhitelistState::User,
            WhitelistState::System => return None,
        };
        return Some(state);
    }
}

/// The packages that the Device Idle whitelist of one device holds.
///
/// The two whitelists are kept apart so that a package that both hold is
/// classified as a system entry: Android manages it there, and the user
/// whitelist cannot release it.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Whitelist {
    system: HashSet<String>,
    user: HashSet<String>,
}

impl Whitelist {
    /// Records a package that Android manages through a system whitelist.
    pub fn add_system(&mut self, package: &str) {
        self.system.insert(String::from(package));
    }

    /// Records a package that the user whitelisted.
    pub fn add_user(&mut self, package: &str) {
        self.user.insert(String::from(package));
    }

    /// Returns the state of one installed package.
    ///
    /// A package that both whitelists hold is a system entry, because that is
    /// the whitelist the user cannot change.
    pub fn state_of(&self, package: &str) -> WhitelistState {
        if self.system.contains(package) {
            return WhitelistState::System;
        }
        if self.user.contains(package) {
            return WhitelistState::User;
        }
        return WhitelistState::NotWhitelisted;
    }
}
