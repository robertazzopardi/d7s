mod app;
mod app_state;
mod docker;
mod event_handlers;
mod keymap;
mod rendering;
mod ui;

use std::io::stdout;

use app::App;
use crossterm::{
    event::{DisableBracketedPaste, EnableBracketedPaste},
    execute,
};

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let mut warnings = k9tui::theme::load_skin(app::PKG_NAME);
    let terminal = ratatui::init();
    execute!(stdout(), EnableBracketedPaste)?;
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(stdout(), DisableBracketedPaste);
        prev_hook(info);
    }));

    let mut app = App::new();
    let (keymap, key_warnings) = keymap::load();
    app.keymap = keymap;
    warnings.extend(key_warnings);
    if !warnings.is_empty() {
        app.status_line
            .set_message(format!("Config: {}", warnings.join("; ")));
    }
    let result = app.run(terminal).await;

    let _ = execute!(stdout(), DisableBracketedPaste);
    ratatui::restore();
    for w in &warnings {
        eprintln!("c8s: {w}");
    }
    result
}
