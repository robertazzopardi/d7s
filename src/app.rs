use color_eyre::Result;
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use d7s_db::{TableData, connection::Connection};
use ratatui::{
    DefaultTerminal, Frame,
    prelude::*,
    widgets::{Block, Borders, TableState},
};

use crate::widgets::{
    hotkey::Hotkey,
    modal::{Modal, Mode},
    table::DataTable,
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
    table_widget: DataTable<Connection>,
}

impl<'a> Default for App<'a> {
    fn default() -> Self {
        d7s_db::sqlite::init_db().unwrap();

        let items = d7s_db::sqlite::get_connections().unwrap_or_default();

        let table_widget = DataTable::new(items.clone());

        Self {
            running: false,
            show_popup: false,
            modal: None,
            hotkeys: CONNECTION_HOTKEYS.to_vec(),
            table_widget,
        }
    }
}

impl<'a> App<'a> {
    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_crossterm_events().await?;
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
        // Use the data table directly
        frame.render_stateful_widget(
            self.table_widget.clone(),
            inner_area,
            &mut self.table_widget.table_state,
        );

        // Render connection modal if open
        if let Some(modal) = &self.modal
            && modal.is_open
        {
            frame.render_widget(modal.clone(), frame.area());
        }
    }

    /// Reads the crossterm events and updates the state of [`App`].
    ///
    /// If your application needs to perform work in between handling events, you can use the
    /// [`event::poll`] function to check if there are any events available with a timeout.
    async fn handle_crossterm_events(&mut self) -> Result<()> {
        match event::read()? {
            // it's important to check KeyEventKind::Press to avoid handling key release events
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                self.on_key_event(key).await?;
            }
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
            _ => {}
        }

        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    async fn on_key_event(&mut self, key: KeyEvent) -> Result<()> {
        // Handle connection modal events first
        if let Some(modal) = &mut self.modal
            && modal.is_open
        {
            modal.handle_key_events(key).await?;
            return Ok(());
        }

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
            // Vim search navigation (conflicts with 'n' for new, so using different keys)
            (_, KeyCode::Char('*')) => {
                // TODO: Implement search for word under cursor
            }
            (_, KeyCode::Char('#')) => {
                // TODO: Implement search for word under cursor backward
            }
            (_, KeyCode::Char('p')) => self.toggle_popup(),
            (_, KeyCode::Esc) => {
                if self.show_popup {
                    self.toggle_popup();
                }
            }
            // Vim keybindings for table navigation
            (_, KeyCode::Char('j')) | (_, KeyCode::Down) => {
                self.table_widget.table_state.select_next();
            }
            (_, KeyCode::Char('k')) | (_, KeyCode::Up) => {
                self.table_widget.table_state.select_previous();
            }
            (_, KeyCode::Char('h')) | (_, KeyCode::Left) => {
                self.table_widget.table_state.select_previous_column();
            }
            (_, KeyCode::Char('l')) | (_, KeyCode::Right) => {
                self.table_widget.table_state.select_next_column();
            }
            // Word movement
            (_, KeyCode::Char('w')) => {
                self.table_widget.table_state.select_next_column();
            }
            (_, KeyCode::Char('b')) => {
                self.table_widget.table_state.select_previous_column();
            }
            // Jump to edges
            (_, KeyCode::Char('0')) => {
                self.table_widget.table_state.select_column(Some(0));
            }
            (_, KeyCode::Char('$')) => {
                if let Some(num_cols) = self
                    .table_widget
                    .items
                    .first()
                    .map(|item| item.num_columns())
                {
                    self.table_widget
                        .table_state
                        .select_column(Some(num_cols.saturating_sub(1)));
                }
            }
            (_, KeyCode::Char('g')) => {
                self.table_widget.table_state.select(Some(1)); // First data row
            }
            (_, KeyCode::Char('G')) => {
                if !self.table_widget.items.is_empty() {
                    self.table_widget
                        .table_state
                        .select(Some(self.table_widget.items.len()));
                }
            }
            // Add other key handlers here.
            _ => {}
        }

        Ok(())
    }

    fn toggle_popup(&mut self) {
        self.show_popup = !self.show_popup;
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }
}
