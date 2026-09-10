use crossterm::event::KeyCode;
use k9tui::widgets::hotkey::Hotkey;

pub const LIST_HOTKEYS: [Hotkey; 6] = [
    Hotkey::new('s', "start/stop"),
    Hotkey::new('r', "restart"),
    Hotkey::new('l', "logs"),
    Hotkey::new('e', "exec shell"),
    Hotkey::new('d', "remove"),
    Hotkey::code(KeyCode::Enter, "details"),
];

pub const GLOBAL_HOTKEYS: [Hotkey; 1] = [Hotkey::new('q', "quit")];

pub fn log_hotkeys() -> Vec<Hotkey> {
    vec![
        Hotkey::code(KeyCode::Esc, "back"),
        Hotkey::new('q', "back"),
    ]
}
