mod app;
mod app_state;
mod connection_manager;
mod database_explorer;
mod database_explorer_state;
mod errors;
mod event_handlers;
mod filtered_data;
mod filtering;
mod rendering;
mod services;

use app::App;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::default().initialise()?.run(terminal).await;
    ratatui::restore();
    result
}
