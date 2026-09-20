use crate::docker::client::DaemonHealth;

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
    /// Daemon health/info panel (c8s's single-daemon analog of a fleet health
    /// dashboard) — version, API version, OS/arch, container counts, image count.
    Info(DaemonHealth),
}
