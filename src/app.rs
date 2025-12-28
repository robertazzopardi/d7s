use color_eyre::Result;
use d7s_db::{connection::Connection, sqlite::init_db};
use d7s_ui::widgets::{
    hotkey::Hotkey,
    modal::ModalManager,
    search_filter::SearchFilter,
    sql_executor::SqlExecutor,
    status_line::StatusLine,
    top_bar_view::CONNECTION_HOTKEYS,
};
use ratatui::DefaultTerminal;

use crate::{
    app_state::AppState,
    database_explorer_state::DatabaseExplorer,
    filtered_data::FilteredData,
    services::{ConnectionService, PasswordService},
};

// Layout constants
pub const TOPBAR_HEIGHT_PERCENT: u16 = 13;

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
    pub(crate) running: bool,
    pub(crate) show_popup: bool,
    pub(crate) modal_manager: ModalManager,
    pub(crate) hotkeys: Vec<Hotkey<'a>>,
    /// Current application state
    pub(crate) state: AppState,
    /// Connection list with filtering
    pub(crate) connections: FilteredData<Connection>,
    /// Database explorer state (when connected to a database)
    pub(crate) database_explorer: Option<DatabaseExplorer>,
    /// SQL executor widget
    pub(crate) sql_executor: SqlExecutor,
    /// Search filter widget
    pub(crate) search_filter: SearchFilter,
    /// Status line widget
    pub(crate) status_line: StatusLine,
    /// Password management service
    pub(crate) password_service: PasswordService,
}

impl Default for App<'_> {
    fn default() -> Self {
        Self {
            running: false,
            show_popup: false,
            modal_manager: ModalManager::new(),
            hotkeys: CONNECTION_HOTKEYS.to_vec(),
            state: AppState::ConnectionList,
            connections: FilteredData::new(Vec::new()),
            database_explorer: None,
            sql_executor: SqlExecutor::new(),
            search_filter: SearchFilter::new(),
            status_line: StatusLine::new(),
            password_service: PasswordService::new(),
        }
    }
}

impl App<'_> {
    pub fn initialise(mut self) -> Result<Self> {
        init_db()?;

        let items = ConnectionService::get_all().unwrap_or_default();
        self.connections = FilteredData::new(items);

        Ok(self)
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_crossterm_events().await?;
        }
        Ok(())
    }

    /// Refresh the table data from the database
    pub(crate) fn refresh_connections(&mut self) {
        if let Ok(connections) = ConnectionService::get_all() {
            self.connections = FilteredData::new(connections);
            // Reapply filter if one is active
            if !self.search_filter.get_filter_query().is_empty() {
                self.apply_filter();
            }
        }
    }

    /// Set running to false to quit the application.
    pub(crate) const fn quit(&mut self) {
        self.running = false;
    }

    pub(crate) const fn toggle_popup(&mut self) {
        self.show_popup = !self.show_popup;
    }

    /// Set the status line message
    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status_line.set_message(message);
    }

    /// Clear the status line
    pub fn clear_status(&mut self) {
        self.status_line.clear();
    }
}
