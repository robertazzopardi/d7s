use color_eyre::Result;
use crossterm::{clipboard, execute};
use d7s_db::{TableData, sqlite::init_db};
use d7s_ui::widgets::{
    hotkey::Hotkey, modal::ModalManager, search_filter::SearchFilter,
    status_line::StatusLine, top_bar_view::CONNECTION_HOTKEYS,
};
use ratatui::DefaultTerminal;

use crate::{
    app_state::{AppState, DatabaseExplorerState},
    database_explorer_state::DatabaseExplorer,
    filtered_data::FilteredData,
    services::{ConnectionService, PasswordService},
};

// Layout constants
pub const TOPBAR_HEIGHT_PERCENT: u16 = 13;

pub const APP_NAME: &str = r"_________________
\______ \______  \______
 |    |  \  /    /  ___/
 |    `   \/    /\___  \
/_______  /____//____  /
        \/           \/
";

// Build metadata
pub const PKG_NAME: &str = env!("CARGO_PKG_NAME");
pub const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The main application which holds the state and logic of the application.
pub struct App<'a> {
    /// Is the application running?
    pub(crate) running: bool,
    pub(crate) modal_manager: ModalManager,
    pub(crate) hotkeys: Vec<Hotkey<'a>>,
    /// Current application state
    pub(crate) state: AppState,
    /// Database explorer state (when connected to a database)
    pub(crate) database_explorer: DatabaseExplorer,
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
            modal_manager: ModalManager::new(),
            hotkeys: CONNECTION_HOTKEYS.to_vec(),
            state: AppState::ConnectionList,
            database_explorer: DatabaseExplorer::default(),
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
        self.database_explorer.connections = FilteredData::new(items);

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
            self.database_explorer.connections = FilteredData::new(connections);
            // Reapply filter if one is active
            if !self.search_filter.get_filter_query().is_empty() {
                self.apply_filter();
            }
        }
    }

    /// Copy the value under the cursor to the clipboard
    pub(crate) fn copy(&mut self) {
        let explorer = &self.database_explorer;
        let value: Option<String> = (|| -> Option<String> {
            let v = match &explorer.state {
                DatabaseExplorerState::Connections => {
                    let view = &explorer.connections.table.view;
                    let selected = view.state.selected()?;
                    let col = view.state.selected_column().unwrap_or(0);
                    explorer
                        .connections
                        .table
                        .model
                        .items
                        .get(selected)?
                        .col(col)
                }
                DatabaseExplorerState::Databases => {
                    let dbs = explorer.databases.as_ref()?;
                    let selected = dbs.table.view.state.selected()?;
                    let col =
                        dbs.table.view.state.selected_column().unwrap_or(0);
                    dbs.table.model.items.get(selected)?.col(col)
                }
                DatabaseExplorerState::Schemas => {
                    let schemas = explorer.schemas.as_ref()?;
                    let selected = schemas.table.view.state.selected()?;
                    let col =
                        schemas.table.view.state.selected_column().unwrap_or(0);
                    schemas.table.model.items.get(selected)?.col(col)
                }
                DatabaseExplorerState::Tables(_) => {
                    let tables = explorer.tables.as_ref()?;
                    let selected = tables.table.view.state.selected()?;
                    let col =
                        tables.table.view.state.selected_column().unwrap_or(0);
                    tables.table.model.items.get(selected)?.col(col)
                }
                DatabaseExplorerState::Columns(_, _) => {
                    let columns = explorer.columns.as_ref()?;
                    let selected = columns.table.view.state.selected()?;
                    let col =
                        columns.table.view.state.selected_column().unwrap_or(0);
                    columns.table.model.items.get(selected)?.col(col)
                }
                DatabaseExplorerState::TableData(_, _) => {
                    let table_data = explorer.table_data.as_ref()?;
                    let selected_row =
                        table_data.table.view.state.selected()?;
                    let selected_col = table_data
                        .table
                        .view
                        .state
                        .selected_column()
                        .unwrap_or(0);
                    let row = table_data.table.model.items.get(selected_row)?;
                    row.values.get(selected_col)?.clone()
                }
                DatabaseExplorerState::SqlExecutor => {
                    let table = &explorer.sql_executor.table_state;
                    let selected_row = table.view.state.selected()?;
                    let selected_col =
                        table.view.state.selected_column().unwrap_or(0);
                    let row = table.model.items.get(selected_row)?;
                    row.values.get(selected_col)?.clone()
                }
            };
            Some(v)
        })();
        if let Some(value) = value
            && execute!(
                std::io::stdout(),
                clipboard::CopyToClipboard {
                    content: value.clone(),
                    destination: clipboard::ClipboardSelection(vec![
                        clipboard::ClipboardType::Clipboard,
                    ]),
                }
            )
            .is_ok()
        {
            self.set_status(format!("Copied: {value}"));
        }
    }

    /// Set running to false to quit the application.
    pub(crate) const fn quit(&mut self) {
        self.running = false;
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
