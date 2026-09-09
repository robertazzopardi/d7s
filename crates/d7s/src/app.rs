use std::{path::Path, process::Command};

use color_eyre::Result;
use crossterm::{
    ExecutableCommand, clipboard,
    event::{DisableBracketedPaste, EnableBracketedPaste},
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use ratatui::DefaultTerminal;
use ratatui_textarea::TextArea;

use crate::{
    app_state::{AppState, DatabaseExplorerState},
    database_explorer_state::DatabaseExplorer,
    db::{RowDeleteSpec, TableData, sqlite::init_db},
    filtered_data::FilteredData,
    services::{ConnectionService, PasswordService, PreferencesService},
    sql::safety::{StatementSafety, classify_statement, split_statements},
    ui::widgets::{
        help_view::HelpRow, hotkey::Hotkey, modal::ModalManager,
        status_line::StatusLine, table::TableDataState,
        top_bar_view::CONNECTION_HOTKEYS,
    },
    virtual_table::VIRTUAL_TABLE_PAGE_SIZE,
};

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
#[allow(clippy::struct_excessive_bools)] // session flags; not worth a state machine
pub struct App<'a> {
    /// Is the application running?
    pub(crate) running: bool,
    pub(crate) modal_manager: ModalManager,
    pub(crate) hotkeys: Vec<Hotkey>,
    /// Current application state
    pub(crate) state: AppState,
    /// Database explorer state (when connected to a database)
    pub(crate) database_explorer: DatabaseExplorer,
    /// Search filter widget
    pub(crate) search_filter: Option<TextArea<'a>>,
    /// Status line widget
    pub(crate) status_line: StatusLine,
    /// Password management service
    pub(crate) password_service: PasswordService,
    /// Build info
    pub(crate) build_info: String,
    /// Signal to the run loop to open the external editor
    pub(crate) open_editor_requested: bool,
    /// Table data: after `d`, row locators awaiting delete confirmation.
    pub(crate) pending_row_deletes: Option<Vec<RowDeleteSpec>>,
    /// k9s-style help panel in main content area (`?` toggles).
    pub(crate) show_help: bool,
    pub(crate) help_table: TableDataState<HelpRow>,
    /// Rows per virtual table page (from prefs / `D7S_PAGE_SIZE`).
    pub(crate) page_size: u32,
    /// First Esc warns before dropping draft rows; second Esc discards.
    pub(crate) draft_discard_pending: bool,
    /// One-shot hint after first connect in a session.
    pub(crate) showed_help_hint: bool,
}

impl Default for App<'_> {
    fn default() -> Self {
        Self {
            running: false,
            modal_manager: ModalManager::new(),
            hotkeys: CONNECTION_HOTKEYS.to_vec(),
            state: AppState::ConnectionList,
            database_explorer: DatabaseExplorer::default(),
            search_filter: None,
            status_line: StatusLine::new(),
            password_service: PasswordService::new(),
            build_info: String::new(),
            open_editor_requested: false,
            pending_row_deletes: None,
            show_help: false,
            help_table: TableDataState::new(Vec::new()),
            page_size: VIRTUAL_TABLE_PAGE_SIZE,
            draft_discard_pending: false,
            showed_help_hint: false,
        }
    }
}

impl App<'_> {
    /// Post initilisation for the App
    pub fn init(mut self) -> Result<Self> {
        init_db()?;

        let items = ConnectionService::get_all().unwrap_or_default();
        self.database_explorer.connections = FilteredData::new(items);
        self.database_explorer.recent_tables =
            PreferencesService::load_recent_tables();
        self.page_size = PreferencesService::effective_page_size();

        self.build_info = build_info()?;

        Ok(self)
    }

    /// Run the application's main loop.
    pub async fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_crossterm_events().await?;

            self.handle_external_terminal(&mut terminal).await?;
        }
        self.save_session_preferences();
        Ok(())
    }

    fn save_session_preferences(&self) {
        let _ = PreferencesService::save_recent_tables(
            &self.database_explorer.recent_tables,
        );
        let sql = self.database_explorer.sql_executor.sql_input();
        if !sql.trim().is_empty() {
            let _ = PreferencesService::push_sql_history(&sql);
        }
        if self.state == AppState::DatabaseConnected {
            let _ = PreferencesService::set_last_connection(
                &self.database_explorer.connection.name,
            );
        }
        let _ = PreferencesService::set_page_size(self.page_size);
    }

    async fn handle_external_terminal(
        &mut self,
        terminal: &mut DefaultTerminal,
    ) -> Result<(), color_eyre::eyre::Error> {
        if self.open_editor_requested {
            self.open_editor_requested = false;
            let new_sql = if std::env::var("D7S_DEMO").is_ok() {
                std::env::var("D7S_DEMO_SQL")
                    .ok()
                    .and_then(|path| std::fs::read_to_string(path).ok())
                    .unwrap_or_default()
            } else {
                let temp_path = std::env::temp_dir()
                    .join(format!("d7s_sql_{}.sql", std::process::id()));
                let current_sql =
                    self.database_explorer.sql_executor.sql_input();
                std::fs::write(&temp_path, &current_sql)?;
                Self::run_editor(terminal, &temp_path)?;
                std::fs::read_to_string(&temp_path).unwrap_or_default()
            };
            self.apply_editor_sql(new_sql).await;
        }

        Ok(())
    }

    async fn apply_editor_sql(&mut self, new_sql: String) {
        let new_sql = new_sql.trim_end_matches('\n');
        if new_sql.is_empty() {
            return;
        }
        self.database_explorer.sql_executor.set_sql(new_sql);
        let statements = split_statements(new_sql);
        if statements.is_empty() {
            self.set_status("No SQL statements found in editor file.");
            return;
        }

        if statements.len() == 1 {
            if let Some(statement) = statements.first() {
                self.prepare_sql_statement_execution(statement.text.clone())
                    .await;
            }
        } else {
            let options =
                statements.into_iter().map(|s| s.text).collect::<Vec<_>>();
            self.modal_manager.open_sql_query_selection_modal(options);
        }
    }

    /// Refresh the table data from the database
    pub(crate) fn refresh_connections(&mut self) {
        if let Ok(connections) = ConnectionService::get_all() {
            self.database_explorer.connections = FilteredData::new(connections);
            // Reapply filter if one is active
            if let Some(search_filter) = &self.search_filter
                && let Some(line) = search_filter.lines().first()
                && !line.is_empty()
            {
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
                DatabaseExplorerState::SqlResults(_) => {
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

    /// Copy the full selected row as tab-separated values.
    pub(crate) fn copy_row_tsv(&mut self) {
        let explorer = &self.database_explorer;
        let values: Option<Vec<String>> = (|| -> Option<Vec<String>> {
            match &explorer.state {
                DatabaseExplorerState::TableData(_, _) => {
                    let table_data = explorer.table_data.as_ref()?;
                    let selected_row =
                        table_data.table.view.state.selected()?;
                    Some(
                        table_data
                            .table
                            .model
                            .items
                            .get(selected_row)?
                            .values
                            .clone(),
                    )
                }
                DatabaseExplorerState::SqlResults(_) => {
                    let table = &explorer.sql_executor.table_state;
                    let selected_row = table.view.state.selected()?;
                    Some(table.model.items.get(selected_row)?.values.clone())
                }
                DatabaseExplorerState::Connections
                | DatabaseExplorerState::Databases
                | DatabaseExplorerState::Schemas
                | DatabaseExplorerState::Tables(_)
                | DatabaseExplorerState::Columns(_, _) => None,
            }
        })();
        if let Some(values) = values {
            let tsv = values.join("\t");
            if execute!(
                std::io::stdout(),
                clipboard::CopyToClipboard {
                    content: tsv,
                    destination: clipboard::ClipboardSelection(vec![
                        clipboard::ClipboardType::Clipboard,
                    ]),
                }
            )
            .is_ok()
            {
                self.set_status("Copied row (TSV)".to_string());
            }
        }
    }

    /// Write SQL result rows to a temp TSV file.
    pub(crate) fn export_sql_results_tsv(&mut self) {
        if !matches!(
            self.database_explorer.state,
            DatabaseExplorerState::SqlResults(_)
        ) {
            return;
        }
        let executor = &self.database_explorer.sql_executor;
        if executor.table_state.model.items.is_empty() {
            self.set_status("No results to export".to_string());
            return;
        }
        let mut out = executor.column_names.join("\t");
        out.push('\n');
        for row in &executor.table_state.model.items {
            out.push_str(&row.values.join("\t"));
            out.push('\n');
        }
        let path = std::env::temp_dir()
            .join(format!("d7s_results_{}.tsv", std::process::id()));
        match std::fs::write(&path, out) {
            Ok(()) => self.set_status(format!("Wrote {}", path.display())),
            Err(e) => self.set_status(format!("Export failed: {e}")),
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

    pub(crate) fn connect_status_message(
        &mut self,
        name: &str,
        env: crate::db::connection::Environment,
    ) -> String {
        if self.showed_help_hint {
            format!("Connected to {name} ({env})")
        } else {
            self.showed_help_hint = true;
            format!("Connected to {name} ({env}) — press ? for help")
        }
    }

    fn run_editor(terminal: &mut DefaultTerminal, path: &Path) -> Result<()> {
        let editor = std::env::var("VISUAL")
            .or_else(|_| std::env::var("EDITOR"))
            .unwrap_or_else(|_| "vim".to_string());
        let (program, args) = Self::parse_editor_command(&editor);

        execute!(std::io::stdout(), DisableBracketedPaste)?;
        std::io::stdout().execute(LeaveAlternateScreen)?;
        disable_raw_mode()?;
        let mut cmd = Command::new(&program);
        cmd.args(args);
        cmd.arg(path).status()?;
        std::io::stdout().execute(EnterAlternateScreen)?;
        enable_raw_mode()?;
        execute!(std::io::stdout(), EnableBracketedPaste)?;
        terminal.clear()?;
        Ok(())
    }

    fn parse_editor_command(editor: &str) -> (String, Vec<String>) {
        let mut parts = editor.split_whitespace();
        let program = parts.next().unwrap_or("vim").to_string();
        let args = parts.map(ToString::to_string).collect();
        (program, args)
    }

    fn enter_sql_results_state(&mut self, statement: String) {
        if !matches!(
            self.database_explorer.state,
            DatabaseExplorerState::SqlResults(_)
        ) {
            let current_state = self.database_explorer.state.clone();
            self.database_explorer.previous_state = Some(current_state);
        }
        self.database_explorer.state =
            DatabaseExplorerState::SqlResults(statement);
    }

    pub(crate) async fn prepare_sql_statement_execution(
        &mut self,
        statement: String,
    ) {
        if classify_statement(&statement)
            == StatementSafety::RequiresConfirmation
        {
            self.modal_manager
                .open_sql_execution_confirmation_modal(statement);
        } else {
            self.execute_sql_statement_now(statement).await;
        }
    }

    pub(crate) async fn execute_sql_statement_now(
        &mut self,
        statement: String,
    ) {
        self.enter_sql_results_state(statement.clone());
        self.database_explorer
            .sql_executor
            .set_selected_statement(statement);
        self.execute_sql_query().await;
    }
}

/// Info related to the program
fn build_info() -> Result<String> {
    let mut lines = vec![
        format!("Name: {}", crate::app::PKG_NAME),
        format!("Version: {}", crate::app::PKG_VERSION),
    ];
    if std::env::var("D7S_DEMO").is_err() {
        let path_buf = std::env::current_dir()?;
        let cwd = path_buf.as_path().to_str().unwrap_or(".");
        lines.push(format!(
            "Path: {}",
            crate::db::connection::shorten_home_path(cwd)
        ));
    }
    Ok(lines.join("\n"))
}
