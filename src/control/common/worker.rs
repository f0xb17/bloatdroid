//! Background execution shared by every control section.
//!
//! ADB calls are slow. A [`Worker`] runs them on its own thread so that the
//! interface keeps redrawing and reading input while an operation is in
//! flight, and reports the results back through a channel that the panel
//! drains from its event loop.

use std::io;
use std::sync::mpsc;
use std::thread;

/// A unit of work that runs on a control worker thread.
pub trait WorkerTask: Send + 'static {
    /// The value the task reports back to the interface.
    type Response: Send + 'static;

    /// Runs the task. This is called on the worker thread.
    fn execute(self) -> Self::Response;
}

/// Runs [`WorkerTask`] values on one background thread.
///
/// Tasks run one after another in the order they were submitted, so their
/// responses arrive in the same order.
#[non_exhaustive]
pub struct Worker<T: WorkerTask> {
    requests: mpsc::Sender<T>,
    responses: mpsc::Receiver<T::Response>,
}

impl<T: WorkerTask> Worker<T> {
    /// Starts a worker thread with the given name.
    pub fn spawn(name: &str) -> io::Result<Self> {
        let (request_sender, request_receiver) = mpsc::channel();
        let (response_sender, response_receiver) = mpsc::channel();
        thread::Builder::new().name(String::from(name)).spawn(move || {
            serve(&request_receiver, &response_sender);
        })?;

        let worker = Self {
            requests: request_sender,
            responses: response_receiver,
        };
        return Ok(worker);
    }

    /// Hands a task to the worker thread.
    pub fn submit(&self, task: T) -> io::Result<()> {
        if self.requests.send(task).is_err() {
            return Err(io::Error::other("the control worker has stopped"));
        }
        return Ok(());
    }

    /// Returns every response that arrived since the last call.
    pub fn drain(&self) -> Vec<T::Response> {
        let mut responses = Vec::new();
        while let Ok(response) = self.responses.try_recv() {
            responses.push(response);
        }
        return responses;
    }
}

/// Runs submitted tasks until the interface drops the worker.
fn serve<T: WorkerTask>(
    requests: &mpsc::Receiver<T>,
    responses: &mpsc::Sender<T::Response>,
) {
    loop {
        let task = match requests.recv() {
            Ok(task) => task,
            Err(_) => return,
        };
        if responses.send(task.execute()).is_err() {
            return;
        }
    }
}
