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
    pub fn new(label: &'static str) -> Self {
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

    pub fn set_focus(&mut self, focused: bool) {
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

    pub fn close(&mut self) {
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
            password: self.fields[5].value.clone(),
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

        let block = Block::default()
            .title(T::title())
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
            .constraints(
                self.fields
                    .iter()
                    .map(|_| Constraint::Length(1))
                    .collect::<Vec<_>>(),
            )
            .split(area);

        for (i, field) in self.fields.iter().enumerate() {
            let label = format!("{}:", field.label);
            let value = if field.value.is_empty() {
                " ".repeat(18)
            } else {
                field.value.clone()
            };
            let text = format!("{:<12} {}", label, value);
            let style = if field.is_focused {
                Style::default().fg(Color::White)
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
            // Try multiple ways to detect Shift+Tab
            (KeyModifiers::SHIFT, KeyCode::Tab) => {
                self.prev_field();
            }
            // Some terminals might send this as a different key code
            (KeyModifiers::SHIFT, KeyCode::Char('\t')) => {
                self.prev_field();
            }
            (_, KeyCode::Tab) => {
                self.next_field();
            }
            (_, KeyCode::Enter) => {
                if self.selected_button == 0 && self.is_valid() {
                    if let Some(connection) = self.get_connection() {
                        println!("New connection created: {:?}", connection);
                        let entry =
                            keyring::Entry::new("d7s", &connection.user)?;
                        entry.set_password(&connection.password)?;

                        // TODO dont save password and user to database
                        d7s_db::sqlite::save_connection(&connection).unwrap();

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
            (_, KeyCode::Up) => {
                self.prev_field();
            }
            (_, KeyCode::Down) => {
                self.next_field();
            }
            (_, KeyCode::Left) => {
                self.selected_button = (self.selected_button + 2) % 3;
            }
            (_, KeyCode::Right) => {
                self.selected_button = (self.selected_button + 1) % 3;
            }
            // Alternative navigation keys
            (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
                self.prev_field();
            }
            (KeyModifiers::CONTROL, KeyCode::Char('n')) => {
                self.next_field();
            }
            _ => {}
        }

        Ok(())
    }
}
