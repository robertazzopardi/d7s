use crossterm::event::KeyCode;
use k9tui::widgets::hotkey::Hotkey;

pub const LIST_HOTKEYS: [Hotkey; 5] = [
    Hotkey::new('s', "start/stop"),
    Hotkey::new('r', "restart"),
    Hotkey::new('l', "logs"),
    Hotkey::new('e', "exec shell"),
    Hotkey::new('d', "remove"),
];

pub const GLOBAL_HOTKEYS: [Hotkey; 1] = [Hotkey::new('q', "quit")];

pub fn log_hotkeys() -> Vec<Hotkey> {
    vec![
        Hotkey::new('j', "down"),
        Hotkey::new('k', "up"),
        Hotkey::code(KeyCode::PageUp, "page up"),
        Hotkey::code(KeyCode::PageDown, "page down"),
        Hotkey::new('g', "top"),
        Hotkey::new('G', "follow"),
        Hotkey::code(KeyCode::Esc, "back"),
        Hotkey::new('q', "back"),
    ]
}
