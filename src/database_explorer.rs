use color_eyre::Result;
use crossterm::event::KeyCode;
use d7s_db::Database;
use d7s_ui::{handlers::TableNavigationHandler, widgets::table::DataTable};

use crate::{
    app::App, app_state::DatabaseExplorerState, filtered_data::FilteredData,
};

impl App<'_> {
    /// Load schemas from the database
    pub async fn load_schemas(&mut self) -> Result<()> {
        if let Some(explorer) = &mut self.database_explorer {
            match explorer.database.get_schemas().await {
                Ok(schemas) => {
                    explorer.schemas = Some(FilteredData::new(schemas));
                    explorer.state = DatabaseExplorerState::Schemas;
                }
                Err(e) => {
                    self.set_status(format!("Failed to load schemas: {e}"));
                }
            }
        }
        Ok(())
    }

    /// Load tables for a schema
    pub async fn load_tables(&mut self, schema_name: &str) -> Result<()> {
        if let Some(explorer) = &mut self.database_explorer {
            match explorer.database.get_tables(schema_name).await {
                Ok(tables) => {
                    explorer.tables = Some(FilteredData::new(tables));
                    explorer.state =
                        DatabaseExplorerState::Tables(schema_name.to_string());
                }
                Err(e) => {
                    self.set_status(format!("Failed to load tables: {e}"));
                }
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
        if let Some(explorer) = &mut self.database_explorer {
            match explorer.database.get_columns(schema_name, table_name).await {
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
        }
        Ok(())
    }

    /// Load table data for a table
    pub async fn load_table_data(
        &mut self,
        schema_name: &str,
        table_name: &str,
    ) -> Result<()> {
        if let Some(explorer) = &mut self.database_explorer {
            if let Ok((data, column_names)) = explorer
                .database
                .get_table_data_with_columns(schema_name, table_name)
                .await
            {
                let table = DataTable::from_raw_data(data, &column_names);
                // Convert to FilteredData
                let filtered = FilteredData {
                    original: table.items.clone(),
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
        }
        Ok(())
    }

    /// Handle database navigation when Enter is pressed
    pub async fn handle_database_navigation(&mut self) -> Result<()> {
        let explorer_state =
            self.database_explorer.as_ref().map(|e| e.state.clone());

        match explorer_state {
            Some(DatabaseExplorerState::Schemas) => {
                if let Some(schema_name) = self.get_selected_schema_name() {
                    if let Some(explorer) = &mut self.database_explorer {
                        explorer.connection.schema = Some(schema_name.clone());
                    }
                    self.load_tables(&schema_name).await?;
                }
            }
            Some(DatabaseExplorerState::Tables(schema_name)) => {
                if let Some(table_name) = self.get_selected_table_name() {
                    if let Some(explorer) = &mut self.database_explorer {
                        explorer.connection.table = Some(table_name.clone());
                    }
                    self.load_table_data(&schema_name, &table_name).await?;
                }
            }
            Some(DatabaseExplorerState::Columns(schema_name, table_name)) => {
                // Toggle to data view
                let schema_name = schema_name.clone();
                let table_name = table_name.clone();
                self.load_table_data(&schema_name, &table_name).await?;
            }
            Some(DatabaseExplorerState::TableData(
                _schema_name,
                _table_name,
            )) => {
                if let Some((column_name, cell_value)) =
                    self.get_selected_cell_value()
                {
                    self.modal_manager
                        .open_cell_value_modal(column_name, cell_value);
                }
            }
            Some(DatabaseExplorerState::SqlExecutor) => {
                self.execute_sql_query().await;
            }
            None => {
                // Load schemas if not loaded yet
                self.load_schemas().await?;
            }
        }
        Ok(())
    }

    /// Get the name of the currently selected schema
    fn get_selected_schema_name(&self) -> Option<String> {
        let explorer = self.database_explorer.as_ref()?;
        let schemas = explorer.schemas.as_ref()?;
        let selected_index = schemas.table.state.selected()?;
        let schema = schemas.table.items.get(selected_index)?;
        Some(schema.name.clone())
    }

    /// Get the name of the currently selected table
    fn get_selected_table_name(&self) -> Option<String> {
        let explorer = self.database_explorer.as_ref()?;
        let tables = explorer.tables.as_ref()?;
        let selected_index = tables.table.state.selected()?;
        let table = tables.table.items.get(selected_index)?;
        Some(table.name.clone())
    }

    /// Get the selected cell value from table data
    fn get_selected_cell_value(&self) -> Option<(String, String)> {
        let explorer = self.database_explorer.as_ref()?;
        let table_data_filtered = explorer.table_data.as_ref()?;
        let table_data = &table_data_filtered.table;

        let selected_row = table_data.state.selected()?;
        let row = table_data.items.get(selected_row)?;
        let selected_col = table_data.state.selected_column().unwrap_or(0);
        let column_names = table_data.dynamic_column_names.as_ref()?;

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
        let sql = self.sql_executor.sql_input().trim();
        if sql.is_empty() {
            return;
        }

        let Some(explorer) = &self.database_explorer else {
            return;
        };

        match explorer.database.execute_sql(sql).await {
            Ok(results) => {
                let data: Vec<Vec<String>> =
                    results.iter().map(|row| row.values.clone()).collect();
                if let Some(first_result) = results.first() {
                    self.sql_executor
                        .set_results(data, &first_result.column_names);
                }
            }
            Err(e) => {
                self.sql_executor.set_error(e.to_string());
            }
        }
    }

    /// Go back to previous level in database navigation
    pub fn go_back_in_database(&mut self) {
        let explorer_state =
            self.database_explorer.as_ref().map(|e| e.state.clone());

        match explorer_state {
            Some(
                DatabaseExplorerState::TableData(schema_name, _)
                | DatabaseExplorerState::Columns(schema_name, _),
            ) => {
                // Go back to tables in the same schema
                if let Some(explorer) = &mut self.database_explorer
                    && explorer.tables.is_some()
                {
                    explorer.state = DatabaseExplorerState::Tables(schema_name);
                    explorer.connection.table = None;
                }
            }
            Some(DatabaseExplorerState::Tables(_)) => {
                // Go back to schemas
                if let Some(explorer) = &mut self.database_explorer
                    && explorer.schemas.is_some()
                {
                    explorer.state = DatabaseExplorerState::Schemas;
                    explorer.connection.schema = None;
                }
            }
            Some(DatabaseExplorerState::SqlExecutor) => {
                // Go back to schemas
                if let Some(explorer) = &mut self.database_explorer
                    && explorer.schemas.is_some()
                {
                    explorer.state = DatabaseExplorerState::Schemas;
                }
            }
            Some(DatabaseExplorerState::Schemas) | None => {
                // Go back to connection list (disconnect)
                self.disconnect_from_database();
            }
        }
    }

    /// Handle table navigation for the current database table
    pub fn handle_database_table_navigation(&mut self, key: KeyCode) {
        let explorer_state =
            self.database_explorer.as_ref().map(|e| e.state.clone());

        match explorer_state {
            Some(DatabaseExplorerState::Schemas) => {
                if let Some(explorer) = &mut self.database_explorer
                    && let Some(ref mut schemas) = explorer.schemas
                {
                    TableNavigationHandler::navigate_table(
                        &mut schemas.table,
                        key,
                    );
                }
            }
            Some(DatabaseExplorerState::Tables(_)) => {
                if let Some(explorer) = &mut self.database_explorer
                    && let Some(ref mut tables) = explorer.tables
                {
                    TableNavigationHandler::navigate_table(
                        &mut tables.table,
                        key,
                    );
                }
            }
            Some(DatabaseExplorerState::Columns(_, _)) => {
                if let Some(explorer) = &mut self.database_explorer
                    && let Some(ref mut columns) = explorer.columns
                {
                    TableNavigationHandler::navigate_table(
                        &mut columns.table,
                        key,
                    );
                }
            }
            Some(DatabaseExplorerState::TableData(_, _)) => {
                if let Some(explorer) = &mut self.database_explorer
                    && let Some(ref mut table_data) = explorer.table_data
                {
                    TableNavigationHandler::navigate_table(
                        &mut table_data.table,
                        key,
                    );
                }
            }
            Some(DatabaseExplorerState::SqlExecutor) => {
                // If we have results, handle table navigation
                if self.sql_executor.table_widget.is_some() {
                    TableNavigationHandler::handle_sql_results_navigation(
                        &mut self.sql_executor,
                        key,
                    );
                }
            }
            None => {}
        }
    }
}
