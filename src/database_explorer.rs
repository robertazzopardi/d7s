use color_eyre::Result;
use crossterm::event::KeyCode;
use d7s_db::Database;
use d7s_ui::{
    handlers::TableNavigationHandler,
    widgets::table::DataTable,
};

use crate::{app::App, app_state::DatabaseExplorerState, filtered_data::FilteredData};

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
                    explorer.state = DatabaseExplorerState::Tables(
                        schema_name.to_string(),
                    );
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
        let explorer_state = self.database_explorer.as_ref().map(|e| e.state.clone());

        match explorer_state {
            Some(DatabaseExplorerState::Schemas) => {
                // Navigate to tables in selected schema
                let schema_name = if let Some(explorer) = &self.database_explorer {
                    if let Some(ref schemas) = explorer.schemas {
                        let Some(selected_index) = schemas.table.state.selected() else {
                            return Ok(());
                        };
                        if selected_index >= schemas.table.items.len() {
                            return Ok(());
                        }
                        let Some(schema) = schemas.table.items.get(selected_index) else {
                            return Ok(());
                        };
                        schema.name.clone()
                    } else {
                        return Ok(());
                    }
                } else {
                    return Ok(());
                };

                if let Some(explorer) = &mut self.database_explorer {
                    explorer.connection.schema = Some(schema_name.clone());
                }

                self.load_tables(&schema_name).await?;
            }
            Some(DatabaseExplorerState::Tables(schema_name)) => {
                // Navigate to table data in selected table (show data first, not columns)
                let table_name = if let Some(explorer) = &self.database_explorer {
                    if let Some(ref tables) = explorer.tables {
                        let Some(selected_index) = tables.table.state.selected() else {
                            return Ok(());
                        };
                        if selected_index >= tables.table.items.len() {
                            return Ok(());
                        }
                        let Some(table) = tables.table.items.get(selected_index) else {
                            return Ok(());
                        };
                        table.name.clone()
                    } else {
                        return Ok(());
                    }
                } else {
                    return Ok(());
                };

                if let Some(explorer) = &mut self.database_explorer {
                    explorer.connection.table = Some(table_name.clone());
                }

                self.load_table_data(&schema_name, &table_name).await?;
            }
            Some(DatabaseExplorerState::Columns(schema_name, table_name)) => {
                // Toggle to data view
                let schema_name = schema_name.clone();
                let table_name = table_name.clone();
                self.load_table_data(&schema_name, &table_name).await?;
            }
            Some(DatabaseExplorerState::TableData(_schema_name, _table_name)) => {
                // Show cell value in dialog if a cell is selected
                if let Some(explorer) = &self.database_explorer {
                    if let Some(ref table_data_filtered) = explorer.table_data {
                        let table_data = &table_data_filtered.table;

                        let Some(selected_row) = table_data.state.selected() else {
                            return Ok(());
                        };
                        if selected_row >= table_data.items.len() {
                            return Ok(());
                        }

                        let selected_col = table_data.state.selected_column().unwrap_or(0);
                        let Some(ref column_names) = table_data.dynamic_column_names else {
                            return Ok(());
                        };

                        if selected_col >= column_names.len() {
                            return Ok(());
                        }

                        let row_values_len = table_data
                            .items
                            .get(selected_row)
                            .map(|row| row.values.len())
                            .unwrap_or_default();

                        if selected_col >= row_values_len {
                            return Ok(());
                        }

                        let column_name = column_names
                            .get(selected_col)
                            .map(String::clone)
                            .unwrap_or_else(|| "Could not get column name.".to_string());

                        let cell_value = table_data
                            .items
                            .get(selected_row)
                            .and_then(|item| item.values.get(selected_col))
                            .map(String::clone)
                            .unwrap_or_else(|| "Could not get cell value.".to_string());

                        self.modal_manager
                            .open_cell_value_modal(column_name, cell_value);
                    }
                }
            }
            Some(DatabaseExplorerState::SqlExecutor) => {
                // Execute SQL query
                if !self.sql_executor.sql_input().trim().is_empty() {
                    if let Some(explorer) = &self.database_explorer {
                        match explorer
                            .database
                            .execute_sql(self.sql_executor.sql_input())
                            .await
                        {
                            Ok(results) => {
                                let data: Vec<Vec<String>> = results
                                    .iter()
                                    .map(|row| row.values.clone())
                                    .collect();

                                let column_names = results
                                    .first()
                                    .map(|result| result.column_names.clone());

                                if let Some(names) = column_names {
                                    self.sql_executor.set_results(data, &names);
                                }
                            }
                            Err(e) => {
                                self.sql_executor.set_error(e.to_string());
                            }
                        }
                    }
                }
            }
            None => {
                // Load schemas if not loaded yet
                self.load_schemas().await?;
            }
        }
        Ok(())
    }

    /// Go back to previous level in database navigation
    pub fn go_back_in_database(&mut self) {
        let explorer_state = self.database_explorer.as_ref().map(|e| e.state.clone());

        match explorer_state {
            Some(
                DatabaseExplorerState::TableData(schema_name, _)
                | DatabaseExplorerState::Columns(schema_name, _),
            ) => {
                // Go back to tables in the same schema
                if let Some(explorer) = &mut self.database_explorer {
                    if explorer.tables.is_some() {
                        explorer.state = DatabaseExplorerState::Tables(schema_name.clone());
                        explorer.connection.table = None;
                    }
                }
            }
            Some(DatabaseExplorerState::Tables(_)) => {
                // Go back to schemas
                if let Some(explorer) = &mut self.database_explorer {
                    if explorer.schemas.is_some() {
                        explorer.state = DatabaseExplorerState::Schemas;
                        explorer.connection.schema = None;
                    }
                }
            }
            Some(DatabaseExplorerState::SqlExecutor) => {
                // Go back to schemas
                if let Some(explorer) = &mut self.database_explorer {
                    if explorer.schemas.is_some() {
                        explorer.state = DatabaseExplorerState::Schemas;
                    }
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
        let explorer_state = self.database_explorer.as_ref().map(|e| e.state.clone());

        match explorer_state {
            Some(DatabaseExplorerState::Schemas) => {
                if let Some(explorer) = &mut self.database_explorer {
                    if let Some(ref mut schemas) = explorer.schemas {
                        TableNavigationHandler::navigate_table(&mut schemas.table, key);
                    }
                }
            }
            Some(DatabaseExplorerState::Tables(_)) => {
                if let Some(explorer) = &mut self.database_explorer {
                    if let Some(ref mut tables) = explorer.tables {
                        TableNavigationHandler::navigate_table(&mut tables.table, key);
                    }
                }
            }
            Some(DatabaseExplorerState::Columns(_, _)) => {
                if let Some(explorer) = &mut self.database_explorer {
                    if let Some(ref mut columns) = explorer.columns {
                        TableNavigationHandler::navigate_table(&mut columns.table, key);
                    }
                }
            }
            Some(DatabaseExplorerState::TableData(_, _)) => {
                if let Some(explorer) = &mut self.database_explorer {
                    if let Some(ref mut table_data) = explorer.table_data {
                        TableNavigationHandler::navigate_table(&mut table_data.table, key);
                    }
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
