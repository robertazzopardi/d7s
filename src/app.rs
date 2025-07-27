use color_eyre::Result;
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use ratatui::{
    DefaultTerminal, Frame,
    prelude::*,
    widgets::{Block, Borders},
};

use crate::widgets::{
    connection::Connection,
    hotkey::Hotkey,
    modal::{Modal, Mode},
    table::{TableData, TableView},
    top_bar_view::{CONNECTION_HOTKEYS, TopBarView},
};

pub const APP_NAME: &str = r"
_________________        
\______ \______  \______ 
 |    |  \  /    /  ___/ 
 |    `   \/    /\___ \  
/_______  /____//____  > 
        \/           \/  
";

/// The main application which holds the state and logic of the application.
pub struct App<'a> {
    /// Is the application running?
    running: bool,
    show_popup: bool,
    modal: Option<Modal<Connection>>,
    hotkeys: Vec<Hotkey<'a>>,
}

impl<'a> Default for App<'a> {
    fn default() -> Self {
        Self {
            running: false,
            show_popup: false,
            modal: None,
            hotkeys: CONNECTION_HOTKEYS.to_vec(),
        }
    }
}

impl<'a> App<'a> {
    /// Construct a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Run the application's main loop.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_crossterm_events()?;
        }
        Ok(())
    }

    /// Renders the user interface.
    ///
    /// This is where you add new widgets. See the following resources for more information:
    ///
    /// - <https://docs.rs/ratatui/latest/ratatui/widgets/index.html>
    /// - <https://github.com/ratatui/ratatui/tree/main/ratatui-widgets/examples>
    fn render(&mut self, frame: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(13),
                Constraint::Percentage(87),
            ])
            .split(frame.area());

        let current_connection = Connection::default();
        frame.render_widget(TopBarView { current_connection }, layout[0]);

        // Create the inner block
        let block = Block::new()
            .borders(Borders::ALL)
            .title(" Connections ")
            .title_alignment(Alignment::Center);

        // Get the inner area of the block (content area)
        let inner_area = block.inner(layout[1]);

        // Render the block itself (borders and title)
        frame.render_widget(block, layout[1]);

        // Render content inside the block
        // For demonstration, we'll render a centered paragraph.
        // Replace this with your actual content widget.
        let mut content = TableView::<Connection>::new();
        content.draw(frame, inner_area);

        // Render connection modal if open
        if let Some(modal) = &self.modal {
            if modal.is_open {
                frame.render_widget(modal.clone(), frame.area());
            }
        }
    }

    /// Reads the crossterm events and updates the state of [`App`].
    ///
    /// If your application needs to perform work in between handling events, you can use the
    /// [`event::poll`] function to check if there are any events available with a timeout.
    fn handle_crossterm_events(&mut self) -> Result<()> {
        match event::read()? {
            // it's important to check KeyEventKind::Press to avoid handling key release events
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                self.on_key_event(key)
            }
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    fn on_key_event(&mut self, key: KeyEvent) {
        // Handle connection modal events first
        if let Some(modal) = &mut self.modal {
            if modal.is_open {
                modal.handle_key_events(key);
                return;
            }
        }

        // if let Some(hotkey) =
        //     self.hotkeys.iter().find(|h| h.keycode == key.code)
        // {
        //     println!("Hotkey pressed: {}", hotkey.description);
        //     return;
        // }

        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q'))
            | (
                KeyModifiers::CONTROL,
                KeyCode::Char('c') | KeyCode::Char('C'),
            ) => self.quit(),
            (_, KeyCode::Char('n')) => {
                if self.modal.is_none() {
                    self.modal =
                        Some(Modal::new(Connection::default(), Mode::New));
                    if let Some(modal) = &mut self.modal {
                        modal.open();
                    }
                }
            }
            (_, KeyCode::Char('p')) => self.toggle_popup(),
            (_, KeyCode::Esc) => {
                if self.show_popup {
                    self.toggle_popup();
                }
            }
            // Add other key handlers here.
            _ => {}
        }
    }

    fn toggle_popup(&mut self) {
        self.show_popup = !self.show_popup;
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }
}
