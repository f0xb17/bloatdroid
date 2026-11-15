//! The execution layer of package removal.
//!
//! A validated request and the strategy of its method form a
//! [`RemovalOperation`]: the unit that can be handed to a background worker,
//! bound to exactly one device and one package. The module is independent of the
//! terminal, so the interface can run an operation off its drawing thread.
//!
//! Each removal method is its own [`RemovalStrategy`], with its own command and
//! its own verification, so an operation never assumes that a package or a
//! device supports a method. [`crate::remove::UserUninstall`] is the first
//! strategy.

use crate::discover::AndroidDevice;
use crate::log::OperationLog;

use super::commands::RemovalMethod;
use super::package::RemovalPackage;
use super::result::RemovalError;
use super::result::RemovalOutcome;
use super::validation::RemovalRequest;

/// One step of a removal operation, for progress reporting.
///
/// The steps carry no removal work. A caller that runs an operation on a worker
/// reports each step to the interface, so execution never waits for rendering.
#[derive(Debug)]
#[non_exhaustive]
pub enum RemovalProgress {
    /// The command is about to run on the device.
    Started,
    /// The command ran and its result is being verified.
    Verifying,
    /// The operation finished with the given outcome.
    Finished(RemovalOutcome),
}

/// Performs one removal method.
///
/// A strategy turns a validated request into the command of its method, verifies
/// what the device did, and reports its lifecycle to the Operations Log. The
/// interface is `Send + Sync`, so an operation can move to a background worker.
/// A strategy that cannot handle a package or a device reports
/// [`RemovalError::UnsupportedMethod`] instead of running a command.
pub trait RemovalStrategy: Send + Sync {
    /// Returns the removal method this strategy performs.
    fn method(&self) -> RemovalMethod;

    /// Removes the package of the request and verifies the result.
    ///
    /// Only the package and the device named by the request are touched, and a
    /// successful command is never reported as a removal on its own.
    fn run(
        &self,
        request: &RemovalRequest,
        log: &OperationLog,
    ) -> RemovalOutcome;
}

/// A validated request together with the strategy that performs it.
#[non_exhaustive]
pub struct RemovalOperation {
    request: RemovalRequest,
    strategy: Box<dyn RemovalStrategy>,
}

impl RemovalOperation {
    /// Pairs a validated request with the strategy of its method.
    ///
    /// The strategy has to perform the method the request names; pairing a
    /// request with another method is refused, because a method is only assumed
    /// to handle its own packages and permissions.
    pub fn new(
        request: RemovalRequest,
        strategy: Box<dyn RemovalStrategy>,
    ) -> Result<Self, RemovalError> {
        if strategy.method() != request.method() {
            return Err(RemovalError::UnsupportedMethod);
        }

        let operation = Self {
            request,
            strategy,
        };
        return Ok(operation);
    }

    /// Returns the request the operation performs.
    pub fn request(&self) -> &RemovalRequest {
        return &self.request;
    }

    /// Returns the device the operation removes the package from.
    pub fn device(&self) -> &AndroidDevice {
        return self.request.device();
    }

    /// Returns the package the operation removes.
    pub fn package(&self) -> &RemovalPackage {
        return self.request.package();
    }

    /// Returns the removal method the operation uses.
    pub fn method(&self) -> RemovalMethod {
        return self.request.method();
    }

    /// Runs the operation and returns what the device did.
    pub fn run(&self, log: &OperationLog) -> RemovalOutcome {
        return self.strategy.run(&self.request, log);
    }
}
