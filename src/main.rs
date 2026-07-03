mod app;
mod app_state;
mod auth;
mod connection_manager;
mod database_explorer;
mod database_explorer_state;
mod db;
mod event_handlers;
mod filtered_data;
mod filtering;
mod rendering;
mod services;
mod sql;
mod table_data_actions;
mod ui;
mod virtual_table;

use std::io::stdout;

use app::App;
use crossterm::{
    event::{DisableBracketedPaste, EnableBracketedPaste},
    execute,
};

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    execute!(stdout(), EnableBracketedPaste)?;
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(stdout(), DisableBracketedPaste);
        prev_hook(info);
    }));
    let result = App::default().init()?.run(terminal).await;
    let _ = execute!(stdout(), DisableBracketedPaste);
    ratatui::restore();
    result
}
