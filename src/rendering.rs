use d7s_db::TableData;
use d7s_ui::sql_executor::SqlExecutor;
use d7s_ui::widgets::top_bar_view::TopBarView;
use ratatui::prelude::Position;
use ratatui::{
    Frame,
    prelude::*,
    widgets::{Block, Borders},
};

use crate::{
    app::{APP_NAME, App, TOPBAR_HEIGHT_PERCENT},
    app_state::{AppState, DatabaseExplorerState},
    filtered_data::FilteredData,
};

impl App<'_> {
    /// Renders the user interface.
    ///
    /// This is where you add new widgets. See the following resources for more information:
    ///
    /// - <https://docs.rs/ratatui/latest/ratatui/widgets/index.html>
    /// - <https://github.com/ratatui/ratatui/tree/main/ratatui-widgets/examples>
    pub fn render(&mut self, frame: &mut Frame) {
        // Split layout: top bar, main content, and status line
        // Status line gets fixed 1 row, main content takes the rest
        let mut main_layout = vec![
            Constraint::Percentage(TOPBAR_HEIGHT_PERCENT),
            Constraint::Min(0),
        ];

        if !self.status_line.message().is_empty() {
            main_layout.push(Constraint::Length(1));
        }

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(main_layout)
            .split(frame.area());
        let first_layout =
            layout.first().copied().unwrap_or_else(Rect::default);

        let current_connection = self
            .database_explorer
            .as_ref()
            .map(|e| e.connection.clone())
            .unwrap_or_default();
        frame.render_widget(
            TopBarView {
                current_connection,
                hotkeys: &self.hotkeys,
                app_name: APP_NAME,
            },
            first_layout,
        );

        // Create the main content area (layout[1] is the middle section)
        let layout_rect =
            layout.get(1).copied().unwrap_or_else(|| frame.area());
        let main_area = if self.search_filter.is_active {
            // If search filter is active, create a layout with search filter at top
            let search_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Search filter height
                    Constraint::Min(0),    // Remaining space for table
                ])
                .split(layout_rect);

            let search_layout_rect =
                search_layout.first().copied().unwrap_or_else(Rect::default);

            // Render search filter
            frame.render_stateful_widget(
                self.search_filter.clone(),
                search_layout_rect,
                &mut (),
            );

            search_layout.get(1).copied().unwrap_or_else(Rect::default)
        } else {
            layout_rect
        };

        match self.state {
            AppState::ConnectionList => {
                // Create the inner block
                let block = Block::new()
                    .borders(Borders::ALL)
                    .title(" Connections ")
                    .title_alignment(Alignment::Center);

                // Get the inner area of the block (content area)
                let inner_area = block.inner(main_area);

                // Render the block itself (borders and title)
                frame.render_widget(block, main_area);

                // Render content inside the block
                // Use the data table directly
                frame.render_stateful_widget(
                    self.connections.table.clone(),
                    inner_area,
                    &mut self.connections.table.state,
                );
            }
            AppState::DatabaseConnected => {
                // Create the inner block for database view
                let block = Block::new()
                    .borders(Borders::ALL)
                    .title(self.get_database_title())
                    .title_alignment(Alignment::Center);

                // Get the inner area of the block (content area)
                let inner_area = block.inner(main_area);

                // Render the block itself (borders and title)
                frame.render_widget(block, main_area);

                // Render the appropriate table based on explorer state
                self.render_database_table(frame, inner_area);
            }
        }

        // Render status line at the bottom
        if !self.status_line.message().is_empty()
            && let Some(status_layout) = layout.get(2)
        {
            frame.render_widget(self.status_line.clone(), *status_layout);
        }

        // Render modals using the modal manager
        self.render_modals(frame);
    }

    /// Render all active modals
    pub fn render_modals(&self, frame: &mut Frame) {
        let area = frame.area();

        if let Some(modal) = self.modal_manager.get_connection_modal() {
            frame.render_widget(modal.clone(), area);
        }

        if let Some(modal) = self.modal_manager.get_confirmation_modal() {
            frame.render_widget(modal.clone(), area);
        }

        if let Some(modal) = self.modal_manager.get_cell_value_modal() {
            frame.render_widget(modal.clone(), area);
        }

        if let Some(modal) = self.modal_manager.get_password_modal() {
            frame.render_widget(modal.clone(), area);
        }
    }

    /// Render the appropriate database table based on explorer state
    pub fn render_database_table(&mut self, frame: &mut Frame, area: Rect) {
        if let Some(explorer) = &self.database_explorer {
            match &explorer.state {
                DatabaseExplorerState::Databases => {
                    render_filtered_data_table(
                        frame,
                        explorer.databases.as_ref(),
                        area,
                    );
                }
                DatabaseExplorerState::Schemas => {
                    render_filtered_data_table(
                        frame,
                        explorer.schemas.as_ref(),
                        area,
                    );
                }
                DatabaseExplorerState::Tables(_) => {
                    render_filtered_data_table(
                        frame,
                        explorer.tables.as_ref(),
                        area,
                    );
                }
                DatabaseExplorerState::Columns(_, _) => {
                    render_filtered_data_table(
                        frame,
                        explorer.columns.as_ref(),
                        area,
                    );
                }
                DatabaseExplorerState::TableData(_, _) => {
                    render_filtered_data_table(
                        frame,
                        explorer.table_data.as_ref(),
                        area,
                    );
                }
                DatabaseExplorerState::SqlExecutor => {
                    frame.render_stateful_widget(
                        SqlExecutor,
                        area,
                        &mut self.sql_executor,
                    );

                    // Set cursor position if SQL executor is active and showing input
                    if self.sql_executor.is_active
                        && self.sql_executor.results.is_none()
                        && self.sql_executor.error_message.is_none()
                    {
                        let cursor_pos = self.sql_executor.cursor_position();
                        let text = self.sql_executor.sql_input();

                        // Calculate cursor position accounting for text wrapping
                        // Paragraph wraps text at area width
                        let area_width = area.width as usize;

                        if area_width > 0 {
                            // Get characters before cursor
                            let chars_before_cursor: Vec<char> =
                                text.chars().take(cursor_pos).collect();

                            // Calculate which line the cursor is on by simulating wrapping
                            let mut current_line = 0;
                            let mut current_line_length = 0;

                            for _ch in &chars_before_cursor {
                                if current_line_length >= area_width {
                                    current_line += 1;
                                    current_line_length = 0;
                                }
                                current_line_length += 1;
                            }

                            if let Ok(line_y) = u16::try_from(current_line)
                                && let Ok(line_x) =
                                    u16::try_from(current_line_length)
                            {
                                // Calculate x position on the current line
                                // Clamp to area bounds
                                let cursor_x = (area.x + line_x)
                                    .min(area.x + area.width.saturating_sub(1));
                                let cursor_y = (area.y + line_y).min(
                                    area.y + area.height.saturating_sub(1),
                                );

                                frame.set_cursor_position(Position::new(
                                    cursor_x, cursor_y,
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    // TODO use an impl for this
    /// Get the title for the database view based on current state
    pub fn get_database_title(&self) -> String {
        self.database_explorer.as_ref().map_or_else(
            || " Database Explorer ".to_string(),
            |explorer| match &explorer.state {
                DatabaseExplorerState::Databases => " Databases ".to_string(),
                DatabaseExplorerState::Schemas => " Schemas ".to_string(),
                DatabaseExplorerState::Tables(schema) => {
                    format!(" {schema} ")
                }
                DatabaseExplorerState::Columns(schema, table)
                | DatabaseExplorerState::TableData(schema, table) => {
                    format!(" {schema}.{table} ")
                }
                DatabaseExplorerState::SqlExecutor => {
                    " SQL Executor ".to_string()
                }
            },
        )
    }
}

fn render_filtered_data_table<T: TableData + Clone + std::fmt::Debug>(
    frame: &mut Frame,
    filtered_data: Option<&FilteredData<T>>,
    area: Rect,
) {
    if let Some(filtered_data) = filtered_data {
        frame.render_stateful_widget(
            filtered_data.table.clone(),
            area,
            &mut filtered_data.table.state.clone(),
        );
    }
}
