//! The package that a removal request targets.
//!
//! The value holds the package identity alone. Which removal method applies and
//! what happened to the package belong to the request and its result, so a
//! package can be named before a method is chosen and keeps its identity across
//! a failed attempt. The identifier is the package name that application
//! discovery reports, so no second package model is introduced.

/// The package a removal operation targets.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct RemovalPackage {
    identifier: String,
}

impl RemovalPackage {
    /// Names the package to remove.
    ///
    /// The name is not checked here: [`crate::remove::validation`] decides
    /// whether an identifier is a valid Android package name before a request
    /// can be built from it.
    pub fn new(identifier: impl Into<String>) -> Self {
        let package = Self {
            identifier: identifier.into(),
        };
        return package;
    }

    /// Returns the package identifier, for example `com.example.app`.
    pub fn identifier(&self) -> &str {
        return self.identifier.as_str();
    }
}
