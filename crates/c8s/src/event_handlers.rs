use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use k9tui::{
    theme,
    widgets::{
        modal::{ConfirmDialog, DialogAction},
        navigation::TableNavigationHandler,
    },
};
use ratatui::{Terminal, backend::Backend, style::Style};

use crate::{
    app::App,
    app_state::{AppState, ResourceKind},
};

impl App {
    pub async fn on_key_event<B: Backend>(
        &mut self,
        key: KeyEvent,
        terminal: &mut Terminal<B>,
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
    fn on_key_list_search(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                self.list_filter.clear();
                self.list_search_open = false;
                self.reapply_list_filter();
            }
            (_, KeyCode::Enter) => self.list_search_open = false,
            (_, KeyCode::Backspace) => {
                self.list_filter.pop();
                self.reapply_list_filter();
            }
            (KeyModifiers::CONTROL, KeyCode::Char('c' | 'C')) => self.quit(),
            (_, KeyCode::Char(c)) => {
                self.list_filter.push(c);
                self.reapply_list_filter();
            }
            _ => {}
        }
    }

    /// Search-bar editing, `q` (clear filter, else quit), Ctrl-C and `/`.
    /// Returns true when the key was consumed. Terminal-free so it is testable.
    fn on_key_list_filter_or_quit(&mut self, key: KeyEvent) -> bool {
        if self.list_search_open {
            self.on_key_list_search(key);
            return true;
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
            _ => return false,
        }
        true
    }

    #[allow(clippy::wildcard_enum_match_arm)]
    #[allow(clippy::too_many_lines)] // flat key-dispatch match
    async fn on_key_list<B: Backend>(
        &mut self,
        key: KeyEvent,
        terminal: &mut Terminal<B>,
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

        if self.on_key_list_filter_or_quit(key) {
            return Ok(());
        }

        match (key.modifiers, key.code) {
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

    /// `d`/`Enter`: describe whatever is selected in the active view.
    async fn describe_selected(&mut self) {
        let Some((id, name)) = self.selected_target() else {
            return;
        };
        self.open_describe(self.view, &id, &name).await;
    }

    /// (docker id-or-name, display label) of the selection in the active view.
    fn selected_target(&self) -> Option<(String, String)> {
        match self.view {
            ResourceKind::Containers => self
                .selected_container()
                .map(|r| (r.id.clone(), r.name.clone())),
            ResourceKind::Images => self
                .selected_image()
                .map(|r| (r.id.clone(), r.repo_tags.clone())),
            ResourceKind::Volumes => self
                .selected_volume()
                .map(|r| (r.name.clone(), r.name.clone())),
            ResourceKind::Networks => self
                .selected_network()
                .map(|r| (r.id.clone(), r.name.clone())),
        }
    }

    /// Open the remove-confirmation dialog for the selection, if any.
    fn prompt_remove_selected(&mut self) {
        let kind = self.view;
        let Some((id, label)) = self.selected_target() else {
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
            1, // default to No (k9s-style)
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

#[cfg(test)]
mod tests {
    use k9tui::widgets::table::TableDataState;
    use ratatui::backend::TestBackend;

    use super::*;
    use crate::docker::{
        ContainerRow, ImageRow, NetworkRow, VolumeRow, client::DockerClient,
    };

    type Term = Terminal<TestBackend>;

    fn term() -> Term {
        Terminal::new(TestBackend::new(120, 30)).unwrap()
    }

    /// App with one row selected in every view and a daemon that is always
    /// unreachable, so docker calls fail fast and surface in the status line.
    fn app() -> App {
        let mut app = App::new();
        app.state = AppState::List;
        app.docker = Some(DockerClient::unreachable());
        app.containers = TableDataState::new(vec![ContainerRow {
            id: "c1".into(),
            name: "web".into(),
            ..Default::default()
        }]);
        app.images = TableDataState::new(vec![ImageRow {
            id: "i1".into(),
            repo_tags: "nginx:latest".into(),
            ..Default::default()
        }]);
        app.volumes = TableDataState::new(vec![VolumeRow {
            name: "data".into(),
            ..Default::default()
        }]);
        app.networks = TableDataState::new(vec![NetworkRow {
            id: "n1".into(),
            name: "backend".into(),
            ..Default::default()
        }]);
        app.containers.view.state.select(Some(0));
        app.images.view.state.select(Some(0));
        app.volumes.view.state.select(Some(0));
        app.networks.view.state.select(Some(0));
        app
    }

    /// Real terminals report `D` as Char('D') + SHIFT.
    fn shift_d() -> KeyEvent {
        KeyEvent::new(KeyCode::Char('D'), KeyModifiers::SHIFT)
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn screen(app: &mut App, term: &mut Term) -> String {
        term.draw(|f| app.render(f)).unwrap();
        let buf = term.backend().buffer();
        buf.content
            .chunks(usize::from(buf.area.width))
            .map(|row| {
                row.iter()
                    .map(ratatui::buffer::Cell::symbol)
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    const KINDS: [ResourceKind; 4] = [
        ResourceKind::Containers,
        ResourceKind::Images,
        ResourceKind::Volumes,
        ResourceKind::Networks,
    ];

    #[tokio::test]
    async fn d_and_enter_describe_every_kind() {
        for kind in KINDS {
            for code in [KeyCode::Char('d'), KeyCode::Enter] {
                let mut app = app();
                let mut t = term();
                app.switch_view(kind);
                app.on_key_event(key(code), &mut t).await.unwrap();
                // Reached the inspect call (daemon unreachable) rather than
                // the old "only available for containers" refusal.
                let out = screen(&mut app, &mut t);
                assert!(out.contains("Inspect failed"), "{kind:?}: {out}");
                assert!(!out.contains("only available"), "{kind:?}");
            }
        }
    }

    #[tokio::test]
    async fn shift_d_opens_confirm_for_every_kind_and_enter_removes() {
        for kind in KINDS {
            let mut app = app();
            let mut t = term();
            app.switch_view(kind);
            app.on_key_event(shift_d(), &mut t).await.unwrap();
            assert!(app.confirm_dialog.is_some(), "{kind:?}");
            assert_eq!(app.pending_remove.as_ref().map(|p| p.0), Some(kind));
            assert_eq!(app.state, AppState::List, "D must not describe");

            // Default is No: move to Yes, then Enter attempts the removal.
            app.on_key_event(key(KeyCode::Left), &mut t).await.unwrap();
            app.on_key_event(key(KeyCode::Enter), &mut t).await.unwrap();
            assert!(app.confirm_dialog.is_none());
            assert!(app.pending_remove.is_none());
            let out = screen(&mut app, &mut t);
            assert!(out.contains("Remove failed"), "{kind:?}: {out}");
        }
    }

    #[tokio::test]
    async fn enter_on_default_no_does_not_remove() {
        let mut app = app();
        let mut t = term();
        app.on_key_event(shift_d(), &mut t).await.unwrap();
        app.on_key_event(key(KeyCode::Enter), &mut t).await.unwrap();
        assert!(app.confirm_dialog.is_none() && app.pending_remove.is_none());
        let out = screen(&mut app, &mut t);
        assert!(!out.contains("Remove failed"), "{out}");
    }

    #[tokio::test]
    async fn pending_remove_targets_selected_id() {
        let mut app = app();
        let mut t = term();
        app.switch_view(ResourceKind::Volumes);
        app.on_key_event(key(KeyCode::Delete), &mut t)
            .await
            .unwrap();
        assert_eq!(
            app.pending_remove,
            Some((ResourceKind::Volumes, "data".to_string()))
        );
        app.on_key_event(key(KeyCode::Esc), &mut t).await.unwrap();
        assert!(app.confirm_dialog.is_none() && app.pending_remove.is_none());
    }

    #[tokio::test]
    async fn esc_cancels_confirm_without_removing() {
        let mut app = app();
        let mut t = term();
        app.on_key_event(shift_d(), &mut t).await.unwrap();
        app.on_key_event(key(KeyCode::Esc), &mut t).await.unwrap();
        let out = screen(&mut app, &mut t);
        assert!(!out.contains("Remove failed"));
        assert!(app.pending_remove.is_none());
    }

    #[tokio::test]
    async fn confirm_dialog_renders_in_buffer() {
        let mut app = app();
        let mut t = term();
        app.switch_view(ResourceKind::Networks);
        app.on_key_event(shift_d(), &mut t).await.unwrap();
        let out = screen(&mut app, &mut t);
        assert!(out.contains("Remove Network?"), "{out}");
        assert!(out.contains("Remove Network 'backend'?"));
        assert!(out.contains("Yes") && out.contains("No"));
    }

    #[tokio::test]
    async fn describe_view_renders_and_scrolls() {
        let mut app = app();
        let mut t = term();
        let text = (0..100)
            .map(|i| format!("line-{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        app.state = AppState::Describe {
            name: "web".into(),
            text,
        };
        let out = screen(&mut app, &mut t);
        assert!(out.contains("Describe: web"));
        assert!(out.contains("line-0"));
        assert!(!out.contains("line-99"));

        app.on_key_event(key(KeyCode::Char('G')), &mut t)
            .await
            .unwrap();
        let out = screen(&mut app, &mut t);
        assert!(out.contains("line-99"));
        assert!(!out.contains("line-0\n") || app.describe_scroll > 0);

        app.on_key_event(key(KeyCode::Esc), &mut t).await.unwrap();
        assert_eq!(app.state, AppState::List);
    }
}

#[cfg(test)]
mod filter_tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use crate::{
        app::App,
        docker::{ContainerRow, ImageRow, NetworkRow, VolumeRow},
    };

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn press(app: &mut App, code: KeyCode) -> bool {
        app.on_key_list_filter_or_quit(key(code))
    }

    fn type_str(app: &mut App, s: &str) {
        for c in s.chars() {
            press(app, KeyCode::Char(c));
        }
    }

    fn app() -> App {
        let mut app = App::new();
        app.running = true;
        app.containers_all = vec![
            ContainerRow {
                name: "Web".into(),
                image: "nginx".into(),
                ..Default::default()
            },
            ContainerRow {
                name: "db".into(),
                image: "postgres".into(),
                ..Default::default()
            },
        ];
        app.images_all = vec![ImageRow {
            repo_tags: "nginx:latest".into(),
            ..Default::default()
        }];
        app.volumes_all = vec![
            VolumeRow {
                name: "data".into(),
                ..Default::default()
            },
            VolumeRow {
                name: "cache".into(),
                driver: "NGINX-driver".into(),
                ..Default::default()
            },
        ];
        app.networks_all = vec![NetworkRow {
            name: "bridge".into(),
            scope: "local".into(),
            ..Default::default()
        }];
        app.reapply_list_filter();
        app
    }

    #[test]
    fn slash_opens_search_and_chars_append() {
        let mut app = app();
        assert!(press(&mut app, KeyCode::Char('/')));
        assert!(app.list_search_open);
        type_str(&mut app, "ng");
        assert_eq!(app.list_filter, "ng");
        // 'q' while typing is text, not quit/clear.
        press(&mut app, KeyCode::Char('q'));
        assert_eq!(app.list_filter, "ngq");
        assert!(app.running);
    }

    #[test]
    fn backspace_pops() {
        let mut app = app();
        press(&mut app, KeyCode::Char('/'));
        type_str(&mut app, "abc");
        press(&mut app, KeyCode::Backspace);
        assert_eq!(app.list_filter, "ab");
    }

    #[test]
    fn enter_closes_keeping_filter() {
        let mut app = app();
        press(&mut app, KeyCode::Char('/'));
        type_str(&mut app, "nginx");
        press(&mut app, KeyCode::Enter);
        assert!(!app.list_search_open);
        assert_eq!(app.list_filter, "nginx");
        assert_eq!(app.containers.model.items.len(), 1);
    }

    #[test]
    fn esc_clears_and_closes() {
        let mut app = app();
        press(&mut app, KeyCode::Char('/'));
        type_str(&mut app, "nginx");
        press(&mut app, KeyCode::Esc);
        assert!(!app.list_search_open);
        assert!(app.list_filter.is_empty());
        assert_eq!(app.containers.model.items.len(), 2);
    }

    #[test]
    fn ctrl_c_while_filtering_quits_without_appending() {
        let mut app = app();
        press(&mut app, KeyCode::Char('/'));
        type_str(&mut app, "ab");
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(app.on_key_list_filter_or_quit(ctrl_c));
        assert!(!app.running);
        assert_eq!(app.list_filter, "ab");
    }

    #[test]
    fn ctrl_c_outside_search_quits() {
        let mut app = app();
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(app.on_key_list_filter_or_quit(ctrl_c));
        assert!(!app.running);
    }

    #[test]
    fn q_with_filter_clears_instead_of_quitting() {
        let mut app = app();
        press(&mut app, KeyCode::Char('/'));
        type_str(&mut app, "nginx");
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('q'));
        assert!(app.running);
        assert!(app.list_filter.is_empty());
        assert_eq!(app.containers.model.items.len(), 2);
    }

    #[test]
    fn q_without_filter_quits() {
        let mut app = app();
        assert!(press(&mut app, KeyCode::Char('q')));
        assert!(!app.running);
    }

    #[test]
    fn other_keys_are_not_consumed() {
        let mut app = app();
        assert!(!press(&mut app, KeyCode::Char('j')));
        assert!(app.running);
    }

    #[test]
    fn filter_applies_to_all_lists_case_insensitive_any_column() {
        let mut app = app();
        press(&mut app, KeyCode::Char('/'));
        type_str(&mut app, "NgInX");
        // containers: image column; images: tags; volumes: driver column;
        // networks: no match.
        assert_eq!(app.containers.model.items.len(), 1);
        assert_eq!(app.images.model.items.len(), 1);
        assert_eq!(app.volumes.model.items.len(), 1);
        assert_eq!(app.networks.model.items.len(), 0);
        // Backspacing to empty restores everything.
        for _ in 0..5 {
            press(&mut app, KeyCode::Backspace);
        }
        assert_eq!(app.containers.model.items.len(), 2);
        assert_eq!(app.volumes.model.items.len(), 2);
        assert_eq!(app.networks.model.items.len(), 1);
    }
}
