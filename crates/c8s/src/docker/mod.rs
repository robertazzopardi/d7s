pub mod client;

use k9tui::widgets::table::TableData;

/// One row in the container list table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContainerRow {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub ports: String,
    pub uptime: String,
}

impl ContainerRow {
    /// Build a row from a bollard container-summary response.
    #[must_use]
    pub fn from_summary(summary: &bollard::models::ContainerSummary) -> Self {
        let id = summary.id.clone().unwrap_or_default();
        let name = summary
            .names
            .as_ref()
            .and_then(|names| names.first())
            .map_or_else(
                || short_id(&id),
                |n| n.trim_start_matches('/').to_string(),
            );
        let image = summary.image.clone().unwrap_or_default();
        let status = summary.state.map(|s| s.to_string()).unwrap_or_default();
        let uptime = summary.status.clone().unwrap_or_default();
        let ports = summary
            .ports
            .as_ref()
            .map(|ports| {
                ports.iter().map(format_port).collect::<Vec<_>>().join(", ")
            })
            .unwrap_or_default();

        Self {
            id,
            name,
            image,
            status,
            ports,
            uptime,
        }
    }
}

/// Render a k9s-style "describe" text block from a full container inspect
/// response — whatever bollard already gives us, no extra API calls.
#[must_use]
pub fn describe_text(inspect: &bollard::models::ContainerInspectResponse) -> String {
    let mut out = String::new();
    let mut line = |label: &str, value: &str| {
        out.push_str(&format!("{label:<14}{value}\n"));
    };

    line("ID:", inspect.id.as_deref().unwrap_or(""));
    line(
        "Name:",
        inspect.name.as_deref().unwrap_or("").trim_start_matches('/'),
    );
    line(
        "Image:",
        inspect
            .config
            .as_ref()
            .and_then(|c| c.image.as_deref())
            .unwrap_or(""),
    );
    if let Some(created) = &inspect.created {
        line("Created:", &created.to_string());
    }
    if let Some(state) = &inspect.state {
        line(
            "Status:",
            &state.status.map(|s| s.to_string()).unwrap_or_default(),
        );
        if let Some(pid) = state.pid {
            line("PID:", &pid.to_string());
        }
        if let Some(started) = &state.started_at {
            line("Started:", &started.to_string());
        }
        if let Some(code) = state.exit_code {
            line("ExitCode:", &code.to_string());
        }
    }

    out.push('\n');
    out.push_str("Ports:\n");
    let ports = inspect
        .network_settings
        .as_ref()
        .and_then(|ns| ns.ports.as_ref());
    match ports {
        Some(ports) if !ports.is_empty() => {
            for (container_port, bindings) in ports {
                match bindings {
                    Some(bindings) if !bindings.is_empty() => {
                        for binding in bindings {
                            let ip = binding.host_ip.as_deref().unwrap_or("");
                            let port = binding.host_port.as_deref().unwrap_or("");
                            out.push_str(&format!(
                                "  {container_port} -> {ip}:{port}\n"
                            ));
                        }
                    }
                    _ => out.push_str(&format!("  {container_port}\n")),
                }
            }
        }
        _ => out.push_str("  (none)\n"),
    }

    out.push('\n');
    out.push_str("Mounts:\n");
    match &inspect.mounts {
        Some(mounts) if !mounts.is_empty() => {
            for mount in mounts {
                let src = mount.source.as_deref().unwrap_or("");
                let dst = mount.destination.as_deref().unwrap_or("");
                let mode = mount.mode.as_deref().unwrap_or("");
                out.push_str(&format!("  {src} -> {dst} ({mode})\n"));
            }
        }
        _ => out.push_str("  (none)\n"),
    }

    out.push('\n');
    out.push_str("Env:\n");
    match inspect.config.as_ref().and_then(|c| c.env.as_ref()) {
        Some(env) if !env.is_empty() => {
            for e in env {
                out.push_str(&format!("  {e}\n"));
            }
        }
        _ => out.push_str("  (none)\n"),
    }

    out
}

fn short_id(id: &str) -> String {
    id.get(..12.min(id.len())).unwrap_or(id).to_string()
}

fn format_port(port: &bollard::models::PortSummary) -> String {
    let proto = port.typ.map(|t| t.to_string()).unwrap_or_default();
    match (port.ip.as_ref(), port.public_port) {
        (Some(ip), Some(public)) => {
            format!("{ip}:{public}->{}/{proto}", port.private_port)
        }
        (None, Some(public)) => {
            format!("{public}->{}/{proto}", port.private_port)
        }
        _ => format!("{}/{proto}", port.private_port),
    }
}

const COLUMNS: [&str; 5] = ["NAME", "IMAGE", "STATUS", "PORTS", "UPTIME"];

impl TableData for ContainerRow {
    fn title() -> &'static str {
        "Containers"
    }

    fn ref_array(&self) -> Vec<String> {
        vec![
            self.name.clone(),
            self.image.clone(),
            self.status.clone(),
            self.ports.clone(),
            self.uptime.clone(),
        ]
    }

    fn num_columns(&self) -> usize {
        COLUMNS.len()
    }

    fn cols() -> Vec<&'static str> {
        COLUMNS.to_vec()
    }
}

/// One row in the image list table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImageRow {
    pub id: String,
    pub repo_tags: String,
    pub size: String,
    pub created: String,
}

impl ImageRow {
    #[must_use]
    pub fn from_summary(summary: &bollard::models::ImageSummary) -> Self {
        let repo_tags = if summary.repo_tags.is_empty() {
            "<none>:<none>".to_string()
        } else {
            summary.repo_tags.join(", ")
        };
        Self {
            id: short_id(summary.id.trim_start_matches("sha256:")),
            repo_tags,
            size: format_size(summary.size),
            created: format_timestamp(summary.created),
        }
    }
}

const IMAGE_COLUMNS: [&str; 4] = ["REPO:TAG", "ID", "SIZE", "CREATED"];

impl TableData for ImageRow {
    fn title() -> &'static str {
        "Images"
    }

    fn ref_array(&self) -> Vec<String> {
        vec![
            self.repo_tags.clone(),
            self.id.clone(),
            self.size.clone(),
            self.created.clone(),
        ]
    }

    fn num_columns(&self) -> usize {
        IMAGE_COLUMNS.len()
    }

    fn cols() -> Vec<&'static str> {
        IMAGE_COLUMNS.to_vec()
    }
}

/// One row in the volume list table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VolumeRow {
    pub name: String,
    pub driver: String,
    pub mountpoint: String,
}

impl VolumeRow {
    #[must_use]
    pub fn from_volume(volume: &bollard::models::Volume) -> Self {
        Self {
            name: volume.name.clone(),
            driver: volume.driver.clone(),
            mountpoint: volume.mountpoint.clone(),
        }
    }
}

const VOLUME_COLUMNS: [&str; 3] = ["NAME", "DRIVER", "MOUNTPOINT"];

impl TableData for VolumeRow {
    fn title() -> &'static str {
        "Volumes"
    }

    fn ref_array(&self) -> Vec<String> {
        vec![
            self.name.clone(),
            self.driver.clone(),
            self.mountpoint.clone(),
        ]
    }

    fn num_columns(&self) -> usize {
        VOLUME_COLUMNS.len()
    }

    fn cols() -> Vec<&'static str> {
        VOLUME_COLUMNS.to_vec()
    }
}

/// One row in the network list table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NetworkRow {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub scope: String,
}

impl NetworkRow {
    #[must_use]
    pub fn from_network(network: &bollard::models::Network) -> Self {
        Self {
            id: network.id.as_deref().map_or_else(String::new, short_id),
            name: network.name.clone().unwrap_or_default(),
            driver: network.driver.clone().unwrap_or_default(),
            scope: network.scope.clone().unwrap_or_default(),
        }
    }
}

const NETWORK_COLUMNS: [&str; 3] = ["NAME", "DRIVER", "SCOPE"];

impl TableData for NetworkRow {
    fn title() -> &'static str {
        "Networks"
    }

    fn ref_array(&self) -> Vec<String> {
        vec![self.name.clone(), self.driver.clone(), self.scope.clone()]
    }

    fn num_columns(&self) -> usize {
        NETWORK_COLUMNS.len()
    }

    fn cols() -> Vec<&'static str> {
        NETWORK_COLUMNS.to_vec()
    }
}

fn format_size(bytes: i64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    #[allow(clippy::cast_precision_loss)]
    let mut size = bytes.max(0) as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    format!("{size:.1}{}", UNITS.get(unit).unwrap_or(&"B"))
}

/// Render a Unix timestamp as a rough "N <unit> ago" string, docker-CLI style.
fn format_timestamp(secs: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let created = secs.max(0).unsigned_abs();
    let elapsed = now.saturating_sub(created);
    match elapsed {
        s if s < 60 => format!("{s}s ago"),
        s if s < 3600 => format!("{}m ago", s / 60),
        s if s < 86400 => format!("{}h ago", s / 3600),
        s => format!("{}d ago", s / 86400),
    }
}

#[cfg(test)]
mod tests {
    use bollard::models::{
        ContainerSummary, ContainerSummaryStateEnum, PortSummary,
        PortSummaryTypeEnum,
    };

    use super::*;

    fn summary() -> ContainerSummary {
        ContainerSummary {
            id: Some("abcdef0123456789".to_string()),
            names: Some(vec!["/my-app".to_string()]),
            image: Some("nginx:latest".to_string()),
            state: Some(ContainerSummaryStateEnum::RUNNING),
            status: Some("Up 3 hours".to_string()),
            ports: Some(vec![PortSummary {
                ip: Some("0.0.0.0".to_string()),
                private_port: 80,
                public_port: Some(8080),
                typ: Some(PortSummaryTypeEnum::TCP),
            }]),
            ..Default::default()
        }
    }

    #[test]
    fn parses_summary_into_row() {
        let row = ContainerRow::from_summary(&summary());
        assert_eq!(row.name, "my-app");
        assert_eq!(row.image, "nginx:latest");
        assert_eq!(row.status, "running");
        assert_eq!(row.uptime, "Up 3 hours");
        assert_eq!(row.ports, "0.0.0.0:8080->80/tcp");
        assert_eq!(row.id, "abcdef0123456789");
    }

    #[test]
    fn falls_back_to_short_id_when_unnamed() {
        let mut s = summary();
        s.names = None;
        let row = ContainerRow::from_summary(&s);
        assert_eq!(row.name, "abcdef012345");
    }

    #[test]
    fn handles_missing_ports() {
        let mut s = summary();
        s.ports = None;
        let row = ContainerRow::from_summary(&s);
        assert_eq!(row.ports, "");
    }

    #[test]
    fn table_data_maps_columns_in_order() {
        let row = ContainerRow::from_summary(&summary());
        assert_eq!(ContainerRow::cols(), COLUMNS.to_vec());
        assert_eq!(
            row.ref_array(),
            vec![
                "my-app".to_string(),
                "nginx:latest".to_string(),
                "running".to_string(),
                "0.0.0.0:8080->80/tcp".to_string(),
                "Up 3 hours".to_string(),
            ]
        );
        assert_eq!(row.num_columns(), 5);
    }
}
