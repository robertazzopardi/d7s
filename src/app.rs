use std::fmt::Display;

use color_eyre::Result;
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use ratatui::{
    DefaultTerminal, Frame,
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

const APP_NAME: &str = r"
_________________        
\______ \______  \______ 
 |    |  \  /    /  ___/ 
 |    `   \/    /\___ \  
/_______  /____//____  > 
        \/           \/  
";

struct Hotkey<'a> {
    keycode: KeyCode,
    description: &'a str,
}

impl<'a> Display for Hotkey<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.keycode)
    }
}

const CONNECTION_HOTKEYS: [Hotkey; 5] = [
    Hotkey {
        keycode: KeyCode::Char('n'),
        description: "New Connection",
    },
    Hotkey {
        keycode: KeyCode::Char('e'),
        description: "Edit Connection",
    },
    Hotkey {
        keycode: KeyCode::Char('d'),
        description: "Delete Connection",
    },
    Hotkey {
        keycode: KeyCode::Char('o'),
        description: "Open Connection",
    },
    Hotkey {
        keycode: KeyCode::Char('t'),
        description: "Test Connection",
    },
];

struct HotkeyView<'a> {
    hotkeys: &'a [Hotkey<'a>],
}

impl<'a> Widget for HotkeyView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut y = area.y;
        let mut x = area.x;
        let max_y = area.y + area.height;
        let column_width = 30; // Width for each column

        for hotkey in self.hotkeys {
            // Check if we need to start a new column
            if y >= max_y {
                x += column_width;
                y = area.y;
            }

            // Create a rectangle for this hotkey row
            let hotkey_area = Rect::new(x, y, column_width, 1);

            // Split the area horizontally for key and description
            let row = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(
                        (hotkey.keycode.to_string().len()
                            + hotkey.description.len())
                            as u16
                            + 3,
                    ), // Space for the key
                    Constraint::Fill(1), // Rest for description
                ])
                .split(hotkey_area);

            Paragraph::new(format!("<{}> {}", hotkey, hotkey.description))
                .render(row[0], buf);
            // Paragraph::new(format!("{}", hotkey.description))
            //     .render(row[1], buf);

            y += 1;
        }
    }
}

#[derive(Debug, Default)]
struct Connection {
    name: String,
    host: String,
    port: u16,
    user: String,
    database: String,
    schema: String,
    table: String,
}

impl Display for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            " Name: {}\n Host: {}\n Port: {}\n User: {}\n Database: {}\n Schema: {}\n Table: {}",
            self.name,
            self.host,
            if self.port == 0 {
                "".to_string()
            } else {
                self.port.to_string()
            },
            self.user,
            self.database,
            self.schema,
            self.table
        )
    }
}

struct TopBarView {
    current_connection: Connection,
}

impl Widget for TopBarView {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let col_constraints = [
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ];
        let row_constraints = [Constraint::Fill(1)];

        let horizontal = Layout::horizontal(col_constraints).spacing(1);
        let vertical = Layout::vertical(row_constraints).spacing(1);

        let rows = vertical.split(area);
        let cells = rows
            .iter()
            .flat_map(|&row| horizontal.split(row).to_vec())
            .collect::<Vec<_>>();

        Paragraph::new(format!("{}", self.current_connection))
            .render(cells[0], buf);
        HotkeyView {
            hotkeys: &CONNECTION_HOTKEYS,
        }
        .render(cells[1], buf);
        Paragraph::new(APP_NAME.trim_start())
            .alignment(Alignment::Right)
            .render(cells[2], buf);
    }
}

/// The main application which holds the state and logic of the application.
#[derive(Debug, Default)]
pub struct App {
    /// Is the application running?
    running: bool,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Run the application's main loop.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_crossterm_events()?;
        }
        Ok(())
    }

    /// Renders the user interface.
    ///
    /// This is where you add new widgets. See the following resources for more information:
    ///
    /// - <https://docs.rs/ratatui/latest/ratatui/widgets/index.html>
    /// - <https://github.com/ratatui/ratatui/tree/main/ratatui-widgets/examples>
    fn render(&mut self, frame: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(13),
                Constraint::Percentage(87),
            ])
            .split(frame.area());

        let current_connection = Connection::default();
        frame.render_widget(TopBarView { current_connection }, layout[0]);
        frame.render_widget(
            Block::new()
                .borders(Borders::ALL)
                .title(" Connections ")
                .title_alignment(Alignment::Center),
            layout[1],
        );
    }

    /// Reads the crossterm events and updates the state of [`App`].
    ///
    /// If your application needs to perform work in between handling events, you can use the
    /// [`event::poll`] function to check if there are any events available with a timeout.
    fn handle_crossterm_events(&mut self) -> Result<()> {
        match event::read()? {
            // it's important to check KeyEventKind::Press to avoid handling key release events
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                self.on_key_event(key)
            }
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    fn on_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (
                KeyModifiers::CONTROL,
                KeyCode::Char('c') | KeyCode::Char('C'),
            ) => self.quit(),
            // Add other key handlers here.
            _ => {}
        }
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }
}
