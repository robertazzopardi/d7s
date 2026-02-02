use std::{fmt::Display, str::FromStr};

use crossterm::event::{KeyCode, KeyEvent};
use d7s_db::{TableData, connection::Connection};
use ratatui::{
    prelude::{
        Alignment, Buffer, Constraint, Direction, Layout, Line, Rect, Widget,
    },
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Clear, Paragraph, StatefulWidget},
};
use tui_menu::{MenuEvent, MenuItem, MenuState};

use crate::widgets::buttons::Buttons;

// Modal dimension constants
const CONNECTION_MODAL_WIDTH: u16 = 40;
const CONFIRMATION_MODAL_WIDTH: u16 = 50;
const CONFIRMATION_MODAL_HEIGHT: u16 = 8;
const PASSWORD_MODAL_WIDTH: u16 = 50;
const PASSWORD_MODAL_HEIGHT: u16 = 8;

#[derive(Clone, Copy, Debug, Default)]
pub enum Mode {
    #[default]
    New,
    Edit,
}

#[derive(Clone, Debug, Default)]
pub enum ModalType {
    #[default]
    Connection,
    Confirmation,
    CellValue,
    Password,
}

#[derive(Clone, Debug, Default)]
pub enum TestResult {
    #[default]
    NotTested,
    Testing,
    Success,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct ModalField {
    pub label: &'static str,
    pub value: String,
    pub is_focused: bool,
    /// When set, this field is a dropdown; value must be one of these options.
    pub options: Option<Vec<&'static str>>,
}

impl ModalField {
    #[must_use]
    pub fn new(label: &'static str) -> Self {
        Self {
            label,
            value: String::new(),
            is_focused: false,
            options: None,
        }
    }

    pub const fn set_focus(&mut self, focused: bool) {
        self.is_focused = focused;
    }

    pub fn add_char(&mut self, c: char) {
        if self.options.is_none() {
            self.value.push(c);
        }
    }

    pub fn remove_char(&mut self) {
        if self.options.is_none() {
            self.value.pop();
        }
    }

    /// Set dropdown options. If value is empty, sets value to first option.
    pub fn set_options(&mut self, options: Vec<&'static str>) {
        if !options.is_empty() {
            let first = options[0].to_string();
            self.options = Some(options);
            if self.value.is_empty() {
                self.value = first;
            }
        }
    }

    /// Ensure value is one of the options. Sets to first option if invalid.
    pub fn clamp_to_options(&mut self) {
        if let Some(ref opts) = self.options {
            if !opts.is_empty() && !opts.iter().any(|o| *o == self.value) {
                self.value = opts[0].to_string();
            }
        }
    }

    #[must_use]
    pub const fn is_dropdown(&self) -> bool {
        self.options.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PasswordStorageType {
    #[default]
    Keyring,
    DontSave,
}

impl Display for PasswordStorageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Keyring => write!(f, "keyring"),
            Self::DontSave => write!(f, "dont_save"),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct PasswordStorageTypeError;

impl FromStr for PasswordStorageType {
    type Err = PasswordStorageTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "keyring" => Self::Keyring,
            _ => Self::DontSave,
        })
    }
}

#[derive(Default)]
pub struct Modal {
    pub fields: Vec<ModalField>,
    pub current_field: usize,
    pub is_open: bool,
    pub selected_button: usize,
    pub mode: Mode,
    pub test_result: TestResult,
    pub original_name: Option<String>,
    pub password_storage: PasswordStorageType,
    /// When Some(field_index), that dropdown field's menu is open (tui-menu).
    pub dropdown_open: Option<usize>,
    /// tui-menu state when a dropdown is open (not Debug).
    pub menu_state: Option<MenuState<&'static str>>,
}

impl std::fmt::Debug for Modal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Modal")
            .field("fields", &self.fields)
            .field("current_field", &self.current_field)
            .field("is_open", &self.is_open)
            .field("selected_button", &self.selected_button)
            .field("mode", &self.mode)
            .field("test_result", &self.test_result)
            .field("original_name", &self.original_name)
            .field("password_storage", &self.password_storage)
            .field("dropdown_open", &self.dropdown_open)
            .field("menu_state", &self.menu_state.is_some())
            .finish()
    }
}

#[derive(Default, Debug, Clone)]
pub struct ConfirmationModal {
    pub is_open: bool,
    pub selected_button: usize,
    pub message: String,
    pub connection: Option<Connection>,
}

#[derive(Default, Debug, Clone)]
pub struct CellValueModal {
    pub is_open: bool,
    pub column_name: String,
    pub cell_value: String,
}

#[derive(Debug, Clone)]
pub struct PasswordModal {
    pub is_open: bool,
    pub password: String,
    pub connection: Option<Connection>,
    pub prompt: String,
    cursor_position: usize,
    selected_button: usize,
}

impl Modal {
    #[must_use]
    pub fn new(_connection: Connection, mode: Mode) -> Self {
        let mut fields: Vec<ModalField> = Connection::cols()
            .iter()
            .map(|c| ModalField::new(c))
            .collect();

        for field in &mut fields {
            match field.label {
                "Type" => field.set_options(vec!["postgres", "sqlite"]),
                "Environment" => {
                    field.set_options(vec!["dev", "staging", "prod"])
                }
                _ => {}
            }
        }

        let mut modal = Self {
            fields,
            current_field: 0,
            is_open: false,
            selected_button: 0,
            mode,
            test_result: TestResult::NotTested,
            original_name: None,
            password_storage: PasswordStorageType::default(),
            dropdown_open: None,
            menu_state: None,
        };

        if !modal.fields.is_empty() {
            modal.fields[0].set_focus(true);
        }

        modal
    }

    pub const fn toggle_password_storage(&mut self) {
        self.password_storage = match self.password_storage {
            PasswordStorageType::Keyring => PasswordStorageType::DontSave,
            PasswordStorageType::DontSave => PasswordStorageType::Keyring,
        };
    }

    pub fn open(&mut self) {
        self.is_open = true;
        self.current_field = 0;

        // Clear all fields
        for field in &mut self.fields {
            field.value.clear();
            field.set_focus(false);
        }

        // Set default values for the connection modal
        if self.fields.len() >= 5 {
            if self.fields[1].label == "Type" {
                self.fields[1].value = "postgres".to_string();
            }
            // if self.fields[2].label == "Url" {
            //     self.fields[2].value =
            //         "postgres://localhost:5432/postgres".to_string();
            // }
            if self.fields[3].label == "Environment" {
                self.fields[3].value = "dev".to_string();
            }
            if self.fields[4].label == "Metadata" {
                self.fields[4].value = "{}".to_string();
            }
        }

        for field in &mut self.fields {
            field.clamp_to_options();
        }

        self.dropdown_open = None;
        self.menu_state = None;

        if !self.fields.is_empty() {
            self.fields[0].set_focus(true);
        }
    }

    pub fn open_for_edit(&mut self, connection: &Connection) {
        self.is_open = true;
        self.current_field = 0;
        self.dropdown_open = None;
        self.menu_state = None;
        self.mode = Mode::Edit;
        self.original_name = Some(connection.name.clone());

        // Populate fields with existing data (form_values: Name, Type, URL, Environment, Metadata, Password)
        let connection_data = connection.form_values();
        for (i, field) in self.fields.iter_mut().enumerate() {
            if i < connection_data.len() {
                field.value.clone_from(&connection_data[i]);
            }
            field.set_focus(false);
        }

        self.password_storage = connection
            .password_storage
            .as_ref()
            .map(|s| PasswordStorageType::from_str(s).unwrap_or_default())
            .unwrap_or_default();

        for field in &mut self.fields {
            field.clamp_to_options();
        }
        if !self.fields.is_empty() {
            self.fields[0].set_focus(true);
        }
    }

    pub const fn close(&mut self) {
        self.is_open = false;
    }

    /// Build and open the tui-menu dropdown for the current field if it is a dropdown field.
    fn open_dropdown_if_focused(&mut self) {
        if self.current_field >= self.visible_fields_count() {
            return;
        }
        let idx = self.current_field;
        let Some(field) = self.fields.get(idx) else {
            return;
        };
        let Some(ref opts) = field.options else {
            return;
        };
        if opts.is_empty() {
            return;
        }
        // Order options with current value first so tui-menu highlights it after push().
        let current = field.value.as_str();
        let mut children: Vec<MenuItem<&'static str>> =
            opts.iter().map(|&o| MenuItem::item(o, o)).collect();
        if let Some(pos) = children.iter().position(|m| m.data == Some(current))
        {
            if pos != 0 {
                let item = children.remove(pos);
                children.insert(0, item);
            }
        }
        let mut state = MenuState::new(vec![MenuItem::group("", children)]);
        state.activate();
        let _ = state.push();
        self.menu_state = Some(state);
        self.dropdown_open = Some(idx);
    }

    /// Close dropdown; if apply is true, set field value from selected menu item.
    fn close_dropdown(&mut self, apply: bool) {
        let field_idx = self.dropdown_open.take();
        let mut state = self.menu_state.take();
        if let (Some(idx), Some(ref mut s)) = (field_idx, state.as_mut()) {
            if apply {
                for event in s.drain_events() {
                    let MenuEvent::Selected(value) = event;
                    if let Some(field) = self.fields.get_mut(idx) {
                        field.value = value.to_string();
                    }
                    break;
                }
            }
            s.reset();
        }
    }

    /// Get total number of navigable items (fields + storage selector when visible + buttons)
    fn total_items(&self) -> usize {
        let storage = if self.is_password_storage_row_visible() {
            1
        } else {
            0
        };
        self.visible_fields_count() + storage + 3
    }

    /// Check if `current_field` is on a button
    fn is_on_button(&self) -> Option<usize> {
        let storage = if self.is_password_storage_row_visible() {
            1
        } else {
            0
        };
        let button_start = self.visible_fields_count() + storage;
        if self.current_field >= button_start
            && self.current_field < button_start + 3
        {
            Some(self.current_field - button_start)
        } else {
            None
        }
    }

    pub fn next_field(&mut self) {
        let total = self.total_items();
        if self.current_field < total - 1 {
            // Clear current focus
            if self.current_field < self.visible_fields_count() {
                let logical_index = self.current_field;
                if logical_index < self.fields.len() {
                    self.fields[logical_index].set_focus(false);
                }
            }

            self.current_field += 1;

            // Set focus on new item
            if self.current_field < self.visible_fields_count() {
                let logical_index = self.current_field;
                if logical_index < self.fields.len() {
                    self.fields[logical_index].set_focus(true);
                }
            }
        }
    }

    pub fn prev_field(&mut self) {
        if self.current_field > 0 {
            // Clear current focus
            if self.current_field < self.visible_fields_count() {
                let logical_index = self.current_field;
                if logical_index < self.fields.len() {
                    self.fields[logical_index].set_focus(false);
                }
            }

            self.current_field -= 1;

            // Set focus on new item
            if self.current_field < self.visible_fields_count() {
                let logical_index = self.current_field;
                if logical_index < self.fields.len() {
                    self.fields[logical_index].set_focus(true);
                }
            }
        }
    }

    pub fn add_char(&mut self, c: char) {
        // Only add characters when on a field, not on storage selector or buttons
        if self.current_field < self.visible_fields_count() {
            let logical_index = self.current_field;
            if let Some(field) = self.fields.get_mut(logical_index) {
                field.add_char(c);
            }
        }
    }

    pub fn remove_char(&mut self) {
        // Only remove characters when on a field, not on storage selector or buttons
        if self.current_field < self.visible_fields_count() {
            let logical_index = self.current_field;
            if let Some(field) = self.fields.get_mut(logical_index) {
                field.remove_char();
            }
        }
    }

    #[must_use]
    pub fn get_connection(&self) -> Option<Connection> {
        // Check if all required fields are filled (password is optional when "ask every time" is selected)
        let password_field_index = self.password_field_index();
        let required_fields: Vec<&ModalField> =
            if self.is_password_field_hidden() {
                // Exclude password field from validation when hidden
                self.fields.iter().take(password_field_index).collect()
            } else {
                // Include all fields when password is visible
                self.fields.iter().collect()
            };

        if required_fields.iter().any(|f| f.value.trim().is_empty()) {
            return None;
        }

        // Password is optional when "ask every time" is selected
        let password = if self.is_password_field_hidden() {
            None
        } else {
            Some(self.fields[password_field_index].value.clone())
        };

        let r#type = self.fields[1].value.parse().unwrap_or_default();
        let environment = self.fields[3].value.parse().unwrap_or_default();
        let metadata = self.fields[4].value.trim();
        let metadata = if metadata.is_empty() {
            serde_json::Value::Object(serde_json::Map::new())
        } else {
            serde_json::from_str(metadata)
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()))
        };

        // SQLite does not support passwords; omit password and storage for that type.
        let (password, password_storage) = if self.is_sqlite() {
            (None, None)
        } else {
            (password, Some(self.password_storage.to_string()))
        };

        Some(Connection {
            name: self.fields[0].value.clone(),
            r#type,
            url: self.fields[2].value.clone(),
            environment,
            metadata,
            selected_database: None,
            schema: None,
            table: None,
            password,
            password_storage,
        })
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        // Password field is optional when "ask every time" is selected
        let password_field_index = self.password_field_index();
        let required_fields: Vec<&ModalField> =
            if self.is_password_field_hidden() {
                // Exclude password field from validation when hidden
                self.fields.iter().take(password_field_index).collect()
            } else {
                // Include all fields when password is visible
                self.fields.iter().collect()
            };

        !required_fields.iter().any(|f| f.value.trim().is_empty())
    }

    /// Handle key events for UI navigation only
    /// Returns an enum indicating what action was triggered
    pub fn handle_key_events_ui(&mut self, key: KeyEvent) -> ModalAction {
        // When a tui-menu dropdown is open, forward navigation to the menu
        if self.menu_state.is_some() {
            return self.handle_menu_key(key);
        }

        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                self.close();
                ModalAction::Cancel
            }
            (_, KeyCode::BackTab | KeyCode::Up) => {
                if self.is_on_button().is_some() {
                    let storage = if self.is_password_storage_row_visible() {
                        1
                    } else {
                        0
                    };
                    // Move focus back to last form element (storage row or last field)
                    self.current_field = self
                        .visible_fields_count()
                        .saturating_add(storage)
                        .saturating_sub(1);
                } else {
                    self.prev_field();
                }
                ModalAction::None
            }
            (_, KeyCode::Tab | KeyCode::Down) => {
                self.next_field();
                ModalAction::None
            }
            (_, KeyCode::Enter) => {
                if let Some(button_idx) = self.is_on_button() {
                    match button_idx {
                        0 if self.is_valid() => ModalAction::Save,
                        1 => ModalAction::Test,
                        2 => {
                            self.close();
                            ModalAction::Cancel
                        }
                        _ => ModalAction::None,
                    }
                } else if self.current_field < self.visible_fields_count() {
                    let idx = self.current_field;
                    if self.fields.get(idx).is_some_and(|f| f.is_dropdown()) {
                        self.open_dropdown_if_focused();
                        ModalAction::None
                    } else if self.is_valid() {
                        ModalAction::Save
                    } else {
                        ModalAction::None
                    }
                } else if self.is_valid() {
                    ModalAction::Save
                } else {
                    ModalAction::None
                }
            }
            (_, KeyCode::Char(c)) => {
                if self.is_password_storage_row_visible()
                    && self.current_field == self.visible_fields_count()
                    && c == ' '
                {
                    self.toggle_password_storage();
                    return ModalAction::None;
                }
                if self.current_field < self.visible_fields_count() {
                    self.add_char(c);
                }
                ModalAction::None
            }
            (_, KeyCode::Backspace) => {
                self.remove_char();
                ModalAction::None
            }
            (_, KeyCode::Left) => {
                if let Some(button_idx) = self.is_on_button() {
                    let storage = if self.is_password_storage_row_visible() {
                        1
                    } else {
                        0
                    };
                    let button_start = self.visible_fields_count() + storage;
                    let new_button_idx = (button_idx + 2) % 3;
                    self.current_field = button_start + new_button_idx;
                } else {
                    self.prev_field();
                }
                ModalAction::None
            }
            (_, KeyCode::Right) => {
                if let Some(button_idx) = self.is_on_button() {
                    let storage = if self.is_password_storage_row_visible() {
                        1
                    } else {
                        0
                    };
                    let button_start = self.visible_fields_count() + storage;
                    let new_button_idx = (button_idx + 1) % 3;
                    self.current_field = button_start + new_button_idx;
                } else {
                    self.next_field();
                }
                ModalAction::None
            }
            _ => ModalAction::None,
        }
    }

    /// Handle keys when tui-menu dropdown is open: Escape to close; Up/Down/j/k to navigate; Enter to select.
    fn handle_menu_key(&mut self, key: KeyEvent) -> ModalAction {
        let Some(ref mut state) = self.menu_state else {
            return ModalAction::None;
        };
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                self.close_dropdown(false);
                ModalAction::None
            }
            (_, KeyCode::Up)
            | (_, KeyCode::Char('k'))
            | (_, KeyCode::Char('K')) => {
                state.up();
                ModalAction::None
            }
            (_, KeyCode::Down)
            | (_, KeyCode::Char('j'))
            | (_, KeyCode::Char('J')) => {
                state.down();
                ModalAction::None
            }
            (_, KeyCode::Enter) => {
                state.select();
                for event in state.drain_events() {
                    let MenuEvent::Selected(value) = event;
                    if let Some(field_idx) = self.dropdown_open {
                        if let Some(field) = self.fields.get_mut(field_idx) {
                            field.value = value.to_string();
                        }
                    }
                    break;
                }
                self.close_dropdown(false);
                ModalAction::None
            }
            _ => ModalAction::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalAction {
    None,
    Save,
    Test,
    Cancel,
}

/// Stateful widget for the connection modal (required because modal holds tui-menu state that is not Clone).
pub struct ConnectionModalWidget;

impl StatefulWidget for ConnectionModalWidget {
    type State = Modal;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.render_into(area, buf);
    }
}

impl Modal {
    /// Render the connection modal into the buffer (used by ConnectionModalWidget).
    pub fn render_into(&self, area: Rect, buf: &mut Buffer) {
        if !self.is_open {
            return;
        }

        let modal_width = CONNECTION_MODAL_WIDTH;
        let field_height = self.fields_section_height();
        let modal_height = 1 + field_height + 1 + 1 + 2;
        let x = area.x + (area.width.saturating_sub(modal_width)) / 2;
        let y = area.y + (area.height.saturating_sub(modal_height)) / 2;
        let modal_area =
            Rect::new(x, y, modal_width, modal_height.min(area.height));

        let title = match self.mode {
            Mode::New => "New Connection".to_string(),
            Mode::Edit => "Edit Connection".to_string(),
        };

        let block = Block::default()
            .title(title)
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue))
            .style(Style::default().bg(Color::Black));
        Clear.render(modal_area, buf);
        block.render(modal_area, buf);

        let inner_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(field_height),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .margin(1)
            .split(modal_area);

        self.render_fields(inner_layout[1], buf);
        self.render_test_result(inner_layout[2], buf);
        self.render_buttons(inner_layout[3], buf);
    }

    /// Fixed height for fields section; dropdown is drawn as overlay and does not expand layout.
    fn fields_section_height(&self) -> u16 {
        let rows = self.visible_fields_count() as u16
            + if self.is_password_storage_row_visible() {
                1
            } else {
                0
            };
        (rows + 2).min(9) // rows + padding, cap for modal
    }
}

impl Modal {
    /// Get the index of the password field (last field)
    const fn password_field_index(&self) -> usize {
        self.fields.len() - 1
    }

    /// True when connection type is SQLite (password not supported).
    fn is_sqlite(&self) -> bool {
        self.fields
            .get(1)
            .map(|f| f.value.eq_ignore_ascii_case("sqlite"))
            .unwrap_or(false)
    }

    /// Check if password field should be hidden (Ask every time, or SQLite which has no passwords).
    fn is_password_field_hidden(&self) -> bool {
        self.password_storage == PasswordStorageType::DontSave
            || self.is_sqlite()
    }

    /// Show password storage row only for Postgres (SQLite has no passwords).
    fn is_password_storage_row_visible(&self) -> bool {
        !self.is_sqlite()
    }

    /// Get the number of visible fields (excluding password if hidden)
    fn visible_fields_count(&self) -> usize {
        if self.is_password_field_hidden() {
            self.fields.len() - 1
        } else {
            self.fields.len()
        }
    }

    fn render_fields(&self, area: Rect, buf: &mut Buffer) {
        // Fixed one row per field (+ storage row only for Postgres); dropdown list is overlay.
        let storage_row = if self.is_password_storage_row_visible() {
            1
        } else {
            0
        };
        let num_rows = self.visible_fields_count() + storage_row;
        let field_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints((0..num_rows).map(|_| Constraint::Length(1)))
            .split(area);

        let highlighted_value = self
            .menu_state
            .as_ref()
            .and_then(|s| s.highlight())
            .and_then(|m| m.data);

        let mut overlay: Option<(Rect, &[&'static str], usize)> = None;
        let mut visible_index = 0;

        for (i, field) in self.fields.iter().enumerate() {
            if self.is_password_field_hidden()
                && i == self.password_field_index()
            {
                continue;
            }

            let row_area = field_layout[visible_index];
            let is_dropdown_open =
                self.dropdown_open == Some(i) && field.is_dropdown();

            if is_dropdown_open {
                let opts = field.options.as_deref().unwrap_or(&[]);
                let hi = highlighted_value
                    .and_then(|v| opts.iter().position(|o| *o == v))
                    .unwrap_or(0)
                    .min(opts.len().saturating_sub(1));
                self.render_trigger_row(row_area, buf, field, true);
                if !opts.is_empty() {
                    overlay = Some((row_area, opts, hi));
                }
            } else {
                let label = format!("{}:", field.label);
                let value = if field.value.is_empty() {
                    " ".repeat(18)
                } else if i == self.password_field_index() {
                    "•".repeat(field.value.len())
                } else if field.is_dropdown() {
                    format!("{} ▼", field.value)
                } else {
                    field.value.clone()
                };
                let text = format!("{label:<12} {value}");
                let style = if field.is_focused {
                    Style::default().fg(Color::Yellow).bg(Color::DarkGray)
                } else {
                    Style::default().fg(Color::White)
                };
                Paragraph::new(text)
                    .style(style)
                    .alignment(Alignment::Left)
                    .render(row_area, buf);
            }
            visible_index += 1;
        }

        if self.is_password_storage_row_visible() {
            let checkbox_text = match self.password_storage {
                PasswordStorageType::Keyring => "[ ] Ask every time",
                PasswordStorageType::DontSave => "[x] Ask every time",
            };
            let storage_style =
                if self.current_field == self.visible_fields_count() {
                    Style::default().fg(Color::Yellow).bg(Color::DarkGray)
                } else {
                    Style::default().fg(Color::Cyan)
                };
            Paragraph::new(checkbox_text)
                .style(storage_style)
                .alignment(Alignment::Left)
                .render(field_layout[self.visible_fields_count()], buf);
        }

        // Draw dropdown list as overlay so it hovers over content below without shifting layout.
        if let Some((trigger_rect, options, highlighted_index)) = overlay {
            self.render_dropdown_overlay(
                buf,
                trigger_rect,
                options,
                highlighted_index,
            );
        }
    }

    fn render_trigger_row(
        &self,
        area: Rect,
        buf: &mut Buffer,
        field: &ModalField,
        is_open: bool,
    ) {
        let trigger_style = if field.is_focused {
            Style::default().fg(Color::Yellow).bg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };
        let arrow = if is_open { " ▲" } else { " ▼" };
        let value_display = if field.value.is_empty() {
            " ".to_string()
        } else {
            format!("{}{}", field.value, arrow)
        };
        let label = format!("{}:", field.label);
        let trigger_text = format!("{label:<12} {value_display}");
        Paragraph::new(trigger_text)
            .style(trigger_style)
            .alignment(Alignment::Left)
            .render(area, buf);
    }

    /// Render dropdown list as a floating overlay below the trigger row (does not affect layout).
    fn render_dropdown_overlay(
        &self,
        buf: &mut Buffer,
        trigger_rect: Rect,
        options: &[&'static str],
        highlighted_index: usize,
    ) {
        if options.is_empty() {
            return;
        }
        // Height: top border + one row per option + bottom border
        let list_height = 2 + options.len() as u16;
        let list_width = trigger_rect.width.max(
            options
                .iter()
                .map(|o| o.len() as u16 + 2)
                .max()
                .unwrap_or(10),
        );

        let overlay_rect = Rect {
            x: trigger_rect.x,
            y: trigger_rect.y + 1,
            width: list_width,
            height: list_height,
        };

        Clear.render(overlay_rect, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));
        let inner = block.inner(overlay_rect);
        block.render(overlay_rect, buf);

        for (i, opt) in options.iter().enumerate() {
            let row_area = Rect {
                x: inner.x,
                y: inner.y + i as u16,
                width: inner.width,
                height: 1,
            };
            let is_highlighted = i == highlighted_index;
            let style = if is_highlighted {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::White).bg(Color::Black)
            };
            let line =
                Line::from(vec![Span::styled(format!(" {}", opt), style)]);
            Paragraph::new(line).render(row_area, buf);
        }
    }

    fn render_buttons(&self, area: Rect, buf: &mut Buffer) {
        // Determine which button is selected based on current_field
        // Only select a button if we're actually on a button, otherwise use out-of-bounds index
        let selected_button = self.is_on_button().unwrap_or(999); // 999 ensures no button is selected
        let buttons = Buttons {
            buttons: vec!["OK", "Test", "Cancel"],
            selected: selected_button,
        };
        buttons.render(area, buf);
    }

    fn render_test_result(&self, area: Rect, buf: &mut Buffer) {
        let (text, style) = match &self.test_result {
            TestResult::NotTested => ("", Style::default()),
            TestResult::Testing => {
                ("Testing connection...", Style::default().fg(Color::Yellow))
            }
            TestResult::Success => {
                ("✓ Connection successful", Style::default().fg(Color::Green))
            }
            TestResult::Failed(msg) => {
                (msg.as_str(), Style::default().fg(Color::Red))
            }
        };

        Paragraph::new(text)
            .style(style)
            .alignment(Alignment::Center)
            .render(area, buf);
    }
}

impl ConfirmationModal {
    #[must_use]
    pub const fn new(message: String, connection: Connection) -> Self {
        Self {
            is_open: true,
            selected_button: 0,
            message,
            connection: Some(connection),
        }
    }

    pub const fn close(&mut self) {
        self.is_open = false;
    }

    pub const fn next_button(&mut self) {
        self.selected_button = (self.selected_button + 1) % 2;
    }

    pub const fn prev_button(&mut self) {
        self.selected_button = (self.selected_button + 1) % 2;
    }

    #[must_use]
    pub const fn confirm(&self) -> bool {
        self.selected_button == 0
    }

    pub const fn handle_key_events(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Enter) => {
                self.close();
            }
            (_, KeyCode::Left) => {
                self.prev_button();
            }
            (_, KeyCode::Right) => {
                self.next_button();
            }
            _ => {}
        }
    }
}

impl Widget for ConfirmationModal {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.is_open {
            return;
        }

        // Center a fixed-size modal
        let modal_width = CONFIRMATION_MODAL_WIDTH;
        let modal_height = CONFIRMATION_MODAL_HEIGHT;
        let x = area.x + (area.width.saturating_sub(modal_width)) / 2;
        let y = area.y + (area.height.saturating_sub(modal_height)) / 2;
        let modal_area = Rect::new(x, y, modal_width, modal_height);

        let block = Block::default()
            .title("Confirm Delete")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red))
            .style(Style::default().bg(Color::Black));
        Clear.render(modal_area, buf);
        block.render(modal_area, buf);

        // Layout inside the modal: Message, Buttons
        let inner_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Message
                Constraint::Length(1), // Buttons
            ])
            .margin(1)
            .split(modal_area);

        // Render message
        Paragraph::new(self.message)
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center)
            .render(inner_layout[0], buf);

        // Render buttons
        let buttons = Buttons {
            buttons: vec!["Yes", "No"],
            selected: self.selected_button,
        };
        buttons.render(inner_layout[1], buf);
    }
}

impl CellValueModal {
    #[must_use]
    pub const fn new(column_name: String, cell_value: String) -> Self {
        Self {
            is_open: true,
            column_name,
            cell_value,
        }
    }

    pub const fn close(&mut self) {
        self.is_open = false;
    }

    pub const fn handle_key_events(&mut self, key: KeyEvent) {
        if let (_, KeyCode::Esc | KeyCode::Enter) = (key.modifiers, key.code) {
            self.close();
        }
    }
}

impl Widget for CellValueModal {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.is_open {
            return;
        }

        // Calculate modal size based on content
        let max_width = 80u16;
        let value_width = self.cell_value.len().min((max_width - 4) as usize);
        let modal_width =
            u16::try_from((value_width + 4).max(40).min(max_width as usize))
                .unwrap_or(max_width);

        // Calculate height: title + column name + value (with wrapping) + buttons
        // Estimate lines needed: ceil(cell_value.len() / (modal_width - 4))
        let content_width = (modal_width.saturating_sub(4)).max(1) as usize;
        let value_lines = if self.cell_value.is_empty() {
            1u16
        } else {
            u16::try_from(self.cell_value.len().div_ceil(content_width))
                .unwrap_or(1u16)
        };
        let modal_height = (3u16.saturating_add(value_lines).saturating_add(1))
            .min(area.height.saturating_sub(4))
            .max(8);

        let x = area.x + (area.width.saturating_sub(modal_width)) / 2;
        let y = area.y + (area.height.saturating_sub(modal_height)) / 2;
        let modal_area = Rect::new(x, y, modal_width, modal_height);

        let block = Block::default()
            .title(self.column_name)
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));
        Clear.render(modal_area, buf);
        block.render(modal_area, buf);

        // Layout inside the modal: Column name, Value, Button
        let inner_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Column name
                Constraint::Min(3),    // Value (with wrapping)
                Constraint::Length(1), // Button
            ])
            .margin(1)
            .split(modal_area);

        // Render cell value with word wrapping
        Paragraph::new(self.cell_value)
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Left)
            .wrap(ratatui::widgets::Wrap { trim: false })
            .render(inner_layout[1], buf);

        // Render button
        let buttons = Buttons {
            buttons: vec!["OK"],
            selected: 0,
        };
        buttons.render(inner_layout[2], buf);
    }
}

impl PasswordModal {
    #[must_use]
    pub const fn new(connection: Connection, prompt: String) -> Self {
        Self {
            is_open: true,
            password: String::new(),
            connection: Some(connection),
            prompt,
            cursor_position: 0,
            selected_button: 0,
        }
    }

    pub const fn close(&mut self) {
        self.is_open = false;
    }

    pub fn add_char(&mut self, c: char) {
        self.password.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }

    pub fn remove_char(&mut self) {
        if self.cursor_position > 0 {
            self.password.remove(self.cursor_position - 1);
            self.cursor_position -= 1;
        }
    }

    pub const fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    pub const fn move_cursor_right(&mut self) {
        if self.cursor_position < self.password.len() {
            self.cursor_position += 1;
        }
    }

    /// Clear the password field and reset cursor position
    pub fn clear_password(&mut self) {
        self.password.clear();
        self.cursor_position = 0;
    }

    pub fn handle_key_events(&mut self, key: KeyEvent) -> ModalAction {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                self.close();
                ModalAction::Cancel
            }
            (_, KeyCode::Tab | KeyCode::Down) => {
                if self.selected_button == 0 {
                    self.selected_button = 1;
                }
                ModalAction::None
            }
            (_, KeyCode::BackTab | KeyCode::Up) => {
                if self.selected_button == 1 {
                    self.selected_button = 0;
                }
                ModalAction::None
            }
            (_, KeyCode::Left) => {
                if self.selected_button == 1 {
                    self.selected_button = 0;
                } else {
                    self.move_cursor_left();
                }
                ModalAction::None
            }
            (_, KeyCode::Right) => {
                if self.selected_button == 0 {
                    self.selected_button = 1;
                } else {
                    self.move_cursor_right();
                }
                ModalAction::None
            }
            (_, KeyCode::Enter) => match self.selected_button {
                0 if !self.password.is_empty() => {
                    self.close();
                    ModalAction::Save
                }
                1 => {
                    self.close();
                    ModalAction::Cancel
                }
                _ => ModalAction::None,
            },
            (_, KeyCode::Char(c)) if !c.is_control() => {
                if self.selected_button == 0 {
                    self.add_char(c);
                }
                ModalAction::None
            }
            (_, KeyCode::Backspace) => {
                if self.selected_button == 0 {
                    self.remove_char();
                }
                ModalAction::None
            }
            _ => ModalAction::None,
        }
    }
}

impl Widget for PasswordModal {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.is_open {
            return;
        }

        // Center a fixed-size modal
        let modal_width = PASSWORD_MODAL_WIDTH;
        let modal_height = PASSWORD_MODAL_HEIGHT;
        let x = area.x + (area.width.saturating_sub(modal_width)) / 2;
        let y = area.y + (area.height.saturating_sub(modal_height)) / 2;
        let modal_area = Rect::new(x, y, modal_width, modal_height);

        let block = Block::default()
            .title("Enter Password")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .style(Style::default().bg(Color::Black));
        Clear.render(modal_area, buf);
        block.render(modal_area, buf);

        // Layout inside the modal: Prompt, Password input, Buttons
        let inner_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Prompt
                Constraint::Length(1), // Password input
                Constraint::Length(1), // Buttons
            ])
            .margin(1)
            .split(modal_area);

        // Render prompt
        Paragraph::new(self.prompt)
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Left)
            .render(inner_layout[0], buf);

        // Render password input (masked) with cursor
        let masked_password: String =
            self.password.chars().map(|_| '•').collect();
        let cursor_pos =
            self.cursor_position.min(masked_password.chars().count());
        let mut spans = Vec::new();

        // Get byte position for cursor
        let cursor_byte_pos = masked_password
            .char_indices()
            .nth(cursor_pos)
            .map_or(masked_password.len(), |(i, _)| i);

        // Add masked password before cursor
        if cursor_pos > 0 {
            spans.push(Span::raw(&masked_password[..cursor_byte_pos]));
        }

        // Add cursor
        spans.push(Span::styled(
            "█",
            Style::default().fg(Color::White).bg(Color::Black),
        ));

        // Add masked password after cursor
        if cursor_pos < masked_password.chars().count() {
            spans.push(Span::raw(&masked_password[cursor_byte_pos..]));
        }

        let line = Line::from(spans);
        Paragraph::new(line)
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Left)
            .render(inner_layout[1], buf);

        // Render buttons
        let buttons = Buttons {
            buttons: vec!["OK", "Cancel"],
            selected: self.selected_button,
        };
        buttons.render(inner_layout[2], buf);
    }
}

/// Manager for handling multiple modals in the application
#[derive(Default, Debug)]
pub struct ModalManager {
    connection_modal: Option<Modal>,
    confirmation_modal: Option<ConfirmationModal>,
    cell_value_modal: Option<CellValueModal>,
    password_modal: Option<PasswordModal>,
    active_modal_type: Option<ModalType>,
}

impl ModalManager {
    /// Create a new modal manager
    #[must_use]
    pub const fn new() -> Self {
        Self {
            connection_modal: None,
            confirmation_modal: None,
            cell_value_modal: None,
            password_modal: None,
            active_modal_type: None,
        }
    }

    /// Check if any modal is currently open
    #[must_use]
    pub fn is_any_modal_open(&self) -> bool {
        self.connection_modal.as_ref().is_some_and(|m| m.is_open)
            || self.confirmation_modal.as_ref().is_some_and(|m| m.is_open)
            || self.cell_value_modal.as_ref().is_some_and(|m| m.is_open)
            || self.password_modal.as_ref().is_some_and(|m| m.is_open)
    }

    /// Open a new connection modal
    pub fn open_new_connection_modal(&mut self) {
        let mut modal = Modal::new(Connection::default(), Mode::New);
        modal.open();
        self.connection_modal = Some(modal);
        self.active_modal_type = Some(ModalType::Connection);
    }

    /// Open an edit connection modal
    pub fn open_edit_connection_modal(
        &mut self,
        connection: &Connection,
        password: String,
    ) {
        let mut connection_with_password = connection.clone();
        connection_with_password.password = Some(password);

        let mut modal =
            Modal::new(connection_with_password.clone(), Mode::Edit);
        modal.open_for_edit(&connection_with_password);
        self.connection_modal = Some(modal);
        self.active_modal_type = Some(ModalType::Connection);
    }

    /// Open a confirmation modal
    pub fn open_confirmation_modal(
        &mut self,
        message: String,
        connection: Connection,
    ) {
        let modal = ConfirmationModal::new(message, connection);
        self.confirmation_modal = Some(modal);
        self.active_modal_type = Some(ModalType::Confirmation);
    }

    /// Open a cell value display modal
    pub fn open_cell_value_modal(
        &mut self,
        column_name: String,
        cell_value: String,
    ) {
        let modal = CellValueModal::new(column_name, cell_value);
        self.cell_value_modal = Some(modal);
        self.active_modal_type = Some(ModalType::CellValue);
    }

    /// Open a password input modal
    pub fn open_password_modal(
        &mut self,
        connection: Connection,
        prompt: String,
    ) {
        let modal = PasswordModal::new(connection, prompt);
        self.password_modal = Some(modal);
        self.active_modal_type = Some(ModalType::Password);
    }

    /// Close the currently active modal
    pub const fn close_active_modal(&mut self) {
        match self.active_modal_type {
            Some(ModalType::Connection) => {
                if let Some(modal) = &mut self.connection_modal {
                    modal.close();
                }
            }
            Some(ModalType::Confirmation) => {
                if let Some(modal) = &mut self.confirmation_modal {
                    modal.close();
                }
            }
            Some(ModalType::CellValue) => {
                if let Some(modal) = &mut self.cell_value_modal {
                    modal.close();
                }
            }
            Some(ModalType::Password) => {
                if let Some(modal) = &mut self.password_modal {
                    modal.close();
                }
            }
            None => {}
        }
        self.active_modal_type = None;
    }

    /// Handle key events for the currently active modal (UI only)
    /// Returns the action that was triggered
    pub fn handle_key_events_ui(&mut self, key: KeyEvent) -> ModalAction {
        match self.active_modal_type {
            Some(ModalType::Connection) => {
                if let Some(modal) = &mut self.connection_modal {
                    let action = modal.handle_key_events_ui(key);
                    // If modal was closed, clear the active type
                    if !modal.is_open {
                        self.active_modal_type = None;
                    }
                    action
                } else {
                    ModalAction::None
                }
            }
            Some(ModalType::Confirmation) => {
                if let Some(modal) = &mut self.confirmation_modal {
                    modal.handle_key_events(key);
                    // If modal was closed, clear the active type
                    if !modal.is_open {
                        self.active_modal_type = None;
                    }
                    if modal.confirm() {
                        ModalAction::Save
                    } else {
                        ModalAction::Cancel
                    }
                } else {
                    ModalAction::None
                }
            }
            Some(ModalType::CellValue) => {
                if let Some(modal) = &mut self.cell_value_modal {
                    modal.handle_key_events(key);
                    // If modal was closed, clear the active type
                    if !modal.is_open {
                        self.active_modal_type = None;
                    }
                    ModalAction::Cancel
                } else {
                    ModalAction::None
                }
            }
            Some(ModalType::Password) => {
                if let Some(modal) = &mut self.password_modal {
                    let action = modal.handle_key_events(key);
                    // If modal was closed, clear the active type
                    if !modal.is_open {
                        self.active_modal_type = None;
                    }
                    action
                } else {
                    ModalAction::None
                }
            }
            None => ModalAction::None,
        }
    }

    /// Get a reference to the connection modal
    #[must_use]
    pub const fn get_connection_modal(&self) -> Option<&Modal> {
        self.connection_modal.as_ref()
    }

    /// Get a mutable reference to the connection modal
    pub const fn get_connection_modal_mut(&mut self) -> Option<&mut Modal> {
        self.connection_modal.as_mut()
    }

    /// Get a reference to the confirmation modal
    #[must_use]
    pub const fn get_confirmation_modal(&self) -> Option<&ConfirmationModal> {
        self.confirmation_modal.as_ref()
    }

    /// Check if the connection modal was just closed and needs a refresh
    #[must_use]
    pub fn was_connection_modal_closed(&self) -> bool {
        self.connection_modal.as_ref().is_some_and(|m| !m.is_open)
    }

    /// Check if the confirmation modal was just closed and confirmed
    #[must_use]
    pub fn was_confirmation_modal_confirmed(&self) -> Option<Connection> {
        if let Some(modal) = &self.confirmation_modal
            && !modal.is_open
            && modal.confirm()
        {
            return modal.connection.clone();
        }

        None
    }

    /// Clear any closed modals from memory
    pub fn cleanup_closed_modals(&mut self) {
        if let Some(modal) = &self.connection_modal
            && !modal.is_open
        {
            self.connection_modal = None;
        }

        if let Some(modal) = &self.confirmation_modal
            && !modal.is_open
        {
            self.confirmation_modal = None;
        }

        if let Some(modal) = &self.cell_value_modal
            && !modal.is_open
        {
            self.cell_value_modal = None;
        }

        if let Some(modal) = &self.password_modal
            && !modal.is_open
        {
            self.password_modal = None;
        }
    }

    /// Get a reference to the password modal
    #[must_use]
    pub const fn get_password_modal(&self) -> Option<&PasswordModal> {
        self.password_modal.as_ref()
    }

    /// Get a mutable reference to the password modal
    pub const fn get_password_modal_mut(
        &mut self,
    ) -> Option<&mut PasswordModal> {
        self.password_modal.as_mut()
    }

    /// Get a reference to the cell value modal
    #[must_use]
    pub const fn get_cell_value_modal(&self) -> Option<&CellValueModal> {
        self.cell_value_modal.as_ref()
    }
}
