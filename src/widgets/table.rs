use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::Text,
    widgets::{
        Cell, HighlightSpacing, Row, StatefulWidget, Table, TableState, Widget,
    },
};
use unicode_width::UnicodeWidthStr;

pub trait TableData {
    fn ref_array(&self) -> Vec<&String>;
    fn num_columns(&self) -> usize;

    fn col(&self, column: usize) -> &str {
        self.ref_array()[column]
    }
}

#[derive(Debug, Clone)]
pub struct Data {
    pub name: String,
    pub address: String,
    pub email: String,
}

impl TableData for Data {
    fn ref_array(&self) -> Vec<&String> {
        vec![&self.name, &self.address, &self.email]
    }

    fn num_columns(&self) -> usize {
        self.ref_array().len()
    }
}

/// A ratatui widget for displaying tabular data with selection and styling
pub struct DataTable {
    items: Vec<Data>,
    longest_item_lens: Vec<u16>, // order is (name, address, email)
}

impl DataTable {
    pub fn new(items: Vec<Data>) -> Self {
        let longest_item_lens = constraint_len_calculator(&items);
        Self {
            items,
            longest_item_lens,
        }
    }
}

impl StatefulWidget for DataTable {
    type State = TableState;

    fn render(
        self,
        area: Rect,
        buf: &mut ratatui::buffer::Buffer,
        state: &mut Self::State,
    ) {
        let header_style = Style::default();
        let selected_row_style =
            Style::default().add_modifier(Modifier::REVERSED);
        let selected_col_style = Style::default();
        let selected_cell_style =
            Style::default().add_modifier(Modifier::REVERSED);

        let header = ["Name", "Address", "Email"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(header_style)
            .height(1);

        let rows = self.items.iter().enumerate().map(|(i, data)| {
            let item = data.ref_array();
            item.into_iter()
                .map(|content| Cell::from(content.to_string()))
                .collect::<Row>()
                .style(Style::new())
                .height(1)
        });

        let bar: &'static str = " █ ";
        let constraints = self
            .longest_item_lens
            .into_iter()
            .map(|len| Constraint::Fill(len));
        let t = Table::new(rows, constraints)
            .header(header)
            .row_highlight_style(selected_row_style)
            .column_highlight_style(selected_col_style)
            .cell_highlight_style(selected_cell_style)
            .highlight_symbol(Text::from(vec![
                "".into(),
                bar.into(),
                bar.into(),
                "".into(),
            ]))
            .highlight_spacing(HighlightSpacing::Always);

        Widget::render(t, area, buf);
    }
}

/// State management for the table widget
pub struct TableWidgetState {
    pub state: TableState,
}

impl TableWidgetState {
    pub fn new(items_len: usize) -> Self {
        Self {
            state: TableState::default().with_selected(0),
        }
    }

    pub fn next_row(&mut self, items_len: usize) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= items_len - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous_row(&mut self, items_len: usize) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    items_len - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn next_column(&mut self) {
        self.state.select_next_column();
    }

    pub fn previous_column(&mut self) {
        self.state.select_previous_column();
    }
}

/// A complete table view that combines the table widget with state management
pub struct TableView {
    state: TableWidgetState,
    items: Vec<Data>,
}

impl TableView {
    pub fn new() -> Self {
        let data_vec = generate_fake_names();
        Self {
            state: TableWidgetState::new(data_vec.len()),
            items: data_vec,
        }
    }

    pub fn with_items(items: Vec<Data>) -> Self {
        Self {
            state: TableWidgetState::new(items.len()),
            items,
        }
    }

    pub fn next_row(&mut self) {
        self.state.next_row(self.items.len());
    }

    pub fn previous_row(&mut self) {
        self.state.previous_row(self.items.len());
    }

    pub fn next_column(&mut self) {
        self.state.next_column();
    }

    pub fn previous_column(&mut self) {
        self.state.previous_column();
    }

    pub fn state(&mut self) -> &mut TableWidgetState {
        &mut self.state
    }

    pub fn items(&self) -> &[Data] {
        &self.items
    }

    pub fn items_mut(&mut self) -> &mut Vec<Data> {
        &mut self.items
    }

    // pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
    //     loop {
    //         terminal.draw(|frame| self.draw(frame))?;

    //         if let Event::Key(key) = event::read()? {
    //             if key.kind == KeyEventKind::Press {
    //                 let shift_pressed =
    //                     key.modifiers.contains(KeyModifiers::SHIFT);
    //                 match key.code {
    //                     KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
    //                     KeyCode::Char('j') | KeyCode::Down => self.next_row(),
    //                     KeyCode::Char('k') | KeyCode::Up => self.previous_row(),
    //                     KeyCode::Char('l') | KeyCode::Right => {
    //                         self.next_column()
    //                     }
    //                     KeyCode::Char('h') | KeyCode::Left => {
    //                         self.previous_column()
    //                     }
    //                     _ => {}
    //                 }
    //             }
    //         }
    //     }
    // }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let table_widget = DataTable::new(self.items.clone());
        frame.render_stateful_widget(table_widget, area, &mut self.state.state);
    }
}

fn generate_fake_names() -> Vec<Data> {
    // Manually create 20 test values
    let names = [
        "Alice Smith",
        "Bob Johnson",
        "Charlie Brown",
        "Diana Prince",
        "Ethan Hunt",
        "Fiona Gallagher",
        "George Bailey",
        "Hannah Abbott",
        "Ian Malcolm",
        "Julia Child",
        "Kevin Flynn",
        "Laura Palmer",
        "Michael Scott",
        "Nina Simone",
        "Oscar Wilde",
        "Pam Beesly",
        "Quentin Blake",
        "Rachel Green",
        "Samwise Gamgee",
        "Tina Fey",
    ];

    let addresses = [
        "123 Main St Springfield, IL 62704",
        "456 Oak Ave Metropolis, NY 10001",
        "789 Pine Rd Gotham, NJ 07001",
        "321 Maple Dr Smallville, KS 66002",
        "654 Elm St Sunnydale, CA 90210",
        "987 Cedar Ln Stars Hollow, CT 06001",
        "246 Birch Blvd Hill Valley, CA 95420",
        "135 Spruce Ct Twin Peaks, WA 98065",
        "864 Willow Way Jurassic, MT 59001",
        "753 Aspen Pl Paris, TX 75460",
        "159 Redwood Ter Tron City, FL 33101",
        "951 Sycamore St Mystic Falls, VA 24112",
        "852 Poplar Ave Scranton, PA 18503",
        "357 Chestnut Rd Tryon, NC 28782",
        "258 Magnolia Cir Dublin, OH 43017",
        "654 Palm St Scranton, PA 18504",
        "147 Cypress Ln London, UK SW1A 1AA",
        "369 Dogwood Dr New York, NY 10012",
        "741 Maple St Shire, ME 04001",
        "963 Oakwood Ave Upper Darby, PA 19082",
    ];

    let emails = [
        "alice.smith@example.com",
        "bob.johnson@example.com",
        "charlie.brown@example.com",
        "diana.prince@example.com",
        "ethan.hunt@example.com",
        "fiona.gallagher@example.com",
        "george.bailey@example.com",
        "hannah.abbott@example.com",
        "ian.malcolm@example.com",
        "julia.child@example.com",
        "kevin.flynn@example.com",
        "laura.palmer@example.com",
        "michael.scott@example.com",
        "nina.simone@example.com",
        "oscar.wilde@example.com",
        "pam.beesly@example.com",
        "quentin.blake@example.com",
        "rachel.green@example.com",
        "samwise.gamgee@example.com",
        "tina.fey@example.com",
    ];

    let mut data: Vec<Data> = (0..20)
        .map(|i| Data {
            name: names[i].to_string(),
            address: addresses[i].to_string(),
            email: emails[i].to_string(),
        })
        .collect();

    data.sort_by(|a, b| a.name.cmp(&b.name));
    data
}

fn constraint_len_calculator<T: TableData>(items: &[T]) -> Vec<u16> {
    if items.is_empty() {
        return Vec::new();
    }

    let num_columns = items[0].num_columns();

    (0..num_columns)
        .map(|col_idx| {
            items
                .iter()
                .flat_map(|data| data.col(col_idx).lines())
                .map(UnicodeWidthStr::width)
                .max()
                .unwrap_or(0) as u16
        })
        .collect()
}
