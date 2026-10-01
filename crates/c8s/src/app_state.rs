/// Top-level application state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum AppState {
    /// Still trying to reach the Docker daemon.
    #[default]
    Connecting,
    /// Daemon unreachable — show the error and offer retry.
    ConnectError(String),
    /// Normal resource list view (containers/images/volumes/networks).
    List,
    /// Full-screen log tail for one container.
    Logs { id: String, name: String },
}

/// Which resource-type table the list view is currently showing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ResourceKind {
    #[default]
    Containers,
    Images,
    Volumes,
    Networks,
}

impl ResourceKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Containers => "Containers",
            Self::Images => "Images",
            Self::Volumes => "Volumes",
            Self::Networks => "Networks",
        }
    }
}
