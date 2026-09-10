use bollard::{
    Docker,
    container::{
        ListContainersOptions, LogsOptions, RemoveContainerOptions,
        RestartContainerOptions, StopContainerOptions,
    },
};
use color_eyre::Result;
use futures_util::StreamExt;
use tokio::sync::mpsc::UnboundedSender;

use super::ContainerRow;

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

    pub async fn list_containers(&self) -> Result<Vec<ContainerRow>> {
        let options = ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        };
        let summaries =
            self.docker.list_containers(Some(options)).await?;
        Ok(summaries.iter().map(ContainerRow::from_summary).collect())
    }

    pub async fn start(&self, id: &str) -> Result<()> {
        self.docker.start_container::<String>(id, None).await?;
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
        let options = LogsOptions::<String> {
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
