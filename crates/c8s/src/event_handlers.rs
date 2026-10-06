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

use crate::{
    app::App,
    app_state::{AppState, ResourceKind},
};

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
            AppState::Describe { .. } => {
                self.on_key_describe(key);
                Ok(())
            }
        }
    }

    fn on_key_describe(&mut self, key: KeyEvent) {
        let AppState::Describe { text, .. } = &self.state else {
            return;
        };
        let total_lines = text.lines().count();
        let max_start =
            total_lines.saturating_sub(self.describe_viewport_height);
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q') | KeyCode::Esc) => self.close_describe(),
            (KeyModifiers::CONTROL, KeyCode::Char('c' | 'C')) => self.quit(),
            (_, KeyCode::Char('k') | KeyCode::Up) => {
                self.describe_scroll = self.describe_scroll.saturating_sub(1);
            }
            (_, KeyCode::Char('j') | KeyCode::Down) => {
                self.describe_scroll =
                    (self.describe_scroll + 1).min(max_start);
            }
            (_, KeyCode::PageUp) => {
                self.describe_scroll = self.describe_scroll.saturating_sub(10);
            }
            (_, KeyCode::PageDown) => {
                self.describe_scroll =
                    (self.describe_scroll + 10).min(max_start);
            }
            (_, KeyCode::Char('g') | KeyCode::Home) => self.describe_scroll = 0,
            (_, KeyCode::Char('G') | KeyCode::End) => {
                self.describe_scroll = max_start;
            }
            _ => {}
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

    #[allow(clippy::wildcard_enum_match_arm)]
    fn on_key_logs(&mut self, key: KeyEvent) {
        if self.log_search_open {
            match key.code {
                KeyCode::Esc => {
                    self.log_filter.clear();
                    self.log_search_open = false;
                }
                KeyCode::Enter => self.log_search_open = false,
                KeyCode::Backspace => {
                    self.log_filter.pop();
                }
                KeyCode::Char(c) => self.log_filter.push(c),
                _ => {}
            }
            self.log_scroll = 0;
            self.log_follow = false;
            return;
        }

        let max_start = self
            .log_lines
            .len()
            .saturating_sub(self.log_viewport_height);
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q') | KeyCode::Esc)
                if self.log_filter.is_empty() =>
            {
                self.close_logs();
            }
            (_, KeyCode::Char('q') | KeyCode::Esc) => self.log_filter.clear(),
            (_, KeyCode::Char('/')) => self.log_search_open = true,
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

    #[allow(clippy::wildcard_enum_match_arm)]
    #[allow(clippy::too_many_lines)] // flat key-dispatch match
    async fn on_key_list(
        &mut self,
        key: KeyEvent,
        terminal: &mut DefaultTerminal,
    ) -> Result<()> {
        if let Some(dialog) = self.confirm_dialog.as_mut() {
            match dialog.handle_key_events(key) {
                DialogAction::Submit => {
                    self.confirm_dialog = None;
                    if let Some((kind, id)) = self.pending_remove.take() {
                        self.remove_resource(kind, &id).await;
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

        if self.list_search_open {
            match key.code {
                KeyCode::Esc => {
                    self.list_filter.clear();
                    self.list_search_open = false;
                    self.reapply_list_filter();
                }
                KeyCode::Enter => self.list_search_open = false,
                KeyCode::Backspace => {
                    self.list_filter.pop();
                    self.reapply_list_filter();
                }
                KeyCode::Char(c) => {
                    self.list_filter.push(c);
                    self.reapply_list_filter();
                }
                _ => {}
            }
            return Ok(());
        }

        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q')) if self.list_filter.is_empty() => {
                self.quit();
            }
            (_, KeyCode::Char('q')) => {
                self.list_filter.clear();
                self.reapply_list_filter();
            }
            (KeyModifiers::CONTROL, KeyCode::Char('c' | 'C')) => {
                self.quit();
            }
            (_, KeyCode::Char('/')) => self.list_search_open = true,
            (_, KeyCode::Char('1')) => {
                self.switch_view(ResourceKind::Containers);
            }
            (_, KeyCode::Char('2')) => self.switch_view(ResourceKind::Images),
            (_, KeyCode::Char('3')) => self.switch_view(ResourceKind::Volumes),
            (_, KeyCode::Char('4')) => self.switch_view(ResourceKind::Networks),
            (_, KeyCode::Char('j') | KeyCode::Down) => {
                self.navigate(KeyCode::Down);
            }
            (_, KeyCode::Char('k') | KeyCode::Up) => self.navigate(KeyCode::Up),
            (_, KeyCode::Char('g')) => self.navigate(KeyCode::Char('g')),
            (_, KeyCode::Char('G')) => self.navigate(KeyCode::Char('G')),
            (_, KeyCode::Char('s' | 'S'))
                if self.view == ResourceKind::Containers =>
            {
                if let Some(row) = self.selected_container().cloned() {
                    if row.status.eq_ignore_ascii_case("running") {
                        self.stop_container(&row.id).await;
                    } else {
                        self.start_container(&row.id).await;
                    }
                }
            }
            (_, KeyCode::Char('r'))
                if self.view == ResourceKind::Containers =>
            {
                if let Some(row) = self.selected_container().cloned() {
                    self.restart_container(&row.id).await;
                }
            }
            (_, KeyCode::Char('l'))
                if self.view == ResourceKind::Containers =>
            {
                if let Some(row) = self.selected_container().cloned() {
                    self.open_logs(&row.id, &row.name);
                }
            }
            (_, KeyCode::Char('d') | KeyCode::Enter) => {
                self.describe_selected().await;
            }
            (_, KeyCode::Char('e'))
                if self.view == ResourceKind::Containers =>
            {
                if let Some(row) = self.selected_container().cloned() {
                    self.exec_shell(terminal, &row.id)?;
                }
            }
            (_, KeyCode::Char('D') | KeyCode::Delete) => {
                self.prompt_remove_selected();
            }
            _ => {}
        }
        Ok(())
    }

    /// Switch the active list view. Selection state per resource kind is
    /// preserved on its own `TableDataState`, so no reset is needed here.
    fn switch_view(&mut self, kind: ResourceKind) {
        self.view = kind;
        self.hotkeys = match kind {
            ResourceKind::Containers => {
                crate::ui::widgets::hotkeys::LIST_HOTKEYS.to_vec()
            }
            ResourceKind::Images
            | ResourceKind::Volumes
            | ResourceKind::Networks => {
                crate::ui::widgets::hotkeys::RESOURCE_HOTKEYS.to_vec()
            }
        };
    }

    fn navigate(&mut self, key: KeyCode) {
        match self.view {
            ResourceKind::Containers => TableNavigationHandler::navigate_table(
                &self.containers.model,
                &mut self.containers.view,
                key,
            ),
            ResourceKind::Images => TableNavigationHandler::navigate_table(
                &self.images.model,
                &mut self.images.view,
                key,
            ),
            ResourceKind::Volumes => TableNavigationHandler::navigate_table(
                &self.volumes.model,
                &mut self.volumes.view,
                key,
            ),
            ResourceKind::Networks => TableNavigationHandler::navigate_table(
                &self.networks.model,
                &mut self.networks.view,
                key,
            ),
        }
    }

    /// Open the remove-confirmation dialog for whatever's selected in the
    /// active view, if anything is selected.
    /// `d`/`Enter`: describe the selected container (other views: status hint).
    async fn describe_selected(&mut self) {
        if self.view != ResourceKind::Containers {
            self.set_status("Describe is only available for containers");
        } else if let Some(row) = self.selected_container().cloned() {
            self.open_describe(&row.id, &row.name).await;
        }
    }

    fn prompt_remove_selected(&mut self) {
        let Some((kind, id, label)) = (match self.view {
            ResourceKind::Containers => self
                .selected_container()
                .map(|r| (self.view, r.id.clone(), r.name.clone())),
            ResourceKind::Images => self
                .selected_image()
                .map(|r| (self.view, r.id.clone(), r.repo_tags.clone())),
            ResourceKind::Volumes => self
                .selected_volume()
                .map(|r| (self.view, r.name.clone(), r.name.clone())),
            ResourceKind::Networks => self
                .selected_network()
                .map(|r| (self.view, r.id.clone(), r.name.clone())),
        }) else {
            return;
        };

        self.pending_remove = Some((kind, id));
        self.confirm_dialog = Some(ConfirmDialog::new(
            format!(" Remove {}? ", singular(kind)),
            format!(
                "Remove {} '{label}'?\nThis action cannot be undone.",
                singular(kind)
            ),
            modal_border(),
            1,
        ));
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

    async fn remove_resource(&mut self, kind: ResourceKind, id: &str) {
        let Some(docker) = self.docker.clone() else {
            return;
        };
        let result = match kind {
            ResourceKind::Containers => docker.remove(id).await,
            ResourceKind::Images => docker.remove_image(id).await,
            ResourceKind::Volumes => docker.remove_volume(id).await,
            ResourceKind::Networks => docker.remove_network(id).await,
        };
        match result {
            Ok(()) => self.set_status(format!("{} removed", singular(kind))),
            Err(e) => self.set_status(format!("Remove failed: {e}")),
        }
    }
}

fn modal_border() -> Style {
    theme::border()
}

/// Human-readable singular label for a resource kind, for status/dialog text.
const fn singular(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Containers => "Container",
        ResourceKind::Images => "Image",
        ResourceKind::Volumes => "Volume",
        ResourceKind::Networks => "Network",
    }
}
