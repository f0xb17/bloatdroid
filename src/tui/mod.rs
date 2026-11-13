//! Terminal user interface for the bloatdroid application.
//!
//! This module owns the terminal lifecycle, the application state, the panel
//! focus management, the event handling, and the rendering.
//!
//! The panels are placeholders: they display a title and show which one has
//! focus. Panel-specific behavior is added in later tasks, which is why the
//! state, the focus management, the input routing, and the rendering are kept
//! apart.

use std::io::{self, Stdout};

use crossterm::event::{self, Event, KeyCode, KeyEvent};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
    enable_raw_mode,
};
use ratatui::Frame;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Block;

/// One of the four panels shown in the terminal interface.
///
/// A panel only carries its identity and its focus behavior at this stage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Panel {
    /// Selects the Android device to operate on.
    DeviceSelection,
    /// Holds the packages that are queued for removal.
    RemovePackages,
    /// Shows the operations that are available or in progress.
    Operations,
    /// Holds the standby crates of packages.
    StandbyCrates,
}

impl Panel {
    /// Returns the title that is shown in the panel border.
    pub fn title(self) -> &'static str {
        let title = match self {
            Panel::DeviceSelection => "Device Selection",
            Panel::RemovePackages => "Remove Packages",
            Panel::Operations => "Operations",
            Panel::StandbyCrates => "Standby Crates",
        };
        return title;
    }

    /// Returns the panel that follows this one in focus order.
    pub fn next(self) -> Self {
        let next_panel = match self {
            Panel::DeviceSelection => Panel::RemovePackages,
            Panel::RemovePackages => Panel::Operations,
            Panel::Operations => Panel::StandbyCrates,
            Panel::StandbyCrates => Panel::DeviceSelection,
        };
        return next_panel;
    }

    /// Returns the panel that precedes this one in focus order.
    pub fn previous(self) -> Self {
        let previous_panel = match self {
            Panel::DeviceSelection => Panel::StandbyCrates,
            Panel::StandbyCrates => Panel::Operations,
            Panel::Operations => Panel::RemovePackages,
            Panel::RemovePackages => Panel::DeviceSelection,
        };
        return previous_panel;
    }
}

/// The application state that the event handling and the rendering share.
#[derive(Debug)]
#[non_exhaustive]
pub struct App {
    focused: Panel,
    running: bool,
}

impl Default for App {
    fn default() -> Self {
        return Self::new();
    }
}

impl App {
    /// Creates the application with the first panel focused.
    pub fn new() -> Self {
        let app = Self {
            focused: Panel::DeviceSelection,
            running: true,
        };
        return app;
    }

    /// Returns the panel that currently has focus.
    pub fn focused(&self) -> Panel {
        return self.focused;
    }

    /// Returns `true` for as long as the application should keep running.
    pub fn is_running(&self) -> bool {
        return self.running;
    }

    /// Moves the focus to the next panel.
    pub fn focus_next(&mut self) {
        self.focused = self.focused.next();
    }

    /// Moves the focus to the previous panel.
    pub fn focus_previous(&mut self) {
        self.focused = self.focused.previous();
    }

    /// Handles a key press.
    ///
    /// The global shortcuts are handled here, so they stay available no matter
    /// which panel has focus. Every other key is passed to the focused panel.
    pub fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => {
                self.running = false;
            }
            KeyCode::Tab => {
                self.focus_next();
            }
            KeyCode::BackTab => {
                self.focus_previous();
            }
            _ => {
                self.handle_panel_key(key);
            }
        }
    }

    /// Passes a key press to the focused panel.
    ///
    /// No panel accepts input yet, so this does nothing. When panels gain
    /// behavior, this is the single place where their keys are dispatched.
    pub fn handle_panel_key(&mut self, _key: KeyEvent) {
        return;
    }
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
        self.terminal.draw(|frame| render(frame, app))?;
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

/// Runs the terminal interface until the user quits.
///
/// The event loop redraws the interface and then blocks for the next terminal
/// event, so it does not poll or busy-wait.
pub fn run() -> io::Result<()> {
    let mut session = TerminalSession::start()?;
    let mut app = App::new();

    while app.is_running() {
        session.draw(&app)?;
        let event = event::read()?;
        if let Event::Key(key) = event {
            app.handle_key(key);
        }
    }

    session.stop()?;
    return Ok(());
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

/// Splits an area into a left and a right column.
fn split_columns(area: Rect) -> Option<[Rect; 2]> {
    let columns = Layout::horizontal([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(area);

    let left = *columns.first()?;
    let right = *columns.get(1)?;

    return Some([left, right]);
}

/// Draws the four panels.
fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let rows = Layout::vertical([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(area);

    let [top, bottom] = match rows.as_ref() {
        [top, bottom] => [*top, *bottom],
        _ => return,
    };

    let top_columns = match split_columns(top) {
        Some(columns) => columns,
        None => return,
    };
    let bottom_columns = match split_columns(bottom) {
        Some(columns) => columns,
        None => return,
    };

    let [top_left, top_right] = top_columns;
    let [bottom_left, bottom_right] = bottom_columns;

    let panels = [
        (top_left, Panel::DeviceSelection),
        (top_right, Panel::RemovePackages),
        (bottom_left, Panel::Operations),
        (bottom_right, Panel::StandbyCrates),
    ];

    for (panel_area, panel) in panels {
        render_panel(frame, app, panel_area, panel);
    }
}

/// Draws a single panel, styled by whether it has focus.
fn render_panel(frame: &mut Frame, app: &App, area: Rect, panel: Panel) {
    let focused = app.focused() == panel;
    let block = Block::bordered()
        .title(panel.title())
        .title_alignment(Alignment::Center)
        .border_style(panel_style(focused));
    frame.render_widget(block, area);
}

/// Returns the border style for a focused or an unfocused panel.
fn panel_style(focused: bool) -> Style {
    let style = if focused {
        Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(Color::DarkGray)
    };
    return style;
}
