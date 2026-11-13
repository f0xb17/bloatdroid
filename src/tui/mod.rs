//! Terminal user interface for the bloatdroid application.
//!
//! [`run`] owns the terminal lifecycle and the single event loop. The
//! application state lives in [`App`], the background discovery worker in
//! `refresh`, and the drawing in `render`.

mod app;
mod refresh;
mod render;

pub use app::{App, Panel};

use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
    enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::adb::AdbClient;
use crate::discover::DeviceScanner;
use crate::discover::REFRESH_INTERVAL;

use refresh::RefreshWorker;

/// How long the event loop waits for a terminal event before it looks for
/// discovery results again.
const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(250);

/// Runs the terminal interface until the user quits.
///
/// The interface redraws only when its state changed and waits for terminal
/// events with a bounded timeout, so device changes appear promptly without
/// busy-waiting. ADB never runs on this thread: a worker scans in the
/// background.
pub fn run() -> io::Result<()> {
    let scanner = DeviceScanner::new(AdbClient::new());
    let refresh = RefreshWorker::spawn(scanner, REFRESH_INTERVAL)?;

    let mut session = TerminalSession::start()?;
    let mut app = App::new(refresh);

    while app.is_running() {
        if app.needs_redraw() {
            session.draw(&app)?;
            app.clear_redraw();
        }

        if event::poll(EVENT_POLL_INTERVAL)?
            && let Event::Key(key) = event::read()?
        {
            app.handle_key(key);
        }

        app.poll_discovery();
    }

    session.stop()?;
    return Ok(());
}

/// Owns the terminal for the lifetime of the application.
///
/// Dropping the session restores the terminal, so an early return or a panic
/// does not leave the terminal in raw mode or on the alternate screen.
#[non_exhaustive]
pub struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalSession {
    /// Enables raw mode, enters the alternate screen, and prepares the backend.
    pub fn start() -> io::Result<Self> {
        enable_raw_mode()?;

        let mut stdout = io::stdout();
        let enter_result = execute!(stdout, EnterAlternateScreen);
        if enter_result.is_err() {
            let _ = disable_raw_mode();
        }
        enter_result?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = match Terminal::new(backend) {
            Ok(terminal) => terminal,
            Err(error) => {
                let _ = restore_terminal();
                return Err(error);
            }
        };

        let session = Self {
            terminal,
        };
        return Ok(session);
    }

    /// Draws the current application state.
    pub fn draw(&mut self, app: &App) -> io::Result<()> {
        self.terminal.draw(|frame| render::render(frame, app))?;
        return Ok(());
    }

    /// Restores the terminal to its original state.
    pub fn stop(&mut self) -> io::Result<()> {
        return restore_terminal();
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = restore_terminal();
    }
}

/// Restores raw mode and the alternate screen.
fn restore_terminal() -> io::Result<()> {
    let raw_mode_result = disable_raw_mode();

    let mut stdout = io::stdout();
    let screen_result = execute!(stdout, LeaveAlternateScreen);

    raw_mode_result?;
    screen_result?;
    return Ok(());
}
