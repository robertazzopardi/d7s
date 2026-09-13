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
            .map(|n| n.trim_start_matches('/').to_string())
            .unwrap_or_else(|| short_id(&id));
        let image = summary.image.clone().unwrap_or_default();
        let status = summary.state.clone().unwrap_or_default();
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

fn short_id(id: &str) -> String {
    id.get(..12.min(id.len())).unwrap_or(id).to_string()
}

fn format_port(port: &bollard::models::Port) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use bollard::models::{ContainerSummary, Port, PortTypeEnum};

    fn summary() -> ContainerSummary {
        ContainerSummary {
            id: Some("abcdef0123456789".to_string()),
            names: Some(vec!["/my-app".to_string()]),
            image: Some("nginx:latest".to_string()),
            state: Some("running".to_string()),
            status: Some("Up 3 hours".to_string()),
            ports: Some(vec![Port {
                ip: Some("0.0.0.0".to_string()),
                private_port: 80,
                public_port: Some(8080),
                typ: Some(PortTypeEnum::TCP),
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
