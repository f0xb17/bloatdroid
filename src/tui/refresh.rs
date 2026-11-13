//! Background device discovery.
//!
//! The worker scans in its own thread so ADB never blocks rendering or input.
//! It performs a scan immediately, then sleeps for the refresh interval, so
//! scans never overlap: the next one only starts after the previous one
//! finished.

use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::discover::AndroidDevice;
use crate::discover::DeviceScanner;

/// The result of one discovery pass.
pub(crate) struct ScanOutcome {
    /// The devices found by the pass, empty when the pass failed.
    pub(crate) devices: Vec<AndroidDevice>,
    /// The error the pass failed with, when it failed.
    pub(crate) error: Option<String>,
}

/// Receives discovery results from the background thread.
pub(crate) struct RefreshWorker {
    receiver: mpsc::Receiver<ScanOutcome>,
}

impl RefreshWorker {
    /// Starts the background thread.
    ///
    /// The thread is detached: it ends when the application exits and the
    /// channel closes.
    pub(crate) fn spawn(
        scanner: DeviceScanner,
        interval: Duration,
    ) -> io::Result<Self> {
        let (sender, receiver) = mpsc::channel();
        thread::Builder::new().name(String::from("device-discovery")).spawn(
            move || {
                run_scans(scanner, sender, interval);
            },
        )?;
        return Ok(Self {
            receiver,
        });
    }

    /// Returns every result that arrived since the last call.
    pub(crate) fn drain(&self) -> Vec<ScanOutcome> {
        let mut outcomes = Vec::new();
        while let Ok(outcome) = self.receiver.try_recv() {
            outcomes.push(outcome);
        }
        return outcomes;
    }
}

/// Scans immediately, then once per interval until the receiver is gone.
fn run_scans(
    scanner: DeviceScanner,
    sender: mpsc::Sender<ScanOutcome>,
    interval: Duration,
) {
    loop {
        if sender.send(scan_once(&scanner)).is_err() {
            return;
        }
        thread::sleep(interval);
    }
}

/// Runs one scan and turns a failure into an outcome that carries the error.
fn scan_once(scanner: &DeviceScanner) -> ScanOutcome {
    match scanner.scan() {
        Ok(devices) => {
            return ScanOutcome {
                devices,
                error: None,
            };
        }
        Err(error) => {
            return ScanOutcome {
                devices: Vec::new(),
                error: Some(error.to_string()),
            };
        }
    }
}
