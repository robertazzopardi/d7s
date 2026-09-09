use k9tui::widgets::hotkey::Hotkey;

pub const CONNECTION_HOTKEYS: [Hotkey; 5] = [
    Hotkey::new('n', "New Connection"),
    Hotkey::new('e', "Edit Connection"),
    Hotkey::new('d', "Delete Connection"),
    Hotkey::new('o', "Open Connection"),
    Hotkey::new('/', "Search"),
];

pub const DATABASE_HOTKEYS: [Hotkey; 5] = [
    Hotkey::new('e', "SQL Editor"),
    Hotkey::new('t', "Table structure"),
    Hotkey::new('E', "Run SQL"),
    Hotkey::new('/', "Search"),
    Hotkey::new('y', "Copy value"),
];

/// Shown in addition to [`DATABASE_HOTKEYS`] while viewing table row data.
pub const TABLE_DATA_VIEW_HOTKEYS: [Hotkey; 5] = [
    Hotkey::new('r', "Refresh"),
    Hotkey::new('a', "New row"),
    Hotkey::new('c', "Duplicate row"),
    Hotkey::new('s', "Commit row"),
    Hotkey::new('d', "Delete row"),
];
