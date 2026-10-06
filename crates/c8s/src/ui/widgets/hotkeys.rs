use crossterm::event::KeyCode;
use k9tui::widgets::hotkey::Hotkey;

pub const LIST_HOTKEYS: [Hotkey; 6] = [
    Hotkey::new('s', "start/stop"),
    Hotkey::new('r', "restart"),
    Hotkey::new('l', "logs"),
    Hotkey::new('e', "exec shell"),
    Hotkey::new('d', "describe"),
    Hotkey::new('D', "remove"),
];

/// Hotkeys shown for the image/volume/network list views, which only
/// support navigation and removal (no start/stop/restart/logs/exec).
pub const RESOURCE_HOTKEYS: [Hotkey; 2] =
    [Hotkey::new('d', "describe"), Hotkey::new('D', "remove")];

pub const VIEW_SWITCH_HOTKEYS: [Hotkey; 4] = [
    Hotkey::new('1', "containers"),
    Hotkey::new('2', "images"),
    Hotkey::new('3', "volumes"),
    Hotkey::new('4', "networks"),
];

pub const GLOBAL_HOTKEYS: [Hotkey; 2] =
    [Hotkey::new('/', "search"), Hotkey::new('q', "quit")];

pub fn log_hotkeys() -> Vec<Hotkey> {
    vec![
        Hotkey::new('j', "down"),
        Hotkey::new('k', "up"),
        Hotkey::code(KeyCode::PageUp, "page up"),
        Hotkey::code(KeyCode::PageDown, "page down"),
        Hotkey::new('g', "top"),
        Hotkey::new('G', "follow"),
        Hotkey::new('/', "search"),
        Hotkey::code(KeyCode::Esc, "back"),
        Hotkey::new('q', "back"),
    ]
}

pub fn describe_hotkeys() -> Vec<Hotkey> {
    vec![
        Hotkey::new('j', "down"),
        Hotkey::new('k', "up"),
        Hotkey::code(KeyCode::PageUp, "page up"),
        Hotkey::code(KeyCode::PageDown, "page down"),
        Hotkey::new('g', "top"),
        Hotkey::new('G', "bottom"),
        Hotkey::code(KeyCode::Esc, "back"),
        Hotkey::new('q', "back"),
    ]
}
