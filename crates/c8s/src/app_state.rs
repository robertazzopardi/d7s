/// Top-level application state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum AppState {
    /// Still trying to reach the Docker daemon.
    #[default]
    Connecting,
    /// Daemon unreachable — show the error and offer retry.
    ConnectError(String),
    /// Normal container list view.
    List,
    /// Full-screen log tail for one container.
    Logs { id: String, name: String },
}
