use ratatui::{
    Frame,
    prelude::*,
    widgets::{Block, Borders},
};

use crate::{
    app::{APP_NAME, App},
    app_state::DatabaseExplorerState,
    db::{TableData, connection::Connection},
    filtered_data::FilteredData,
    ui::{
        sql_executor::SqlExecutor,
        widgets::{
            modal::ConnectionModalWidget, table::DataTable,
            top_bar_view::TopBarView,
        },
    },
};

const TOPBAR_HEIGHT: u16 = 6;

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
        let mut main_layout =
            vec![Constraint::Length(TOPBAR_HEIGHT), Constraint::Min(0)];

        if !self.status_line.message().is_empty() {
            main_layout.push(Constraint::Length(1));
        }

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(main_layout)
            .split(frame.area());
        let first_layout =
            layout.first().copied().unwrap_or_else(Rect::default);

        let (current_connection, build_info) = if matches!(
            self.database_explorer.state,
            DatabaseExplorerState::Connections
        ) {
            (&Connection::default(), Some(self.build_info.clone()))
        } else {
            // Show connection details when connected
            (&self.database_explorer.connection, None)
        };
        frame.render_widget(
            TopBarView {
                current_connection,
                hotkeys: &self.hotkeys,
                app_name: APP_NAME,
                build_info,
            },
            first_layout,
        );

        // Create the main content area (layout[1] is the middle section)
        let layout_rect =
            layout.get(1).copied().unwrap_or_else(|| frame.area());
        let main_area = if self.search_filter.is_some() {
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
            if let Some(textarea) = &self.search_filter {
                frame.render_widget(textarea, search_layout_rect);
            }

            search_layout.get(1).copied().unwrap_or_else(Rect::default)
        } else {
            layout_rect
        };

        // Use explorer state for title and content (Connections uses same path as other states)
        let title = self.database_explorer.state.to_string();
        let block = Block::new()
            .borders(Borders::ALL)
            .title(title)
            .title_alignment(Alignment::Center);

        let inner_area = block.inner(main_area);
        frame.render_widget(block, main_area);
        self.render_database_table(frame, inner_area);

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
    pub fn render_modals(&mut self, frame: &mut Frame) {
        let area = frame.area();

        if let Some(modal) = self.modal_manager.get_connection_modal_mut() {
            frame.render_stateful_widget(ConnectionModalWidget, area, modal);
        }

        if let Some(modal) = self.modal_manager.get_confirmation_modal() {
            frame.render_widget(modal.clone(), area);
        }

        if let Some(modal) =
            self.modal_manager.get_sql_execution_confirmation_modal()
        {
            frame.render_widget(modal.clone(), area);
        }

        if let Some(modal) = self.modal_manager.get_sql_query_selection_modal()
        {
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
        let explorer = &self.database_explorer;
        match &explorer.state {
            DatabaseExplorerState::Connections => {
                frame.render_stateful_widget(
                    DataTable::<Connection>::default(),
                    area,
                    &mut self.database_explorer.connections.table,
                );
            }
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
            DatabaseExplorerState::SqlResults(_) => {
                frame.render_stateful_widget(
                    SqlExecutor,
                    area,
                    &mut self.database_explorer.sql_executor,
                );
            }
        }
    }
}

fn render_filtered_data_table<T: TableData + Clone + std::fmt::Debug>(
    frame: &mut Frame,
    filtered_data: Option<&FilteredData<T>>,
    area: Rect,
) {
    if let Some(filtered_data) = filtered_data {
        frame.render_stateful_widget(
            DataTable::<T>::default(),
            area,
            &mut filtered_data.table.clone(),
        );
    }
}
