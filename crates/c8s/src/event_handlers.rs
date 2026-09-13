use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use k9tui::{
    theme,
    widgets::{
        modal::{ConfirmDialog, DialogAction},
        navigation::TableNavigationHandler,
    },
};
use ratatui::{DefaultTerminal, style::Style};

use crate::{app::App, app_state::AppState};

impl App {
    pub async fn on_key_event(
        &mut self,
        key: KeyEvent,
        terminal: &mut DefaultTerminal,
    ) -> Result<()> {
        match &self.state {
            AppState::ConnectError(_) => self.on_key_connect_error(key).await,
            AppState::Connecting => Ok(()),
            AppState::List => self.on_key_list(key, terminal).await,
            AppState::Logs { .. } => {
                self.on_key_logs(key);
                Ok(())
            }
        }
    }

    async fn on_key_connect_error(&mut self, key: KeyEvent) -> Result<()> {
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('r') | KeyCode::Enter) => {
                self.state = AppState::Connecting;
                self.connect().await;
            }
            (_, KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c' | 'C')) => {
                self.quit();
            }
            _ => {}
        }
        Ok(())
    }

    fn on_key_logs(&mut self, key: KeyEvent) {
        let max_start = self
            .log_lines
            .len()
            .saturating_sub(self.log_viewport_height);
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q') | KeyCode::Esc) => self.close_logs(),
            (KeyModifiers::CONTROL, KeyCode::Char('c' | 'C')) => self.quit(),
            (_, KeyCode::Char('k') | KeyCode::Up) => {
                self.log_follow = false;
                self.log_scroll = self.log_scroll.saturating_sub(1);
            }
            (_, KeyCode::Char('j') | KeyCode::Down) => {
                self.log_scroll = (self.log_scroll + 1).min(max_start);
                self.log_follow = self.log_scroll >= max_start;
            }
            (KeyModifiers::CONTROL, KeyCode::Char('u'))
            | (_, KeyCode::PageUp) => {
                self.log_follow = false;
                self.log_scroll = self.log_scroll.saturating_sub(10);
            }
            (KeyModifiers::CONTROL, KeyCode::Char('d'))
            | (_, KeyCode::PageDown) => {
                self.log_scroll = (self.log_scroll + 10).min(max_start);
                self.log_follow = self.log_scroll >= max_start;
            }
            (_, KeyCode::Char('g') | KeyCode::Home) => {
                self.log_follow = false;
                self.log_scroll = 0;
            }
            (_, KeyCode::Char('G') | KeyCode::End) => {
                self.log_follow = true;
            }
            _ => {}
        }
    }

    async fn on_key_list(
        &mut self,
        key: KeyEvent,
        terminal: &mut DefaultTerminal,
    ) -> Result<()> {
        if let Some(dialog) = self.confirm_dialog.as_mut() {
            match dialog.handle_key_events(key) {
                DialogAction::Submit => {
                    self.confirm_dialog = None;
                    if let Some(id) = self.pending_remove.take() {
                        self.remove_container(&id).await;
                    }
                }
                DialogAction::Cancel => {
                    self.confirm_dialog = None;
                    self.pending_remove = None;
                }
                DialogAction::None => {}
            }
            return Ok(());
        }

        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c' | 'C')) => {
                self.quit();
            }
            (_, KeyCode::Char('j') | KeyCode::Down) => {
                TableNavigationHandler::navigate_table(
                    &self.containers.model,
                    &mut self.containers.view,
                    KeyCode::Down,
                );
            }
            (_, KeyCode::Char('k') | KeyCode::Up) => {
                TableNavigationHandler::navigate_table(
                    &self.containers.model,
                    &mut self.containers.view,
                    KeyCode::Up,
                );
            }
            (_, KeyCode::Char('g')) => {
                TableNavigationHandler::navigate_table(
                    &self.containers.model,
                    &mut self.containers.view,
                    KeyCode::Char('g'),
                );
            }
            (_, KeyCode::Char('G')) => {
                TableNavigationHandler::navigate_table(
                    &self.containers.model,
                    &mut self.containers.view,
                    KeyCode::Char('G'),
                );
            }
            (_, KeyCode::Char('s' | 'S')) => {
                if let Some(row) = self.selected_container().cloned() {
                    if row.status.eq_ignore_ascii_case("running") {
                        self.stop_container(&row.id).await;
                    } else {
                        self.start_container(&row.id).await;
                    }
                }
            }
            (_, KeyCode::Char('r')) => {
                if let Some(row) = self.selected_container().cloned() {
                    self.restart_container(&row.id).await;
                }
            }
            (_, KeyCode::Char('l')) => {
                if let Some(row) = self.selected_container().cloned() {
                    self.open_logs(&row.id, &row.name);
                }
            }
            (_, KeyCode::Char('e')) => {
                if let Some(row) = self.selected_container().cloned() {
                    self.exec_shell(terminal, &row.id)?;
                }
            }
            (_, KeyCode::Char('d') | KeyCode::Delete) => {
                if let Some(row) = self.selected_container().cloned() {
                    self.pending_remove = Some(row.id);
                    self.confirm_dialog = Some(ConfirmDialog::new(
                        " Remove container? ",
                        format!(
                            "Remove container '{}'?\nThis action cannot be undone.",
                            row.name
                        ),
                        modal_border(),
                        1,
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn start_container(&mut self, id: &str) {
        let Some(docker) = self.docker.clone() else {
            return;
        };
        match docker.start(id).await {
            Ok(()) => self.set_status("Starting container"),
            Err(e) => self.set_status(format!("Start failed: {e}")),
        }
    }

    async fn stop_container(&mut self, id: &str) {
        let Some(docker) = self.docker.clone() else {
            return;
        };
        match docker.stop(id).await {
            Ok(()) => self.set_status("Stopping container"),
            Err(e) => self.set_status(format!("Stop failed: {e}")),
        }
    }

    async fn restart_container(&mut self, id: &str) {
        let Some(docker) = self.docker.clone() else {
            return;
        };
        match docker.restart(id).await {
            Ok(()) => self.set_status("Restarting container"),
            Err(e) => self.set_status(format!("Restart failed: {e}")),
        }
    }

    async fn remove_container(&mut self, id: &str) {
        let Some(docker) = self.docker.clone() else {
            return;
        };
        match docker.remove(id).await {
            Ok(()) => self.set_status("Container removed"),
            Err(e) => self.set_status(format!("Remove failed: {e}")),
        }
    }
}

fn modal_border() -> Style {
    theme::border()
}
