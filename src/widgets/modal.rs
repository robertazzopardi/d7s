use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use d7s_db::{Database, TableData, connection::Connection};
use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Direction, Layout, Rect, Widget},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::widgets::buttons::Buttons;

#[derive(Clone, Debug, Default)]
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
    pub const fn new(label: &'static str) -> Self {
        Self {
            label,
            value: String::new(),
            is_focused: false,
        }
    }

    pub fn with_value(mut self, value: String) -> Self {
        self.value = value;
        self
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

#[derive(Default, Debug, Clone)]
pub struct Modal<T: TableData> {
    pub fields: Vec<ModalField>,
    pub current_field: usize,
    pub is_open: bool,
    pub selected_button: usize,
    pub data: T,
    pub mode: Mode,
    pub test_result: TestResult,
    pub original_name: Option<String>,
}

#[derive(Default, Debug, Clone)]
pub struct ConfirmationModal {
    pub is_open: bool,
    pub selected_button: usize,
    pub message: String,
    pub connection: Option<Connection>,
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
        };

        // Set focus on first field
        if !modal.fields.is_empty() {
            modal.fields[0].set_focus(true);
        }

        modal
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
                field.value = connection_data[i].clone();
            }
            field.set_focus(false);
        }

        // Set focus on first field
        if !self.fields.is_empty() {
            self.fields[0].set_focus(true);
        }
    }

    pub const fn close(&mut self) {
        self.is_open = false;
    }

    pub fn next_field(&mut self) {
        if self.current_field < self.fields.len() - 1 {
            self.fields[self.current_field].set_focus(false);
            self.current_field += 1;
            self.fields[self.current_field].set_focus(true);
        }
    }

    pub fn prev_field(&mut self) {
        if self.current_field > 0 {
            self.fields[self.current_field].set_focus(false);
            self.current_field -= 1;
            self.fields[self.current_field].set_focus(true);
        }
    }

    pub fn add_char(&mut self, c: char) {
        if let Some(field) = self.fields.get_mut(self.current_field) {
            field.add_char(c);
        }
    }

    pub fn remove_char(&mut self) {
        if let Some(field) = self.fields.get_mut(self.current_field) {
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
        })
    }

    pub fn is_valid(&self) -> bool {
        !self.fields.iter().any(|f| f.value.trim().is_empty())
    }
}

impl<T: TableData> Widget for Modal<T> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.is_open {
            return;
        }

        // Center a fixed-size modal
        let modal_width = 40;
        let modal_height = 14;
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

        // Layout inside the modal: Title, Subtitle, Fields, Buttons
        let inner_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Title
                Constraint::Min(8),    // Fields (6 fields + some padding)
                Constraint::Length(1), // Test result
                Constraint::Length(1), // Buttons
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
        let field_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(self.fields.iter().map(|_| Constraint::Length(1)))
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
    }

    fn render_buttons(&self, area: Rect, buf: &mut Buffer) {
        let buttons = Buttons {
            buttons: vec!["OK", "Test", "Cancel"],
            selected: self.selected_button,
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

    pub async fn handle_key_events(&mut self, key: KeyEvent) -> Result<()> {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                self.close();
            }
            (KeyModifiers::SHIFT, KeyCode::Tab) | (_, KeyCode::Up) => {
                self.prev_field();
            }
            (_, KeyCode::Tab) | (_, KeyCode::Down) => {
                self.next_field();
            }
            (_, KeyCode::Enter) => {
                if self.selected_button == 0 && self.is_valid() {
                    if let Some(connection) = self.get_connection() {
                        match self.mode {
                            Mode::New => {
                                let entry = keyring::Entry::new(
                                    "d7s",
                                    &connection.user,
                                )?;
                                if let Some(password) = &connection.password {
                                    entry.set_password(password)?;
                                }

                                // TODO dont save password and user to database
                                d7s_db::sqlite::save_connection(&connection)
                                    .unwrap();
                            }
                            Mode::Edit => {
                                let entry = keyring::Entry::new(
                                    "d7s",
                                    &connection.user,
                                )?;
                                if let Some(password) = &connection.password {
                                    entry.set_password(password)?;
                                }

                                // Use the stored original name for updating
                                if let Some(original_name) = &self.original_name
                                {
                                    d7s_db::sqlite::update_connection(
                                        original_name,
                                        &connection,
                                    )
                                    .unwrap();
                                }
                            }
                        }

                        self.close();
                    }
                } else if self.selected_button == 1 {
                    // Test button - handle test in a blocking way
                    if let Some(connection) = self.get_connection() {
                        let postgres = connection.to_postgres();
                        self.test_result = TestResult::Testing;

                        // Use block_on to run the async test in a blocking context
                        let result = postgres.test().await;
                        self.test_result = if result {
                            TestResult::Success
                        } else {
                            TestResult::Failed("Connection failed".to_string())
                        };
                    } else {
                        self.test_result = TestResult::Failed(
                            "Please fill in all fields".to_string(),
                        );
                    }
                } else if self.selected_button == 2 {
                    self.close();
                }
            }
            (_, KeyCode::Char(c)) => {
                self.add_char(c);
            }
            (_, KeyCode::Backspace) => {
                self.remove_char();
            }

            (_, KeyCode::Left) => {
                self.selected_button = (self.selected_button + 2) % 3;
            }
            (_, KeyCode::Right) => {
                self.selected_button = (self.selected_button + 1) % 3;
            }
            _ => {}
        }

        Ok(())
    }
}

impl ConfirmationModal {
    pub fn new(message: String, connection: Connection) -> Self {
        Self {
            is_open: true,
            selected_button: 0,
            message,
            connection: Some(connection),
        }
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn next_button(&mut self) {
        self.selected_button = (self.selected_button + 1) % 2;
    }

    pub fn prev_button(&mut self) {
        self.selected_button = (self.selected_button + 1) % 2;
    }

    pub fn confirm(&self) -> bool {
        self.selected_button == 0
    }

    pub async fn handle_key_events(&mut self, key: KeyEvent) -> Result<()> {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                self.close();
            }
            (_, KeyCode::Left) => {
                self.prev_button();
            }
            (_, KeyCode::Right) => {
                self.next_button();
            }
            (_, KeyCode::Enter) => {
                self.close();
            }
            _ => {}
        }

        Ok(())
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
