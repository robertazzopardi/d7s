use bollard::{
    Docker,
    models::{ContainerInspectResponse, ImageInspect, NetworkInspect, Volume},
    query_parameters::{
        InspectContainerOptions, InspectNetworkOptions, ListContainersOptions,
        ListImagesOptions, ListNetworksOptions, ListVolumesOptions,
        LogsOptions, RemoveContainerOptions, RemoveImageOptions,
        RemoveVolumeOptions, RestartContainerOptions, StopContainerOptions,
    },
};
use color_eyre::Result;
use futures_util::StreamExt;
use tokio::sync::mpsc::UnboundedSender;

use super::{ContainerRow, ImageRow, NetworkRow, VolumeRow};

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

    /// A client pointed at a closed loopback port: every call fails fast.
    /// Lets key-handler tests reach the docker layer without a daemon, on
    /// every platform (unix sockets don't exist on Windows).
    #[cfg(test)]
    pub fn unreachable() -> Self {
        let docker = Docker::connect_with_http(
            "http://127.0.0.1:1",
            1,
            bollard::API_DEFAULT_VERSION,
        )
        .expect("lazy connect");
        Self { docker }
    }

    /// Verify the daemon is actually reachable (connect succeeds lazily otherwise).
    pub async fn ping(&self) -> Result<()> {
        self.docker.ping().await?;
        Ok(())
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

    pub async fn list_images(&self) -> Result<Vec<ImageRow>> {
        let options = ListImagesOptions {
            all: false,
            ..Default::default()
        };
        let summaries = self.docker.list_images(Some(options)).await?;
        Ok(summaries.iter().map(ImageRow::from_summary).collect())
    }

    pub async fn remove_image(&self, id: &str) -> Result<()> {
        self.docker
            .remove_image(
                id,
                Some(RemoveImageOptions {
                    force: true,
                    ..Default::default()
                }),
                None,
            )
            .await?;
        Ok(())
    }

    pub async fn list_volumes(&self) -> Result<Vec<VolumeRow>> {
        let response =
            self.docker.list_volumes(None::<ListVolumesOptions>).await?;
        Ok(response
            .volumes
            .unwrap_or_default()
            .iter()
            .map(VolumeRow::from_volume)
            .collect())
    }

    pub async fn remove_volume(&self, name: &str) -> Result<()> {
        self.docker
            .remove_volume(name, Some(RemoveVolumeOptions { force: true }))
            .await?;
        Ok(())
    }

    pub async fn list_networks(&self) -> Result<Vec<NetworkRow>> {
        let networks = self
            .docker
            .list_networks(None::<ListNetworksOptions>)
            .await?;
        Ok(networks.iter().map(NetworkRow::from_network).collect())
    }

    pub async fn remove_network(&self, name: &str) -> Result<()> {
        self.docker.remove_network(name).await?;
        Ok(())
    }

    /// Fetch full inspect details for `id` (id, image, status, ports, mounts, env, created time).
    pub async fn inspect(&self, id: &str) -> Result<ContainerInspectResponse> {
        Ok(self
            .docker
            .inspect_container(id, None::<InspectContainerOptions>)
            .await?)
    }

    pub async fn inspect_image(&self, id: &str) -> Result<ImageInspect> {
        Ok(self.docker.inspect_image(id).await?)
    }

    pub async fn inspect_volume(&self, name: &str) -> Result<Volume> {
        Ok(self.docker.inspect_volume(name).await?)
    }

    pub async fn inspect_network(&self, id: &str) -> Result<NetworkInspect> {
        Ok(self
            .docker
            .inspect_network(id, None::<InspectNetworkOptions>)
            .await?)
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
