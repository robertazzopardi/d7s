use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::{
    app::App,
    app_state::DatabaseExplorerState,
    db::{Database, connection::ConnectionType},
    filtered_data::FilteredData,
    ui::{handlers::TableNavigationHandler, widgets::table::TableDataState},
    virtual_table::{VIRTUAL_TABLE_PAGE_SIZE, VirtualTableMeta},
};

impl App<'_> {
    /// Load databases from the connection
    pub async fn load_databases(&mut self) -> Result<()> {
        let explorer = &mut self.database_explorer;

        let Some(database) = explorer.database.as_mut() else {
            self.set_status("Not connected to database.");
            return Ok(());
        };

        match database.get_databases().await {
            Ok(databases) => {
                explorer.databases = Some(FilteredData::new(databases));
                explorer.state = DatabaseExplorerState::Databases;
            }
            Err(e) => {
                self.set_status(format!("Failed to load databases: {e}"));
            }
        }

        Ok(())
    }

    /// Select a database and reconnect to it
    pub async fn select_database(&mut self, database_name: &str) -> Result<()> {
        let explorer = &mut self.database_explorer;
        if explorer.database.is_some() {
            // Update connection with selected database
            explorer.connection.selected_database =
                Some(database_name.to_string());

            let db: Box<dyn Database> = match explorer.connection.r#type {
                ConnectionType::Postgres => explorer.connection.to_postgres(),
                ConnectionType::Sqlite => explorer.connection.to_sqlite(),
            };

            if db.test().await {
                explorer.database = Some(db);
                self.load_schemas().await?;
            } else {
                // TODO probably dont need database name here or at all
                self.set_status(format!(
                    "Failed to connect to database: {database_name}",
                ));
            }
        }

        Ok(())
    }

    /// Load schemas from the database
    pub async fn load_schemas(&mut self) -> Result<()> {
        let explorer = &mut self.database_explorer;
        let Some(database) = explorer.database.as_mut() else {
            self.set_status("Not connected to database");
            return Ok(());
        };

        // SQLite doesn't need the Schemas navigation step
        // Skip directly to loading tables from the default sqlite_schema
        if explorer.connection.r#type == ConnectionType::Sqlite {
            return self.load_tables("sqlite_schema").await;
        }

        match database.get_schemas().await {
            Ok(schemas) => {
                explorer.schemas = Some(FilteredData::new(schemas));
                explorer.state = DatabaseExplorerState::Schemas;
            }
            Err(e) => {
                self.set_status(format!("Failed to load schemas: {e}"));
            }
        }

        Ok(())
    }

    /// Load tables for a schema
    pub async fn load_tables(&mut self, schema_name: &str) -> Result<()> {
        let explorer = &mut self.database_explorer;
        let Some(database) = explorer.database.as_mut() else {
            self.set_status("Not connected to database");
            return Ok(());
        };

        match database.get_tables(schema_name).await {
            Ok(tables) => {
                explorer.tables = Some(FilteredData::new(tables));
                explorer.state =
                    DatabaseExplorerState::Tables(schema_name.to_string());
            }
            Err(e) => {
                self.set_status(format!("Failed to load tables: {e}"));
            }
        }

        Ok(())
    }

    /// Load columns for a table
    pub async fn load_columns(
        &mut self,
        schema_name: &str,
        table_name: &str,
    ) -> Result<()> {
        let explorer = &mut self.database_explorer;
        let Some(database) = explorer.database.as_mut() else {
            self.set_status("Not connected to database");
            return Ok(());
        };

        match database.get_columns(schema_name, table_name).await {
            Ok(columns) => {
                explorer.columns = Some(FilteredData::new(columns));
                explorer.state = DatabaseExplorerState::Columns(
                    schema_name.to_string(),
                    table_name.to_string(),
                );
            }
            Err(e) => {
                self.set_status(format!("Failed to load columns: {e}"));
            }
        }

        Ok(())
    }

    /// Load table data for a table (first page of a paged / virtual table).
    pub async fn load_table_data(
        &mut self,
        schema_name: &str,
        table_name: &str,
    ) -> Result<()> {
        let explorer = &mut self.database_explorer;
        let Some(database) = explorer.database.as_ref() else {
            self.set_status("Not connected to database");
            return Ok(());
        };

        let total_rows = database
            .get_table_row_count(schema_name, table_name)
            .await
            .ok();
        let page_size = VIRTUAL_TABLE_PAGE_SIZE;

        if let Ok((data, column_names)) = database
            .get_table_data_page(schema_name, table_name, 0, page_size)
            .await
        {
            let loaded = data.len();
            let meta =
                VirtualTableMeta::from_fetch(0, page_size, loaded, total_rows);
            let mut table = TableDataState::default();
            table.reset(data, &column_names);
            let filtered = FilteredData {
                original: table.model.items.clone(),
                table,
            };
            explorer.table_data = Some(filtered);
            explorer.table_data_virtual = Some(meta);
            explorer.state = DatabaseExplorerState::TableData(
                schema_name.to_string(),
                table_name.to_string(),
            );
        } else {
            explorer.table_data_virtual = None;
            self.set_status("Failed to load table data");
        }

        Ok(())
    }

    /// Load the next page of rows for the current table data view.
    pub async fn fetch_next_table_page(&mut self) -> Result<()> {
        let explorer = &mut self.database_explorer;
        let Some(meta) = explorer.table_data_virtual.as_ref() else {
            return Ok(());
        };
        if !meta.has_more_after {
            self.set_status("Already at last page.");
            return Ok(());
        }
        let DatabaseExplorerState::TableData(schema, table) = &explorer.state
        else {
            return Ok(());
        };
        let new_start = meta.window_start + meta.loaded_count as u64;
        let page_size = meta.page_size;
        let total_rows = meta.total_rows;
        let Some(database) = explorer.database.as_ref() else {
            return Ok(());
        };

        match database
            .get_table_data_page(schema, table, new_start, page_size)
            .await
        {
            Ok((data, column_names)) => {
                let loaded = data.len();
                let meta = VirtualTableMeta::from_fetch(
                    new_start, page_size, loaded, total_rows,
                );
                let mut table_state = TableDataState::default();
                table_state.reset(data, &column_names);
                explorer.table_data = Some(FilteredData {
                    original: table_state.model.items.clone(),
                    table: table_state,
                });
                explorer.table_data_virtual = Some(meta);
            }
            Err(e) => {
                self.set_status(format!("Failed to load page: {e}"));
            }
        }

        Ok(())
    }

    /// Load the previous page of rows for the current table data view.
    pub async fn fetch_prev_table_page(&mut self) -> Result<()> {
        let explorer = &mut self.database_explorer;
        let Some(meta) = explorer.table_data_virtual.as_ref() else {
            return Ok(());
        };
        if !meta.has_more_before {
            self.set_status("Already at first page.");
            return Ok(());
        }
        let page_size = meta.page_size;
        let new_start = meta.window_start.saturating_sub(u64::from(page_size));
        let total_rows = meta.total_rows;

        let DatabaseExplorerState::TableData(schema, table) = &explorer.state
        else {
            return Ok(());
        };
        let Some(database) = explorer.database.as_ref() else {
            return Ok(());
        };

        match database
            .get_table_data_page(schema, table, new_start, page_size)
            .await
        {
            Ok((data, column_names)) => {
                let loaded = data.len();
                let meta = VirtualTableMeta::from_fetch(
                    new_start, page_size, loaded, total_rows,
                );
                let mut table_state = TableDataState::default();
                table_state.reset(data, &column_names);
                explorer.table_data = Some(FilteredData {
                    original: table_state.model.items.clone(),
                    table: table_state,
                });
                explorer.table_data_virtual = Some(meta);
            }
            Err(e) => {
                self.set_status(format!("Failed to load page: {e}"));
            }
        }

        Ok(())
    }

    /// At the first/last row of a loaded page, j/k loads the previous/next page.
    pub async fn try_step_virtual_table_page(
        &mut self,
        key: KeyEvent,
    ) -> Result<bool> {
        if !key.modifiers.is_empty() {
            return Ok(false);
        }
        let code = key.code;
        if !matches!(
            code,
            KeyCode::Down | KeyCode::Up | KeyCode::Char('j' | 'k')
        ) {
            return Ok(false);
        }

        let edge: Option<bool> = {
            let explorer = &self.database_explorer;
            if !matches!(explorer.state, DatabaseExplorerState::TableData(..)) {
                return Ok(false);
            }
            let Some(meta) = explorer.table_data_virtual.as_ref() else {
                return Ok(false);
            };
            let Some(ref table_fd) = explorer.table_data else {
                return Ok(false);
            };
            if table_fd.is_filtered() {
                return Ok(false);
            }
            let Some(selected) = table_fd.table.view.state.selected() else {
                return Ok(false);
            };
            let len = table_fd.table.model.items.len();
            if len == 0 {
                return Ok(false);
            }

            if matches!(code, KeyCode::Char('j') | KeyCode::Down)
                && selected == len - 1
                && meta.has_more_after
            {
                Some(true)
            } else if matches!(code, KeyCode::Char('k') | KeyCode::Up)
                && selected == 0
                && meta.has_more_before
            {
                Some(false)
            } else {
                None
            }
        };

        match edge {
            Some(true) => {
                self.fetch_next_table_page().await?;
                Ok(true)
            }
            Some(false) => {
                self.fetch_prev_table_page().await?;
                if let Some(ref mut td) = self.database_explorer.table_data {
                    let last = td.table.model.items.len().saturating_sub(1);
                    td.table.view.state.select(Some(last));
                }
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Handle database navigation when Enter is pressed
    pub async fn handle_database_navigation(&mut self) -> Result<()> {
        let explorer_state = self.database_explorer.state.clone();

        match explorer_state {
            DatabaseExplorerState::Connections => {
                self.connect_to_database().await?;
            }
            DatabaseExplorerState::Databases => {
                if let Some(database_name) = self.get_selected_database_name() {
                    self.select_database(&database_name).await?;
                }
            }
            DatabaseExplorerState::Schemas => {
                if let Some(schema_name) = self.get_selected_schema_name() {
                    self.database_explorer.connection.schema =
                        Some(schema_name.clone());
                    self.load_tables(&schema_name).await?;
                }
            }
            DatabaseExplorerState::Tables(schema_name) => {
                if let Some(table_name) = self.get_selected_table_name() {
                    self.database_explorer.connection.table =
                        Some(table_name.clone());
                    self.load_table_data(&schema_name, &table_name).await?;
                }
            }
            DatabaseExplorerState::Columns(schema_name, table_name) => {
                // Toggle to data view
                let schema_name = schema_name.clone();
                let table_name = table_name.clone();
                self.load_table_data(&schema_name, &table_name).await?;
            }
            DatabaseExplorerState::TableData(_schema_name, _table_name) => {
                if let Some((column_name, cell_value)) =
                    self.get_selected_cell_value()
                {
                    self.modal_manager
                        .open_cell_value_modal(column_name, cell_value);
                }
            }
            DatabaseExplorerState::SqlResults(_) => {
                // Enter should not re-run SQL in results mode.
            }
        }
        Ok(())
    }

    /// Get the name of the currently selected database
    fn get_selected_database_name(&self) -> Option<String> {
        let explorer = &self.database_explorer;
        let databases = explorer.databases.as_ref()?;
        let selected_index = databases.table.view.state.selected()?;
        let database = databases.table.model.items.get(selected_index)?;
        Some(database.name.clone())
    }

    /// Get the name of the currently selected schema
    fn get_selected_schema_name(&self) -> Option<String> {
        let explorer = &self.database_explorer;
        let schemas = explorer.schemas.as_ref()?;
        let selected_index = schemas.table.view.state.selected()?;
        let schema = schemas.table.model.items.get(selected_index)?;
        Some(schema.name.clone())
    }

    /// Get the name of the currently selected table
    fn get_selected_table_name(&self) -> Option<String> {
        let explorer = &self.database_explorer;
        let tables = explorer.tables.as_ref()?;
        let selected_index = tables.table.view.state.selected()?;
        let table = tables.table.model.items.get(selected_index)?;
        Some(table.name.clone())
    }

    /// Get the selected cell value from table data
    fn get_selected_cell_value(&self) -> Option<(String, String)> {
        let explorer = &self.database_explorer;
        let table_data_filtered = explorer.table_data.as_ref()?;
        let table_data = &table_data_filtered.table;

        let selected_row = table_data.view.state.selected()?;
        let row = table_data.model.items.get(selected_row)?;
        let selected_col = table_data.view.state.selected_column().unwrap_or(0);
        let column_names = table_data.model.dynamic_column_names.as_ref()?;

        if selected_col >= column_names.len()
            || selected_col >= row.values.len()
        {
            return None;
        }

        let column_name = column_names.get(selected_col)?.clone();
        let cell_value = row.values.get(selected_col)?.clone();
        Some((column_name, cell_value))
    }

    /// Execute SQL query from the SQL executor
    pub(crate) async fn execute_sql_query(&mut self) {
        let sql = self
            .database_explorer
            .sql_executor
            .selected_statement()
            .unwrap_or_default()
            .trim()
            .to_string();
        if sql.is_empty() {
            self.set_status("No SQL statement selected for execution.");
            return;
        }

        let Some(database) = self.database_explorer.database.as_ref() else {
            return;
        };

        // Clear any previous results/errors before executing
        self.database_explorer.sql_executor.clear_results();

        match database.execute_sql(&sql).await {
            Ok(results) => {
                let data: Vec<Vec<String>> =
                    results.iter().map(|row| row.values.clone()).collect();
                if data.is_empty() {
                    // No data returned - show message in status bar
                    self.set_status(
                        "Query executed successfully but returned no data",
                    );
                } else if let Some(first_result) = results.first() {
                    // Has data - show results in SQL executor
                    self.database_explorer
                        .sql_executor
                        .set_results(data, &first_result.column_names);
                }
            }
            Err(e) => {
                // Error occurred - show in status bar instead of SQL executor widget
                self.set_status(format!("SQL Error: {e}"));
            }
        }
    }

    /// Go back to previous level in database navigation
    pub fn go_back_in_database(&mut self) {
        let explorer_state = self.database_explorer.state.clone();
        let explorer = &mut self.database_explorer;
        let is_sqlite = explorer.connection.r#type == ConnectionType::Sqlite;

        match explorer_state {
            DatabaseExplorerState::Connections => {
                // Nowhere to go back from connections list
            }
            DatabaseExplorerState::TableData(schema_name, _)
            | DatabaseExplorerState::Columns(schema_name, _) => {
                // Go back to tables in the same schema
                if explorer.tables.is_some() {
                    explorer.state = DatabaseExplorerState::Tables(schema_name);
                    explorer.connection.table = None;
                }
            }
            DatabaseExplorerState::Tables(_) => {
                // SQLite: Go back to connections (disconnect)
                // Postgres: Go back to schemas
                if is_sqlite {
                    self.disconnect_from_database();
                } else if explorer.schemas.is_some() {
                    explorer.state = DatabaseExplorerState::Schemas;
                    explorer.connection.schema = None;
                }
            }
            DatabaseExplorerState::Schemas => {
                // Go back to databases
                if explorer.databases.is_some() {
                    explorer.state = DatabaseExplorerState::Databases;
                }
            }
            DatabaseExplorerState::SqlResults(_) => {
                // SQLite: Go back to tables
                // Postgres: Go back to schemas
                if is_sqlite {
                    if explorer.tables.is_some() {
                        explorer.state = DatabaseExplorerState::Tables(
                            "sqlite_schema".to_string(),
                        );
                    }
                } else if explorer.schemas.is_some() {
                    explorer.state = DatabaseExplorerState::Schemas;
                }
            }
            DatabaseExplorerState::Databases => {
                // Go back to connection list (disconnect)
                self.disconnect_from_database();
                self.refresh_connections();
            }
        }
    }

    /// Handle table navigation for the current database table
    pub fn handle_database_table_navigation(&mut self, key: KeyCode) {
        match self.database_explorer.state {
            DatabaseExplorerState::Connections => {
                self.database_explorer.connections.navigate(key);
            }
            DatabaseExplorerState::SqlResults(_) => {
                TableNavigationHandler::navigate_table(
                    &self.database_explorer.sql_executor.table_state.model,
                    &mut self.database_explorer.sql_executor.table_state.view,
                    key,
                );
            }
            DatabaseExplorerState::Databases
            | DatabaseExplorerState::Schemas
            | DatabaseExplorerState::Tables(_)
            | DatabaseExplorerState::Columns(..)
            | DatabaseExplorerState::TableData(..) => {
                self.database_explorer.navigate_current(key);
            }
        }
    }
}
