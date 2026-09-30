use std::time::Duration;

use ansi_to_tui::IntoText;
use color_eyre::Result;
use crossterm::{
    ExecutableCommand,
    event::{DisableBracketedPaste, EnableBracketedPaste, Event, KeyEventKind},
    execute,
};
use k9tui::widgets::{
    hotkey::Hotkey, modal::ConfirmDialog, status_line::StatusLine,
    table::{TableData, TableDataState},
};

use ratatui::{DefaultTerminal, text::Line};
use tokio::sync::mpsc::{
    UnboundedReceiver, UnboundedSender, unbounded_channel,
};

use crate::{
    app_state::{AppState, ResourceKind},
    docker::{ContainerRow, ImageRow, NetworkRow, VolumeRow, client::DockerClient},
};

pub const APP_NAME: &str = r"         ______
  ____  /  __  \  ______
_/ ___\ >      < /  ___/
\  \___/   --   \\___ \
 \___  >______  /____  >
     \/       \/     \/
";

pub const PKG_NAME: &str = env!("CARGO_PKG_NAME");
pub const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

const POLL_INTERVAL: Duration = Duration::from_secs(2);

/// Update pushed from a background task into the app's event loop.
pub enum BackgroundEvent {
    /// Container list refreshed by the poller.
    Containers(Vec<ContainerRow>),
    /// Image list refreshed by the poller.
    Images(Vec<ImageRow>),
    /// Volume list refreshed by the poller.
    Volumes(Vec<VolumeRow>),
    /// Network list refreshed by the poller.
    Networks(Vec<NetworkRow>),
    /// The poller failed to reach the daemon (e.g. it was stopped mid-session).
    PollError(String),
    /// One log line from the active log-tail task.
    LogLine(String),
}

pub struct App {
    pub(crate) running: bool,
    pub(crate) state: AppState,
    /// Which resource table the list view currently shows.
    pub(crate) view: ResourceKind,
    pub(crate) docker: Option<DockerClient>,
    pub(crate) hotkeys: Vec<Hotkey>,
    pub(crate) containers: TableDataState<ContainerRow>,
    pub(crate) images: TableDataState<ImageRow>,
    pub(crate) volumes: TableDataState<VolumeRow>,
    pub(crate) networks: TableDataState<NetworkRow>,
    /// Unfiltered rows from the last poll, kept so the `/` filter can be
    /// re-applied (or cleared) without waiting for the next poll tick.
    pub(crate) containers_all: Vec<ContainerRow>,
    pub(crate) images_all: Vec<ImageRow>,
    pub(crate) volumes_all: Vec<VolumeRow>,
    pub(crate) networks_all: Vec<NetworkRow>,
    /// Case-insensitive substring filter applied to the active list view(s).
    /// Empty = no filter.
    pub(crate) list_filter: String,
    /// True while the `/` search bar is open for editing in the list view.
    pub(crate) list_search_open: bool,
    pub(crate) status_line: StatusLine,
    pub(crate) confirm_dialog: Option<ConfirmDialog>,
    /// Resource kind + id pending removal once the confirm dialog resolves.
    pub(crate) pending_remove: Option<(ResourceKind, String)>,
    pub(crate) log_lines: Vec<Line<'static>>,
    /// Absolute index of the first visible log line, synced each render.
    pub(crate) log_scroll: usize,
    /// True while the log view tracks new output; false once the user scrolls up.
    pub(crate) log_follow: bool,
    /// Height of the last-rendered log viewport, used to clamp scrolling.
    pub(crate) log_viewport_height: usize,
    /// Case-insensitive substring filter applied to log lines. Empty = no filter.
    pub(crate) log_filter: String,
    /// True while the `/` search bar is open for editing.
    pub(crate) log_search_open: bool,
    pub(crate) build_info: String,

    pub(crate) bg_tx: UnboundedSender<BackgroundEvent>,
    pub(crate) bg_rx: UnboundedReceiver<BackgroundEvent>,
    /// Aborts the running log-tail task, if any, when leaving the log view.
    pub(crate) log_task: Option<tokio::task::JoinHandle<()>>,
    poll_task: Option<tokio::task::JoinHandle<()>>,
}

impl App {
    #[must_use]
    pub fn new() -> Self {
        let (bg_tx, bg_rx) = unbounded_channel();
        Self {
            running: false,
            state: AppState::default(),
            view: ResourceKind::default(),
            docker: None,
            hotkeys: crate::ui::widgets::hotkeys::LIST_HOTKEYS.to_vec(),
            containers: TableDataState::new(Vec::new()),
            images: TableDataState::new(Vec::new()),
            volumes: TableDataState::new(Vec::new()),
            networks: TableDataState::new(Vec::new()),
            containers_all: Vec::new(),
            images_all: Vec::new(),
            volumes_all: Vec::new(),
            networks_all: Vec::new(),
            list_filter: String::new(),
            list_search_open: false,
            status_line: StatusLine::new(),
            confirm_dialog: None,
            pending_remove: None,
            log_lines: Vec::new(),
            log_scroll: 0,
            log_follow: true,
            log_viewport_height: 0,
            log_filter: String::new(),
            log_search_open: false,
            build_info: format!("Name: {PKG_NAME}\nVersion: {PKG_VERSION}"),
            bg_tx,
            bg_rx,
            log_task: None,
            poll_task: None,
        }
    }

    /// Attempt to connect to the daemon and, on success, start polling.
    pub async fn connect(&mut self) {
        match DockerClient::connect() {
            Ok(client) => match client.ping().await {
                Ok(()) => {
                    self.docker = Some(client);
                    self.state = AppState::List;
                    self.start_polling();
                }
                Err(e) => {
                    self.state = AppState::ConnectError(format!(
                        "Docker unreachable: {e}"
                    ));
                }
            },
            Err(e) => {
                self.state =
                    AppState::ConnectError(format!("Docker unreachable: {e}"));
            }
        }
    }

    fn start_polling(&mut self) {
        let Some(docker) = self.docker.clone() else {
            return;
        };
        if let Some(handle) = self.poll_task.take() {
            handle.abort();
        }
        let tx = self.bg_tx.clone();
        self.poll_task = Some(tokio::spawn(async move {
            loop {
                match docker.list_containers().await {
                    Ok(rows) => {
                        let _ = tx.send(BackgroundEvent::Containers(rows));
                    }
                    Err(e) => {
                        if tx
                            .send(BackgroundEvent::PollError(e.to_string()))
                            .is_err()
                        {
                            return;
                        }
                    }
                }
                match docker.list_images().await {
                    Ok(rows) => {
                        let _ = tx.send(BackgroundEvent::Images(rows));
                    }
                    Err(e) => {
                        let _ = tx.send(BackgroundEvent::PollError(e.to_string()));
                    }
                }
                match docker.list_volumes().await {
                    Ok(rows) => {
                        let _ = tx.send(BackgroundEvent::Volumes(rows));
                    }
                    Err(e) => {
                        let _ = tx.send(BackgroundEvent::PollError(e.to_string()));
                    }
                }
                match docker.list_networks().await {
                    Ok(rows) => {
                        let _ = tx.send(BackgroundEvent::Networks(rows));
                    }
                    Err(e) => {
                        let _ = tx.send(BackgroundEvent::PollError(e.to_string()));
                    }
                }
                tokio::time::sleep(POLL_INTERVAL).await;
            }
        }));
    }

    pub(crate) fn selected_container(&self) -> Option<&ContainerRow> {
        let idx = self.containers.view.state.selected()?;
        self.containers.model.items.get(idx)
    }

    pub(crate) fn selected_image(&self) -> Option<&ImageRow> {
        let idx = self.images.view.state.selected()?;
        self.images.model.items.get(idx)
    }

    pub(crate) fn selected_volume(&self) -> Option<&VolumeRow> {
        let idx = self.volumes.view.state.selected()?;
        self.volumes.model.items.get(idx)
    }

    pub(crate) fn selected_network(&self) -> Option<&NetworkRow> {
        let idx = self.networks.view.state.selected()?;
        self.networks.model.items.get(idx)
    }

    pub(crate) const fn quit(&mut self) {
        self.running = false;
    }

    pub(crate) fn set_status(&mut self, message: impl Into<String>) {
        self.status_line.set_message(message);
    }

    /// Run the application's main loop.
    pub async fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        self.connect().await;

        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            self.drain_background_events();
            self.poll_terminal_event(&mut terminal).await?;
        }
        Ok(())
    }

    fn drain_background_events(&mut self) {
        while let Ok(event) = self.bg_rx.try_recv() {
            match event {
                BackgroundEvent::Containers(mut rows) => {
                    rows.sort_by_key(|r| r.name.to_lowercase());
                    self.containers_all = rows;
                    let filtered = filter_rows(&self.containers_all, &self.list_filter);
                    apply_table_update(&mut self.containers, filtered);
                }
                BackgroundEvent::Images(mut rows) => {
                    rows.sort_by_key(|r| r.repo_tags.to_lowercase());
                    self.images_all = rows;
                    let filtered = filter_rows(&self.images_all, &self.list_filter);
                    apply_table_update(&mut self.images, filtered);
                }
                BackgroundEvent::Volumes(mut rows) => {
                    rows.sort_by_key(|r| r.name.to_lowercase());
                    self.volumes_all = rows;
                    let filtered = filter_rows(&self.volumes_all, &self.list_filter);
                    apply_table_update(&mut self.volumes, filtered);
                }
                BackgroundEvent::Networks(mut rows) => {
                    rows.sort_by_key(|r| r.name.to_lowercase());
                    self.networks_all = rows;
                    let filtered = filter_rows(&self.networks_all, &self.list_filter);
                    apply_table_update(&mut self.networks, filtered);
                }
                BackgroundEvent::PollError(e) => {
                    self.set_status(format!("Refresh failed: {e}"));
                }
                BackgroundEvent::LogLine(line) => {
                    const MAX_LOG_LINES: usize = 5000;
                    let parsed = line
                        .as_bytes()
                        .into_text()
                        .unwrap_or_else(|_| ratatui::text::Text::raw(line));
                    self.log_lines.extend(parsed.lines);
                    if self.log_lines.len() > MAX_LOG_LINES {
                        let drop = self.log_lines.len() - MAX_LOG_LINES;
                        self.log_lines.drain(..drop);
                        self.log_scroll = self.log_scroll.saturating_sub(drop);
                    }
                }
            }
        }
    }

    /// Poll crossterm for an input event with a short timeout so the loop
    /// keeps spinning to pick up background updates (poller, log tail).
    async fn poll_terminal_event(
        &mut self,
        terminal: &mut DefaultTerminal,
    ) -> Result<()> {
        if !crossterm::event::poll(Duration::from_millis(100))? {
            return Ok(());
        }
        match crossterm::event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                self.on_key_event(key, terminal).await?;
            }
            Event::Key(_)
            | Event::FocusGained
            | Event::FocusLost
            | Event::Mouse(_)
            | Event::Paste(_)
            | Event::Resize(_, _) => {}
        }
        Ok(())
    }

    /// Suspend the TUI, spawn an interactive `docker exec` shell, and resume.
    pub(crate) fn exec_shell(
        &mut self,
        terminal: &mut DefaultTerminal,
        id: &str,
    ) -> Result<()> {
        execute!(std::io::stdout(), DisableBracketedPaste)?;
        std::io::stdout().execute(crossterm::terminal::LeaveAlternateScreen)?;
        crossterm::terminal::disable_raw_mode()?;

        let status = std::process::Command::new("docker")
            .args(["exec", "-it", id, "sh"])
            .status();

        std::io::stdout().execute(crossterm::terminal::EnterAlternateScreen)?;
        crossterm::terminal::enable_raw_mode()?;
        execute!(std::io::stdout(), EnableBracketedPaste)?;
        terminal.clear()?;

        match status {
            Ok(s) if s.success() => self.set_status("Exec session ended"),
            Ok(s) => self.set_status(format!("Exec exited: {s}")),
            Err(e) => self.set_status(format!("Failed to exec: {e}")),
        }
        Ok(())
    }

    /// Enter the log view and start tailing the container's logs.
    pub(crate) fn open_logs(&mut self, id: &str, name: &str) {
        let Some(docker) = self.docker.clone() else {
            return;
        };
        self.log_lines.clear();
        self.log_scroll = 0;
        self.log_follow = true;
        self.log_filter.clear();
        self.log_search_open = false;
        self.state = AppState::Logs {
            id: id.to_string(),
            name: name.to_string(),
        };
        if let Some(handle) = self.log_task.take() {
            handle.abort();
        }
        let tx = self.bg_tx.clone();
        let id = id.to_string();
        self.log_task = Some(tokio::spawn(async move {
            let (line_tx, mut line_rx) = unbounded_channel::<String>();
            let follower = tokio::spawn(async move {
                docker.tail_logs(&id, line_tx).await;
            });
            while let Some(line) = line_rx.recv().await {
                if tx.send(BackgroundEvent::LogLine(line)).is_err() {
                    break;
                }
            }
            follower.abort();
        }));
    }

    /// Leave the log view, stopping the tail task.
    pub(crate) fn close_logs(&mut self) {
        if let Some(handle) = self.log_task.take() {
            handle.abort();
        }
        self.state = AppState::List;
    }

    /// Re-apply the `/` filter to all four resource lists from their
    /// unfiltered masters, e.g. after the filter text changes.
    pub(crate) fn reapply_list_filter(&mut self) {
        let filtered = filter_rows(&self.containers_all, &self.list_filter);
        apply_table_update(&mut self.containers, filtered);
        let filtered = filter_rows(&self.images_all, &self.list_filter);
        apply_table_update(&mut self.images, filtered);
        let filtered = filter_rows(&self.volumes_all, &self.list_filter);
        apply_table_update(&mut self.volumes, filtered);
        let filtered = filter_rows(&self.networks_all, &self.list_filter);
        apply_table_update(&mut self.networks, filtered);
    }
}

/// Case-insensitive substring filter over every column, reusing
/// `TableDataState`'s own filter so the matching logic stays in one place.
fn filter_rows<T: TableData + Clone>(rows: &[T], query: &str) -> Vec<T> {
    if query.is_empty() {
        return rows.to_vec();
    }
    TableDataState::new(rows.to_vec()).filter(query)
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

/// Merge a freshly polled row list into a table, preserving the current
/// selection where possible instead of resetting it to row 0.
fn apply_table_update<T: TableData + Clone>(
    table: &mut TableDataState<T>,
    rows: Vec<T>,
) {
    let selected = table.view.state.selected();
    table.model.longest_item_lens = k9tui::widgets::constraint_len_calculator(&rows);
    table.model.items = rows;

    let len = table.model.items.len();
    match selected {
        Some(sel) if sel >= len => {
            table
                .view
                .state
                .select(if len == 0 { None } else { Some(len - 1) });
        }
        None if len > 0 => table.view.state.select(Some(0)),
        _ => {}
    }
}
