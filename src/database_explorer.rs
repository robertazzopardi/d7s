use color_eyre::Result;
use crossterm::event::KeyCode;
use d7s_db::Database;
use d7s_ui::{
    handlers::TableNavigationHandler, widgets::table::TableDataState,
};

use crate::{
    app::App, app_state::DatabaseExplorerState, filtered_data::FilteredData,
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
            explorer.connection.database = database_name.to_string();

            // Create new Postgres connection with selected database
            let postgres = explorer.connection.to_postgres();

            // Test the connection to the selected database
            if postgres.test().await {
                // Replace the database client with the new one
                explorer.database = Some(Box::new(postgres));

                // Load schemas for the selected database
                self.load_schemas().await?;
            } else {
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

    /// Load table data for a table
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

        if let Ok((data, column_names)) = database
            .get_table_data_with_columns(schema_name, table_name)
            .await
        {
            let mut table = TableDataState::default();
            table.reset(data, &column_names);
            // Convert to FilteredData
            let filtered = FilteredData {
                original: table.model.items.clone(),
                table,
            };
            explorer.table_data = Some(filtered);
            explorer.state = DatabaseExplorerState::TableData(
                schema_name.to_string(),
                table_name.to_string(),
            );
        } else {
            self.set_status("Failed to load table data");
        }

        Ok(())
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
            DatabaseExplorerState::SqlExecutor => {
                self.execute_sql_query().await;
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
    async fn execute_sql_query(&mut self) {
        let sql = self.sql_executor.sql_input().trim().to_string();
        if sql.is_empty() {
            return;
        }

        let explorer = &self.database_explorer;
        let Some(database) = explorer.database.as_ref() else {
            return;
        };

        // Clear any previous results/errors before executing
        self.sql_executor.clear_results();

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
                    self.sql_executor
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
                // Go back to schemas
                if explorer.schemas.is_some() {
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
            DatabaseExplorerState::SqlExecutor => {
                // Go back to schemas
                if explorer.schemas.is_some() {
                    explorer.state = DatabaseExplorerState::Schemas;
                }
            }
            DatabaseExplorerState::Databases => {
                // Go back to connection list (disconnect)
                self.disconnect_from_database();
            }
        }
    }

    /// Handle table navigation for the current database table
    pub fn handle_database_table_navigation(&mut self, key: KeyCode) {
        let explorer_state = self.database_explorer.state.clone();

        match explorer_state {
            DatabaseExplorerState::Connections => {
                TableNavigationHandler::navigate_table(
                    &mut self.connections.table,
                    key,
                );
            }
            DatabaseExplorerState::Databases => {
                if let Some(ref mut databases) =
                    self.database_explorer.databases
                {
                    TableNavigationHandler::navigate_table(
                        &mut databases.table,
                        key,
                    );
                }
            }
            DatabaseExplorerState::Schemas => {
                if let Some(ref mut schemas) = self.database_explorer.schemas {
                    TableNavigationHandler::navigate_table(
                        &mut schemas.table,
                        key,
                    );
                }
            }
            DatabaseExplorerState::Tables(_) => {
                if let Some(ref mut tables) = self.database_explorer.tables {
                    TableNavigationHandler::navigate_table(
                        &mut tables.table,
                        key,
                    );
                }
            }
            DatabaseExplorerState::Columns(_, _) => {
                if let Some(ref mut columns) = self.database_explorer.columns {
                    TableNavigationHandler::navigate_table(
                        &mut columns.table,
                        key,
                    );
                }
            }
            DatabaseExplorerState::TableData(_, _) => {
                if let Some(ref mut table_data) =
                    self.database_explorer.table_data
                {
                    TableNavigationHandler::navigate_table(
                        &mut table_data.table,
                        key,
                    );
                }
            }
            DatabaseExplorerState::SqlExecutor => {
                TableNavigationHandler::navigate_table(
                    &mut self.sql_executor.table_state,
                    key,
                );
            }
        }
    }
}
