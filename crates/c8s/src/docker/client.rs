use bollard::{
    Docker,
    query_parameters::{
        ListContainersOptions, LogsOptions, RemoveContainerOptions,
        RestartContainerOptions, StopContainerOptions,
    },
};
use color_eyre::Result;
use futures_util::StreamExt;
use tokio::sync::mpsc::UnboundedSender;

use super::ContainerRow;

/// Snapshot of daemon-level health/info — c8s's single-daemon analog of a
/// fleet-wide health dashboard (there's only ever one daemon to summarize).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DaemonHealth {
    pub server_version: String,
    pub api_version: String,
    pub os: String,
    pub arch: String,
    pub containers_running: i64,
    pub containers_paused: i64,
    pub containers_stopped: i64,
    pub images: i64,
}

/// Thin wrapper around a bollard `Docker` handle, scoped to what c8s needs.
#[derive(Clone)]
pub struct DockerClient {
    docker: Docker,
}

impl DockerClient {
    /// Connect to the local Docker daemon over the default socket.
    pub fn connect() -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()?;
        Ok(Self { docker })
    }

    /// Verify the daemon is actually reachable (connect succeeds lazily otherwise).
    pub async fn ping(&self) -> Result<()> {
        self.docker.ping().await?;
        Ok(())
    }

    /// Cheap daemon-level summary (version + container counts by state), for
    /// c8s's health panel. Two lightweight API calls; no per-container size scan.
    pub async fn daemon_health(&self) -> Result<DaemonHealth> {
        let version = self.docker.version().await?;
        let info = self.docker.info().await?;
        Ok(DaemonHealth {
            server_version: version.version.unwrap_or_default(),
            api_version: version.api_version.unwrap_or_default(),
            os: version.os.unwrap_or_default(),
            arch: version.arch.unwrap_or_default(),
            containers_running: info.containers_running.unwrap_or_default(),
            containers_paused: info.containers_paused.unwrap_or_default(),
            containers_stopped: info.containers_stopped.unwrap_or_default(),
            images: info.images.unwrap_or_default(),
        })
    }

    pub async fn list_containers(&self) -> Result<Vec<ContainerRow>> {
        let options = ListContainersOptions {
            all: true,
            ..Default::default()
        };
        let summaries = self.docker.list_containers(Some(options)).await?;
        Ok(summaries.iter().map(ContainerRow::from_summary).collect())
    }

    pub async fn start(&self, id: &str) -> Result<()> {
        self.docker.start_container(id, None).await?;
        Ok(())
    }

    pub async fn stop(&self, id: &str) -> Result<()> {
        self.docker
            .stop_container(id, None::<StopContainerOptions>)
            .await?;
        Ok(())
    }

    pub async fn restart(&self, id: &str) -> Result<()> {
        self.docker
            .restart_container(id, None::<RestartContainerOptions>)
            .await?;
        Ok(())
    }

    pub async fn remove(&self, id: &str) -> Result<()> {
        self.docker
            .remove_container(
                id,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await?;
        Ok(())
    }

    /// Stream log lines for `id` into `tx` until the stream ends or the
    /// receiver is dropped (e.g. the user left the log view).
    pub async fn tail_logs(&self, id: &str, tx: UnboundedSender<String>) {
        let options = LogsOptions {
            follow: true,
            stdout: true,
            stderr: true,
            tail: "200".to_string(),
            ..Default::default()
        };
        let mut stream = self.docker.logs(id, Some(options));
        while let Some(chunk) = stream.next().await {
            let line = match chunk {
                Ok(output) => output.to_string(),
                Err(e) => format!("[log stream error: {e}]"),
            };
            for line in line.lines() {
                if tx.send(line.to_string()).is_err() {
                    return;
                }
            }
        }
    }
}
