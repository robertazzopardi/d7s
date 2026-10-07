use k9tui::widgets::hotkey::Hotkey;

pub const CONNECTION_HOTKEYS: [Hotkey; 6] = [
    Hotkey::new('n', "New Connection").action("new_connection"),
    Hotkey::new('e', "Edit Connection").action("edit_connection"),
    Hotkey::new('D', "Delete Connection").action("delete_connection"),
    Hotkey::new('o', "Open Connection").action("open_connection"),
    Hotkey::new('d', "Describe").action("describe"),
    Hotkey::new('/', "Search"),
];

pub const DATABASE_HOTKEYS: [Hotkey; 8] = [
    Hotkey::new('e', "SQL Editor").action("sql_editor"),
    Hotkey::new('t', "Table structure").action("table_structure"),
    Hotkey::new('d', "Describe").action("describe"),
    Hotkey::new('E', "Run SQL").action("run_sql"),
    Hotkey::new('/', "Search"),
    Hotkey::new('y', "Copy value").action("copy_value"),
    Hotkey::new('A', "Activity (Postgres)"),
    Hotkey::new('L', "Query log"),
];

/// Shown in addition to [`DATABASE_HOTKEYS`] while viewing table row data.
pub const TABLE_DATA_VIEW_HOTKEYS: [Hotkey; 5] = [
    Hotkey::new('r', "Refresh").action("refresh"),
    Hotkey::new('a', "New row").action("new_row"),
    Hotkey::new('c', "Duplicate row").action("duplicate_row"),
    Hotkey::new('s', "Commit row").action("commit_row"),
    Hotkey::new('D', "Delete row").action("delete_row"),
];
