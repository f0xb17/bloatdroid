//! Helpers for driving the control panel through its public interface.

use std::thread;
use std::time::Duration;

use bloatdroid::control::ControlLine;
use bloatdroid::control::ControlPanel;
use bloatdroid::control::ControlTarget;
use bloatdroid::discover::apps::ApplicationFilter;
use bloatdroid::log::LogLevel;

use crate::common::MockAdb;

/// The serial of the device every control test targets.
pub const SERIAL: &str = "serial";

/// The SDK level of Android 13.
pub const ANDROID_13: &str = "33";

/// The SDK level of Android 10, which has no restricted bucket.
pub const ANDROID_10: &str = "29";

/// How long a test waits before it looks at the panel again.
const WAIT_STEP: Duration = Duration::from_millis(5);

/// How many times a test looks at the panel before it gives up.
const WAIT_LIMIT: usize = 400;

/// Creates a panel that runs ADB through the mock and targets a device.
pub fn panel_for(mock: &MockAdb, sdk_version: &str) -> ControlPanel {
    let panel = match ControlPanel::new(mock.client()) {
        Ok(panel) => panel,
        Err(error) => panic!("could not create the control panel: {error}"),
    };

    let mut panel = panel;
    let target =
        ControlTarget::new(String::from(SERIAL), String::from(sdk_version));
    panel.set_target(Some(target));
    return panel;
}

/// Polls the panel until the expected state is reached.
pub fn wait_for(
    panel: &mut ControlPanel,
    done: impl Fn(&ControlPanel) -> bool,
) {
    for _ in 0..WAIT_LIMIT {
        panel.poll();
        if done(panel) {
            return;
        }
        thread::sleep(WAIT_STEP);
    }
    panic!("the control panel did not reach the expected state in time");
}

/// Polls the panel until the mock answered every expected command and the
/// panel applied the answers.
///
/// The mock records a command as soon as it answers it, which happens before
/// the worker thread delivers the result to the panel. Waiting for the commands
/// alone would therefore let a test assert on a state the panel has not applied
/// yet, so the effect the test expects is part of the condition.
pub fn wait_for_commands(
    panel: &mut ControlPanel,
    mock: &MockAdb,
    applied: impl Fn(&ControlPanel) -> bool,
) {
    for _ in 0..WAIT_LIMIT {
        panel.poll();
        if mock.is_complete() && applied(panel) {
            return;
        }
        thread::sleep(WAIT_STEP);
    }
    panic!("the control worker did not answer and apply its work in time");
}

/// Returns the text of the section headers, in display order.
pub fn header_texts(panel: &ControlPanel) -> Vec<String> {
    let mut texts = Vec::new();
    for line in panel.lines(true) {
        if line.is_header() {
            texts.push(String::from(line.text()));
        }
    }
    return texts;
}

/// Returns whether each section header is expanded, in display order.
pub fn header_expansion(panel: &ControlPanel) -> Vec<bool> {
    let mut expanded = Vec::new();
    for line in panel.lines(true) {
        if line.is_header() {
            expanded.push(line.is_expanded());
        }
    }
    return expanded;
}

/// Returns the text of one panel line, with the two columns of an application
/// row joined by the columns that separate them.
pub fn line_text(line: &ControlLine) -> String {
    let state = match line.state() {
        Some(state) => state,
        None => return String::from(line.text()),
    };
    return format!("{}  {}", line.text(), state);
}

/// Returns the text of the application rows, in display order.
pub fn entry_texts(panel: &ControlPanel) -> Vec<String> {
    let mut texts = Vec::new();
    for line in panel.lines(true) {
        if line.is_header() || line.is_status() {
            continue;
        }
        texts.push(line_text(&line));
    }
    return texts;
}

/// Returns the text of the status rows, in display order.
pub fn status_texts(panel: &ControlPanel) -> Vec<String> {
    let mut texts = Vec::new();
    for line in panel.lines(true) {
        if line.is_status() {
            texts.push(String::from(line.text()));
        }
    }
    return texts;
}

/// Returns the text of the row that holds the cursor.
pub fn focused_text(panel: &ControlPanel) -> Option<String> {
    for line in panel.lines(true) {
        if line.is_focused() {
            return Some(line_text(&line));
        }
    }
    return None;
}

/// Returns the levels and messages the Operations Log holds, in order.
pub fn log_entries(panel: &ControlPanel) -> Vec<(LogLevel, String)> {
    let mut entries = Vec::new();
    for entry in panel.log().snapshot() {
        entries.push((entry.level(), String::from(entry.message())));
    }
    return entries;
}

/// Returns `true` when the log holds the given message at the given level.
pub fn log_has(panel: &ControlPanel, level: LogLevel, message: &str) -> bool {
    for (logged_level, logged_message) in log_entries(panel) {
        if logged_level == level && logged_message == message {
            return true;
        }
    }
    return false;
}

/// Applies the global system application setting to the panel.
pub fn set_filter(panel: &mut ControlPanel, show_system: bool) -> bool {
    let filter = ApplicationFilter::with_show_system(show_system);
    return panel.set_filter(filter);
}
