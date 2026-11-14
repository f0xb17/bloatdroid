//! A mock ADB runner that answers commands without running anything.
//!
//! A command is only answered when the test expected it, with exactly the
//! arguments it expected. Anything else is recorded as unexpected and fails,
//! so a test never passes because an unplanned command happened to succeed.
//! The mock never starts a process, so it cannot reach a real device.

use std::io;
use std::process::ExitStatus;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use bloatdroid::adb::AdbClient;
use bloatdroid::adb::AdbError;
use bloatdroid::adb::AdbRunner;

/// What the mock answers for one expected command.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum MockResponse {
    /// The command succeeds with this standard output.
    Output(String),
    /// The command exits unsuccessfully with this standard error.
    Failure(String),
    /// ADB cannot be started at all.
    Missing,
}

impl MockResponse {
    /// Builds a successful answer with the given standard output.
    pub fn output(stdout: impl Into<String>) -> Self {
        return MockResponse::Output(stdout.into());
    }

    /// Builds a failed answer with the given standard error.
    pub fn failure(stderr: impl Into<String>) -> Self {
        return MockResponse::Failure(stderr.into());
    }

    /// Builds the answer of a command that cannot be started.
    pub fn missing() -> Self {
        return MockResponse::Missing;
    }
}

/// One command the mock expects, and the answer it gives.
#[derive(Debug)]
struct Expectation {
    arguments: Vec<String>,
    response: MockResponse,
    matched: bool,
}

/// The state the mock shares with the clients built from it.
#[derive(Debug)]
struct MockState {
    expectations: Vec<Expectation>,
    calls: Vec<Vec<String>>,
    unexpected: Vec<Vec<String>>,
    delay: Option<Duration>,
}

/// An ADB runner that answers from expectations instead of running ADB.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct MockAdb {
    state: Arc<Mutex<MockState>>,
    in_flight: Arc<AtomicUsize>,
    peak: Arc<AtomicUsize>,
}

impl Default for MockAdb {
    fn default() -> Self {
        return Self::new();
    }
}

impl MockAdb {
    /// Creates a mock without expectations.
    pub fn new() -> Self {
        let state = MockState {
            expectations: Vec::new(),
            calls: Vec::new(),
            unexpected: Vec::new(),
            delay: None,
        };
        let mock = Self {
            state: Arc::new(Mutex::new(state)),
            in_flight: Arc::new(AtomicUsize::new(0)),
            peak: Arc::new(AtomicUsize::new(0)),
        };
        return mock;
    }

    /// Makes every answer take a moment, so overlapping commands can be seen.
    pub fn set_delay(&self, delay: Duration) {
        self.lock().delay = Some(delay);
    }

    /// Returns the largest number of commands that ran at the same time.
    pub fn peak_concurrency(&self) -> usize {
        return self.peak.load(Ordering::SeqCst);
    }

    /// Expects one ADB command and the answer it produces.
    pub fn expect(&self, arguments: &[&str], response: MockResponse) {
        let expectation = Expectation {
            arguments: to_vec(arguments),
            response,
            matched: false,
        };
        let mut state = self.lock();
        state.expectations.push(expectation);
    }

    /// Creates a client that runs every command through this mock.
    pub fn client(&self) -> AdbClient {
        let runner: Arc<dyn AdbRunner> = Arc::new(self.clone());
        return AdbClient::with_runner(runner);
    }

    /// Returns the arguments of every command the mock ran, in order.
    pub fn calls(&self) -> Vec<Vec<String>> {
        return self.lock().calls.clone();
    }

    /// Returns the arguments of every command that was not expected.
    pub fn unexpected(&self) -> Vec<Vec<String>> {
        return self.lock().unexpected.clone();
    }

    /// Returns `true` when every expectation was used and nothing else ran.
    pub fn is_complete(&self) -> bool {
        let state = self.lock();
        if !state.unexpected.is_empty() {
            return false;
        }
        for expectation in &state.expectations {
            if !expectation.matched {
                return false;
            }
        }
        return true;
    }

    /// Asserts that every expectation was used and that nothing else ran.
    pub fn assert_complete(&self) {
        let state = self.lock();
        assert!(
            state.unexpected.is_empty(),
            "unexpected adb commands: {:?}",
            state.unexpected
        );
        for expectation in &state.expectations {
            assert!(
                expectation.matched,
                "unused adb expectation: {:?}",
                expectation.arguments
            );
        }
    }

    /// Returns the shared state, recovering it from a poisoned lock.
    fn lock(&self) -> MutexGuard<'_, MockState> {
        match self.state.lock() {
            Ok(state) => return state,
            Err(poisoned) => {
                return poisoned.into_inner();
            }
        }
    }
}

impl AdbRunner for MockAdb {
    fn run(&self, arguments: &[&str]) -> Result<String, AdbError> {
        let (answer, delay) = self.answer(arguments);

        // The counters stay outside the state lock, so the mock shows how many
        // commands really overlap instead of how many are waiting for it.
        let active = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(active, Ordering::SeqCst);
        if let Some(delay) = delay {
            thread::sleep(delay);
        }
        self.in_flight.fetch_sub(1, Ordering::SeqCst);
        return answer;
    }
}

impl MockAdb {
    /// Records one command and returns its answer and the configured delay.
    fn answer(
        &self,
        arguments: &[&str],
    ) -> (Result<String, AdbError>, Option<Duration>) {
        let wanted = to_vec(arguments);
        let mut state = self.lock();
        let delay = state.delay;
        state.calls.push(wanted.clone());

        let mut found = None;
        for (index, expectation) in state.expectations.iter().enumerate() {
            if !expectation.matched && expectation.arguments == wanted {
                found = Some(index);
                break;
            }
        }

        let index = match found {
            Some(index) => index,
            None => {
                state.unexpected.push(wanted.clone());
                let message =
                    format!("unexpected adb command: {}", joined(&wanted));
                let error = AdbError::Io(io::Error::other(message));
                return (Err(error), delay);
            }
        };

        let response = match state.expectations.get_mut(index) {
            Some(expectation) => {
                expectation.matched = true;
                expectation.response.clone()
            }
            None => {
                let message = format!("lost expectation: {}", joined(&wanted));
                let error = AdbError::Io(io::Error::other(message));
                return (Err(error), delay);
            }
        };
        return (respond(&response, &wanted), delay);
    }
}

/// Builds the answer of one expected command.
fn respond(
    response: &MockResponse,
    arguments: &[String],
) -> Result<String, AdbError> {
    match response {
        MockResponse::Output(stdout) => {
            return Ok(String::from(stdout.as_str()));
        }
        MockResponse::Failure(stderr) => {
            let error = AdbError::CommandFailed {
                command: format!("mock adb {}", joined(arguments)),
                status: failed_status(),
                stderr: String::from(stderr.as_str()),
            };
            return Err(error);
        }
        MockResponse::Missing => {
            let message =
                format!("could not run mock adb {}", joined(arguments));
            let error = io::Error::new(io::ErrorKind::NotFound, message);
            return Err(AdbError::Io(error));
        }
    }
}

/// Returns the exit status of a command that ran and failed.
#[cfg(unix)]
fn failed_status() -> ExitStatus {
    use std::os::unix::process::ExitStatusExt;

    return ExitStatus::from_raw(1 << 8);
}

/// Returns the exit status of a command that ran and failed.
#[cfg(windows)]
fn failed_status() -> ExitStatus {
    use std::os::windows::process::ExitStatusExt;

    return ExitStatus::from_raw(1);
}

/// Copies string arguments into owned strings.
fn to_vec(arguments: &[&str]) -> Vec<String> {
    let mut owned = Vec::new();
    for argument in arguments {
        owned.push(String::from(*argument));
    }
    return owned;
}

/// Joins arguments into a readable command line for messages.
fn joined(arguments: &[String]) -> String {
    let mut text = String::new();
    let mut first = true;
    for argument in arguments {
        if !first {
            text.push(' ');
        }
        text.push_str(argument);
        first = false;
    }
    return text;
}
