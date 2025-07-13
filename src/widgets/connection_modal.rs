use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Direction, Layout, Rect, Widget},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::widgets::{buttons::Buttons, connection::Connection};

#[derive(Debug, Clone)]
pub struct ConnectionField {
    pub label: &'static str,
    pub value: String,
    pub is_focused: bool,
}

impl ConnectionField {
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

#[derive(Debug, Clone)]
pub struct ConnectionModal {
    pub fields: Vec<ConnectionField>,
    pub current_field: usize,
    pub is_open: bool,
    pub selected_button: usize,
}

impl ConnectionModal {
    pub fn new() -> Self {
        let fields = vec![
            ConnectionField::new("Name"),
            ConnectionField::new("Host"),
            ConnectionField::new("Port"),
            ConnectionField::new("User"),
            ConnectionField::new("Database"),
            ConnectionField::new("Schema"),
            ConnectionField::new("Table"),
        ];

        let mut modal = Self {
            fields,
            current_field: 0,
            is_open: false,
            selected_button: 0,
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
            schema: self.fields[5].value.clone(),
            table: self.fields[6].value.clone(),
        })
    }

    pub fn is_valid(&self) -> bool {
        !self.fields.iter().any(|f| f.value.trim().is_empty())
    }
}

impl Widget for ConnectionModal {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.is_open {
            return;
        }

        // Center a fixed-size modal (e.g., 60x18)
        let modal_width = 40;
        let modal_height = 12;
        let x = area.x + (area.width.saturating_sub(modal_width)) / 2;
        let y = area.y + (area.height.saturating_sub(modal_height)) / 2;
        let modal_area = Rect::new(x, y, modal_width, modal_height);

        let block = Block::default()
            .title("New Connection")
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
                Constraint::Min(7),    // Fields
                Constraint::Length(1), // Buttons
            ])
            .margin(1)
            .split(modal_area);

        // Title: Postgres Connection
        let title = Paragraph::new("Postgres Connection")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::LightRed));
        title.render(inner_layout[0], buf);

        // Render form fields inside the modal
        self.render_fields(inner_layout[1], buf);

        // Render buttons at the bottom
        self.render_buttons(inner_layout[2], buf);
    }
}

impl ConnectionModal {
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
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default().bg(Color::Black).fg(Color::White)
            };
            Paragraph::new(text)
                .style(style)
                .alignment(Alignment::Left)
                .render(field_layout[i], buf);
        }
    }

    fn render_buttons(&self, area: Rect, buf: &mut Buffer) {
        let buttons = Buttons {
            buttons: vec!["OK", "Cancel"],
            selected: self.selected_button,
        };
        buttons.render(area, buf);
    }
}
