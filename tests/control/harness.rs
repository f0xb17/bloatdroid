//! Helpers for driving the control panel through its public interface.

use std::thread;
use std::time::Duration;

use bloatdroid::control::ControlPanel;
use bloatdroid::control::ControlTarget;

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

/// Polls the panel until the mock answered every expected command.
pub fn wait_for_commands(panel: &mut ControlPanel, mock: &MockAdb) {
    for _ in 0..WAIT_LIMIT {
        panel.poll();
        if mock.is_complete() {
            panel.poll();
            return;
        }
        thread::sleep(WAIT_STEP);
    }
    panic!("the control worker did not answer in time");
}

/// Returns the text of the section headers, in display order.
pub fn header_texts(panel: &ControlPanel) -> Vec<String> {
    let mut texts = Vec::new();
    for line in panel.lines() {
        if line.is_header() {
            texts.push(String::from(line.text()));
        }
    }
    return texts;
}

/// Returns whether each section header is expanded, in display order.
pub fn header_expansion(panel: &ControlPanel) -> Vec<bool> {
    let mut expanded = Vec::new();
    for line in panel.lines() {
        if line.is_header() {
            expanded.push(line.is_expanded());
        }
    }
    return expanded;
}

/// Returns the text of the application rows, in display order.
pub fn entry_texts(panel: &ControlPanel) -> Vec<String> {
    let mut texts = Vec::new();
    for line in panel.lines() {
        if line.is_header() || line.is_status() {
            continue;
        }
        texts.push(String::from(line.text()));
    }
    return texts;
}

/// Returns the text of the status rows, in display order.
pub fn status_texts(panel: &ControlPanel) -> Vec<String> {
    let mut texts = Vec::new();
    for line in panel.lines() {
        if line.is_status() {
            texts.push(String::from(line.text()));
        }
    }
    return texts;
}

/// Returns the text of the row that holds the cursor.
pub fn focused_text(panel: &ControlPanel) -> Option<String> {
    for line in panel.lines() {
        if line.is_focused() {
            return Some(String::from(line.text()));
        }
    }
    return None;
}
