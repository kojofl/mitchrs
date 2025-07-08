use std::cmp::min;

use crate::{
    bluetooth::{BluetoothEvent, mitch::MitchList},
    event::{AppEvent, Event, EventHandler},
};
use color_eyre::eyre::eyre;
use crossterm::event::KeyEventKind;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
};

/// Application.
#[derive(Debug)]
pub struct App {
    pub state: AppState,
    /// Is the application running?
    pub running: bool,
    /// List of discovered mitches
    pub mitches: MitchList,
    /// Event handler.
    pub events: EventHandler,
}

#[derive(Debug)]
pub enum AppState {
    Menu,
    Mitch,
}

impl Default for App {
    fn default() -> Self {
        Self {
            running: true,
            events: EventHandler::new(),
            mitches: MitchList::new(),
            state: AppState::Menu,
        }
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        while self.running {
            terminal.draw(|frame| frame.render_widget(&self, frame.area()))?;
            match self.events.next().await? {
                Event::Tick => {
                    self.tick().await?;
                }
                Event::Crossterm(event) => {
                    if let crossterm::event::Event::Key(key_event) = event {
                        self.handle_key_events(key_event)?
                    }
                }
                Event::App(app_event) => match app_event {
                    AppEvent::Quit => self.quit(),
                    AppEvent::PrevMitch => self.prev(),
                    AppEvent::NextMitch => self.next(),
                    AppEvent::Connect => {
                        self.mitches.get_active_mut().connect().await?;
                    }
                    AppEvent::Disconnect => {
                        self.mitches.get_active_mut().disconnect().await?;
                    }
                    AppEvent::StopRecord => {
                        self.mitches.get_active_mut().stop_recording().await?;
                    }
                    AppEvent::StartRecord => {
                        self.mitches.get_active_mut().start_recording().await?;
                    }
                },
                Event::Bluetooth(bluetooth_event) => match bluetooth_event {
                    BluetoothEvent::Discovered(mitch) => {
                        self.mitches.insert(mitch);
                    }
                    BluetoothEvent::NotActive => {
                        return Err(eyre!("Bluetooth not activated"));
                    }
                },
            }
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        // The key events depend on the current app state
        match self.state {
            AppState::Menu => {
                if key_event.kind == KeyEventKind::Release {
                    return Ok(());
                }
                match key_event.code {
                    KeyCode::Esc | KeyCode::Char('q') => self.events.send(AppEvent::Quit),
                    KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                        self.events.send(AppEvent::Quit)
                    }
                    KeyCode::Up => self.events.send(AppEvent::PrevMitch),
                    KeyCode::Down => self.events.send(AppEvent::NextMitch),
                    KeyCode::Enter => self.state = AppState::Mitch,
                    // Other handlers you could add here.
                    _ => {}
                }
            }
            AppState::Mitch => {
                if key_event.kind == KeyEventKind::Release {
                    return Ok(());
                }
                match key_event.code {
                    KeyCode::Esc | KeyCode::Char('q') => self.state = AppState::Menu,
                    KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                        self.events.send(AppEvent::Quit)
                    }
                    KeyCode::Char('c') => self.events.send(AppEvent::Connect),
                    KeyCode::Char('d') => self.events.send(AppEvent::Disconnect),
                    KeyCode::Char('r') => self.events.send(AppEvent::StartRecord),
                    KeyCode::Char('s') => self.events.send(AppEvent::StopRecord),
                    _ => {}
                }
            }
        }
        Ok(())
    }

    /// Handles the tick event of the terminal.
    ///
    /// The tick event is where you can update the state of your application with any logic that
    /// needs to be updated at a fixed frame rate. E.g. polling a server, updating an animation.
    pub async fn tick(&mut self) -> color_eyre::Result<()> {
        self.mitches.update().await?;
        Ok(())
    }

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn next(&mut self) {
        self.mitches.active = min(self.mitches.active + 1, self.mitches.len() - 1);
    }

    pub fn prev(&mut self) {
        self.mitches.active = self.mitches.active.saturating_sub(1);
    }
}
