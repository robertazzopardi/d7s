use std::{fmt::Display, str::FromStr};

use crossterm::event::{KeyCode, KeyEvent};
use d7s_db::{TableData, connection::Connection};
use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Direction, Layout, Rect, Widget},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::widgets::buttons::Buttons;

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
}

impl ModalField {
    #[must_use]
    pub const fn new(label: &'static str) -> Self {
        Self {
            label,
            value: String::new(),
            is_focused: false,
        }
    }

    pub const fn set_focus(&mut self, focused: bool) {
        self.is_focused = focused;
    }

    pub fn add_char(&mut self, c: char) {
        self.value.push(c);
    }

    pub fn remove_char(&mut self) {
        self.value.pop();
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

#[derive(Debug, Clone)]
pub struct Modal<T: TableData> {
    pub fields: Vec<ModalField>,
    pub current_field: usize,
    pub is_open: bool,
    pub selected_button: usize,
    #[allow(dead_code)]
    pub data: T,
    pub mode: Mode,
    pub test_result: TestResult,
    pub original_name: Option<String>,
    pub password_storage: PasswordStorageType,
}

impl<T> Default for Modal<T>
where
    T: TableData + Default,
{
    fn default() -> Self {
        Self {
            fields: vec![],
            current_field: 0,
            is_open: false,
            selected_button: 0,
            data: T::default(),
            mode: Mode::default(),
            test_result: TestResult::default(),
            original_name: None,
            password_storage: PasswordStorageType::default(),
        }
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
    pub save_password: bool,
}

impl<T: TableData> Modal<T> {
    pub fn new(data: T, mode: Mode) -> Self {
        let fields = T::cols().iter().map(|c| ModalField::new(c)).collect();

        let mut modal = Self {
            fields,
            current_field: 0,
            is_open: false,
            selected_button: 0,
            data,
            mode,
            test_result: TestResult::NotTested,
            original_name: None,
            password_storage: PasswordStorageType::default(),
        };

        // Set focus on first field
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
        // Set focus on first field
        if !self.fields.is_empty() {
            self.fields[0].set_focus(true);
        }
    }

    pub fn open_for_edit(&mut self, connection: &Connection) {
        self.is_open = true;
        self.current_field = 0;
        self.mode = Mode::Edit;
        self.original_name = Some(connection.name.clone());

        // Populate fields with existing data
        let connection_data = connection.ref_array();
        for (i, field) in self.fields.iter_mut().enumerate() {
            if i < connection_data.len() {
                // For password field (last field), use actual password instead of masked version
                if i == connection_data.len() - 1 {
                    field.value =
                        connection.password.clone().unwrap_or_default();
                } else {
                    field.value.clone_from(&connection_data[i]);
                }
            }
            field.set_focus(false);
        }

        // Load password storage preference from connection
        self.password_storage = connection
            .password_storage
            .as_ref()
            .map(|s| PasswordStorageType::from_str(s).unwrap_or_default())
            .unwrap_or_default();

        // Set focus on first field
        if !self.fields.is_empty() {
            self.fields[0].set_focus(true);
        }
    }

    pub const fn close(&mut self) {
        self.is_open = false;
    }

    /// Get total number of navigable items (fields + storage selector + buttons)
    const fn total_items(&self) -> usize {
        self.fields.len() + 1 + 3 // fields + storage selector + 3 buttons
    }

    /// Check if `current_field` is on a button
    const fn is_on_button(&self) -> Option<usize> {
        let button_start = self.fields.len() + 1;
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
            if self.current_field < self.fields.len() {
                self.fields[self.current_field].set_focus(false);
            }

            self.current_field += 1;

            // Set focus on new item
            if self.current_field < self.fields.len() {
                self.fields[self.current_field].set_focus(true);
            }
        }
    }

    pub fn prev_field(&mut self) {
        if self.current_field > 0 {
            // Clear current focus
            if self.current_field < self.fields.len() {
                self.fields[self.current_field].set_focus(false);
            }

            self.current_field -= 1;

            // Set focus on new item
            if self.current_field < self.fields.len() {
                self.fields[self.current_field].set_focus(true);
            }
        }
    }

    pub fn add_char(&mut self, c: char) {
        // Only add characters when on a field, not on storage selector or buttons
        if self.current_field < self.fields.len()
            && let Some(field) = self.fields.get_mut(self.current_field)
        {
            field.add_char(c);
        }
    }

    pub fn remove_char(&mut self) {
        // Only remove characters when on a field, not on storage selector or buttons
        if self.current_field < self.fields.len()
            && let Some(field) = self.fields.get_mut(self.current_field)
        {
            field.remove_char();
        }
    }

    pub fn get_connection(&self) -> Option<Connection> {
        if self.fields.iter().any(|f| f.value.is_empty()) {
            return None;
        }

        Some(Connection {
            name: self.fields[0].value.clone(),
            host: self.fields[1].value.clone(),
            port: self.fields[2].value.clone(),
            user: self.fields[3].value.clone(),
            database: self.fields[4].value.clone(),
            password: Some(self.fields[5].value.clone()),
            schema: None,
            table: None,
            password_storage: Some(self.password_storage.to_string()),
        })
    }

    pub fn is_valid(&self) -> bool {
        !self.fields.iter().any(|f| f.value.trim().is_empty())
    }

    /// Handle key events for UI navigation only
    /// Returns an enum indicating what action was triggered
    pub fn handle_key_events_ui(&mut self, key: KeyEvent) -> ModalAction {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                self.close();
                ModalAction::Cancel
            }
            (_, KeyCode::BackTab | KeyCode::Up) => {
                // If on buttons, go to storage selector above
                if self.is_on_button().is_some() {
                    self.current_field = self.fields.len(); // Storage selector
                } else {
                    self.prev_field();
                }
                ModalAction::None
            }
            (_, KeyCode::Tab | KeyCode::Down) => {
                // If on buttons, don't navigate (nothing below buttons)
                if self.is_on_button().is_none() {
                    self.next_field();
                }
                ModalAction::None
            }
            (_, KeyCode::Enter) => {
                // Check if we're on a button
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
                } else {
                    // If on a field or storage selector, treat Enter as Save if valid
                    if self.is_valid() {
                        ModalAction::Save
                    } else {
                        ModalAction::None
                    }
                }
            }
            (_, KeyCode::Char(c)) => {
                // If focused on storage selector, Space toggles it
                if self.current_field == self.fields.len() && c == ' ' {
                    self.toggle_password_storage();
                    return ModalAction::None;
                }
                // Only add characters when on a field (not on buttons)
                if self.current_field < self.fields.len() {
                    self.add_char(c);
                }
                ModalAction::None
            }
            (_, KeyCode::Backspace) => {
                self.remove_char();
                ModalAction::None
            }
            (_, KeyCode::Left) => {
                // If on buttons, navigate left between buttons
                if let Some(button_idx) = self.is_on_button() {
                    let new_button_idx = (button_idx + 2) % 3;
                    self.current_field = self.fields.len() + 1 + new_button_idx;
                } else {
                    // Otherwise, move to previous item
                    self.prev_field();
                }
                ModalAction::None
            }
            (_, KeyCode::Right) => {
                // If on buttons, navigate right between buttons
                if let Some(button_idx) = self.is_on_button() {
                    let new_button_idx = (button_idx + 1) % 3;
                    self.current_field = self.fields.len() + 1 + new_button_idx;
                } else {
                    // Otherwise, move to next item
                    self.next_field();
                }
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

impl<T: TableData> Widget for Modal<T> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.is_open {
            return;
        }

        // Center a fixed-size modal
        let modal_width = 40;
        let modal_height = 15; // Extra height for storage selector
        let x = area.x + (area.width.saturating_sub(modal_width)) / 2;
        let y = area.y + (area.height.saturating_sub(modal_height)) / 2;
        let modal_area = Rect::new(x, y, modal_width, modal_height);

        let title = match self.mode {
            Mode::New => format!("New {}", T::title()),
            Mode::Edit => format!("Edit {}", T::title()),
        };

        let block = Block::default()
            .title(title)
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue))
            .style(Style::default().bg(Color::Black));
        Clear.render(modal_area, buf);
        block.render(modal_area, buf);

        // Layout inside the modal: Title, Subtitle, Fields, Storage selector, Test result, Buttons
        let field_height = 9; // 6 fields + storage selector + padding

        let inner_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),         // Title
                Constraint::Min(field_height), // Fields
                Constraint::Length(1),         // Test result
                Constraint::Length(1),         // Buttons
            ])
            .margin(1)
            .split(modal_area);

        // Render form fields inside the modal
        self.render_fields(inner_layout[1], buf);

        // Render test result
        self.render_test_result(inner_layout[2], buf);

        // Render buttons at the bottom
        self.render_buttons(inner_layout[3], buf);
    }
}

impl<T: TableData> Modal<T> {
    fn render_fields(&self, area: Rect, buf: &mut Buffer) {
        // Each field is a row: label left, value right after colon
        let num_rows = self.fields.len() + 1; // +1 for storage selector

        let field_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints((0..num_rows).map(|_| Constraint::Length(1)))
            .split(area);

        for (i, field) in self.fields.iter().enumerate() {
            let label = format!("{}:", field.label);
            let value = if field.value.is_empty() {
                " ".repeat(18)
            } else {
                // Check if this is a password field (last field)
                if i == self.fields.len() - 1 {
                    "•".repeat(field.value.len())
                } else {
                    field.value.clone()
                }
            };
            let text = format!("{label:<12} {value}");

            // Apply different styling based on focus state
            let style = if field.is_focused {
                Style::default().fg(Color::Yellow).bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };

            Paragraph::new(text)
                .style(style)
                .alignment(Alignment::Left)
                .render(field_layout[i], buf);
        }

        // Render password storage selector as a checkbox toggle
        let checkbox_text = match self.password_storage {
            PasswordStorageType::Keyring => "[ ] Ask every time",
            PasswordStorageType::DontSave => "[x] Ask every time",
        };
        let storage_style = if self.current_field == self.fields.len() {
            // Focused on storage selector
            Style::default().fg(Color::Yellow).bg(Color::DarkGray)
        } else {
            Style::default().fg(Color::Cyan)
        };
        Paragraph::new(checkbox_text)
            .style(storage_style)
            .alignment(Alignment::Left)
            .render(field_layout[self.fields.len()], buf);
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
        let modal_width = 50;
        let modal_height = 8;
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
            save_password: false, // Default to not saving password
        }
    }

    pub const fn toggle_save_password(&mut self) {
        self.save_password = !self.save_password;
    }

    pub const fn close(&mut self) {
        self.is_open = false;
    }

    pub fn add_char(&mut self, c: char) {
        self.password.push(c);
    }

    pub fn remove_char(&mut self) {
        self.password.pop();
    }

    pub fn handle_key_events(&mut self, key: KeyEvent) -> ModalAction {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                self.close();
                ModalAction::Cancel
            }
            (_, KeyCode::Enter) => {
                if self.password.is_empty() {
                    ModalAction::None
                } else {
                    self.close();
                    ModalAction::Save
                }
            }
            (_, KeyCode::Char(' ')) => {
                // Space toggles save password checkbox (don't add space to password)
                self.toggle_save_password();
                ModalAction::None
            }
            (_, KeyCode::Char(c)) if !c.is_control() => {
                self.add_char(c);
                ModalAction::None
            }
            (_, KeyCode::Backspace) => {
                self.remove_char();
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
        let modal_width = 50;
        let modal_height = 8;
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

        // Layout inside the modal: Prompt, Password input, Save checkbox, Buttons
        let inner_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Prompt
                Constraint::Length(1), // Password input
                Constraint::Length(2), // Save password checkbox
                Constraint::Length(1), // Buttons
            ])
            .margin(1)
            .split(modal_area);

        // Render prompt
        Paragraph::new(self.prompt)
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Left)
            .render(inner_layout[0], buf);

        // Render password input (masked)
        let masked_password = "•".repeat(self.password.len());
        Paragraph::new(masked_password)
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Left)
            .render(inner_layout[1], buf);

        // Render save password checkbox
        let checkbox_text = if self.save_password {
            "[x] Save password in keyring"
        } else {
            "[ ] Save password in keyring"
        };
        Paragraph::new(checkbox_text)
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Left)
            .render(inner_layout[2], buf);

        // Render buttons
        let buttons = Buttons {
            buttons: vec!["OK", "Cancel"],
            selected: usize::from(self.password.is_empty()),
        };
        buttons.render(inner_layout[3], buf);
    }
}

/// Manager for handling multiple modals in the application
#[derive(Default, Debug)]
pub struct ModalManager {
    connection_modal: Option<Modal<Connection>>,
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
    pub const fn get_connection_modal(&self) -> Option<&Modal<Connection>> {
        self.connection_modal.as_ref()
    }

    /// Get a mutable reference to the connection modal
    pub const fn get_connection_modal_mut(
        &mut self,
    ) -> Option<&mut Modal<Connection>> {
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
