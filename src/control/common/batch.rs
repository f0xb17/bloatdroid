//! Bounded concurrent execution of one ADB query per package.
//!
//! A scan needs one answer per package, and every answer needs its own ADB
//! process: Android has no bulk query for standby buckets, AppOps modes, or
//! inactive states. Process startup dominates such a scan, so the queries run
//! a few at a time instead of one after another. The number of threads is
//! fixed, so a device with many packages can never cause an unbounded number
//! of ADB processes.

use std::io;
use std::sync::mpsc;
use std::thread;

use crate::adb::AdbError;

/// How many package queries run at the same time during a scan.
pub const QUERY_WORKERS: usize = 8;

/// Runs one query per package with at most [`QUERY_WORKERS`] threads.
///
/// The answers are returned in the order of `packages`, so a scan can pair
/// every answer with its package. Each query keeps its own result: one failing
/// package never hides the others. A query that never answers leaves an error
/// in its place, so a partial answer is never mistaken for a successful one.
pub fn query_packages<T, F>(
    packages: &[String],
    query: F,
) -> Vec<Result<T, AdbError>>
where
    T: Send + 'static,
    F: Fn(&str) -> Result<T, AdbError> + Sync,
{
    let count = packages.len();
    if count == 0 {
        return Vec::new();
    }

    let query = &query;
    let workers = QUERY_WORKERS.min(count);
    let chunk_size = count.div_ceil(workers);
    let (sender, receiver) = mpsc::channel();

    thread::scope(|scope| {
        for (chunk_index, chunk) in packages.chunks(chunk_size).enumerate() {
            let sender = sender.clone();
            scope.spawn(move || {
                for (position, package) in chunk.iter().enumerate() {
                    let index = chunk_index * chunk_size + position;
                    let answer = query(package);
                    if sender.send((index, answer)).is_err() {
                        return;
                    }
                }
            });
        }
    });

    let mut answers = Vec::new();
    for _ in 0..count {
        answers.push(Err(unfinished()));
    }
    for (index, answer) in receiver.try_iter() {
        if let Some(slot) = answers.get_mut(index) {
            *slot = answer;
        }
    }
    return answers;
}

/// Returns the error of a query that never answered.
fn unfinished() -> AdbError {
    let error = io::Error::other("the package query did not finish");
    return AdbError::Io(error);
}
