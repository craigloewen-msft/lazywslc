use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Deserialize)]
pub struct Container {
    #[serde(rename = "CreatedAt", deserialize_with = "deserialize_timestamp")]
    pub created_at: i64,
    #[serde(rename = "ID", alias = "Id")]
    pub id: String,
    #[serde(rename = "Image")]
    pub image: String,
    #[serde(rename = "Name", alias = "Names")]
    pub name: String,
    #[serde(rename = "Ports", default, deserialize_with = "deserialize_ports")]
    pub ports: Vec<Port>,
    #[serde(rename = "State", deserialize_with = "deserialize_state")]
    pub state: u8,
    #[serde(
        rename = "StateChangedAt",
        default,
        deserialize_with = "deserialize_optional_timestamp"
    )]
    pub state_changed_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Port {
    #[serde(default)]
    pub binding_address: String,
    #[serde(default)]
    pub host_port: Option<u16>,
    #[serde(default)]
    pub container_port: Option<u16>,
    #[serde(default, deserialize_with = "deserialize_protocol")]
    pub protocol: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Image {
    #[serde(
        rename = "Created",
        alias = "CreatedAt",
        deserialize_with = "deserialize_timestamp"
    )]
    pub created: i64,
    #[serde(rename = "ID", alias = "Id")]
    pub id: String,
    #[serde(rename = "Repository")]
    pub repository: Option<String>,
    #[serde(rename = "Size", deserialize_with = "deserialize_size")]
    pub size: u64,
    #[serde(rename = "Tag")]
    pub tag: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Volume {
    pub driver: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Stats {
    #[serde(default, alias = "CPUPerc")]
    pub cpu_perc: Option<String>,
    #[serde(default, alias = "MemUsage")]
    pub mem_usage: Option<String>,
    #[serde(default, alias = "MemPerc")]
    pub mem_perc: Option<String>,
    #[serde(default, alias = "NetIO")]
    pub net_io: Option<String>,
    #[serde(default, alias = "BlockIO")]
    pub block_io: Option<String>,
    #[serde(default, alias = "PIDs")]
    pub pids: Option<serde_json::Value>,
    #[serde(default, alias = "Name")]
    pub name: Option<String>,
    #[serde(default, alias = "Container", alias = "ID")]
    pub container: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum NumberOrString {
    Signed(i64),
    Unsigned(u64),
    String(String),
}

fn deserialize_timestamp<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = NumberOrString::deserialize(deserializer)?;
    parse_timestamp(value).map_err(serde::de::Error::custom)
}

fn deserialize_optional_timestamp<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<NumberOrString>::deserialize(deserializer)?;
    match value {
        Some(value) => parse_timestamp(value).map_err(serde::de::Error::custom),
        None => Ok(0),
    }
}

fn parse_timestamp(value: NumberOrString) -> Result<i64, String> {
    match value {
        NumberOrString::Signed(value) => Ok(value),
        NumberOrString::Unsigned(value) => {
            i64::try_from(value).map_err(|_| format!("timestamp {value} is too large"))
        }
        NumberOrString::String(value) => {
            if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(&value) {
                return Ok(parsed.timestamp());
            }
            if let Ok(parsed) = chrono::DateTime::parse_from_str(&value, "%Y-%m-%d %H:%M:%S %z") {
                return Ok(parsed.timestamp());
            }

            let without_zone_name = value
                .rsplit_once(' ')
                .map(|(prefix, _)| prefix)
                .unwrap_or(&value);
            chrono::DateTime::parse_from_str(without_zone_name, "%Y-%m-%d %H:%M:%S %z")
                .map(|parsed| parsed.timestamp())
                .map_err(|error| format!("invalid timestamp '{value}': {error}"))
        }
    }
}

fn deserialize_state<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: Deserializer<'de>,
{
    let value = NumberOrString::deserialize(deserializer)?;
    match value {
        NumberOrString::Signed(value) => u8::try_from(value).map_err(serde::de::Error::custom),
        NumberOrString::Unsigned(value) => u8::try_from(value).map_err(serde::de::Error::custom),
        NumberOrString::String(value) => match value.to_ascii_lowercase().as_str() {
            "created" => Ok(0),
            "running" | "restarting" => Ok(2),
            "exited" | "dead" => Ok(3),
            "paused" => Ok(4),
            "stopped" => Ok(5),
            _ => Err(serde::de::Error::custom(format!(
                "unknown container state '{value}'"
            ))),
        },
    }
}

fn deserialize_size<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = NumberOrString::deserialize(deserializer)?;
    match value {
        NumberOrString::Signed(value) => u64::try_from(value).map_err(serde::de::Error::custom),
        NumberOrString::Unsigned(value) => Ok(value),
        NumberOrString::String(value) => parse_size(&value).map_err(serde::de::Error::custom),
    }
}

fn deserialize_protocol<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<NumberOrString>::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(NumberOrString::Signed(value)) => protocol_number(value)
            .map(Some)
            .map_err(serde::de::Error::custom),
        Some(NumberOrString::Unsigned(value)) => i64::try_from(value)
            .map_err(serde::de::Error::custom)
            .and_then(|value| {
                protocol_number(value)
                    .map(Some)
                    .map_err(serde::de::Error::custom)
            }),
        Some(NumberOrString::String(value)) => Ok(Some(value.to_ascii_lowercase())),
    }
}

fn protocol_number(value: i64) -> Result<String, String> {
    match value {
        6 => Ok("tcp".into()),
        17 => Ok("udp".into()),
        value if value >= 0 => Ok(value.to_string()),
        value => Err(format!("invalid protocol number {value}")),
    }
}

fn parse_size(value: &str) -> Result<u64, String> {
    let value = value.trim();
    let split_at = value
        .find(|character: char| !character.is_ascii_digit() && character != '.')
        .unwrap_or(value.len());
    let (number, unit) = value.split_at(split_at);
    let number = number
        .parse::<f64>()
        .map_err(|error| format!("invalid size '{value}': {error}"))?;
    let multiplier = match unit.trim().to_ascii_uppercase().as_str() {
        "" | "B" => 1,
        "KB" | "KIB" => 1024,
        "MB" | "MIB" => 1024_u64.pow(2),
        "GB" | "GIB" => 1024_u64.pow(3),
        "TB" | "TIB" => 1024_u64.pow(4),
        unit => return Err(format!("unsupported size unit '{unit}'")),
    };
    Ok((number * multiplier as f64).round() as u64)
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PortsValue {
    Structured(Vec<Port>),
    Display(String),
}

fn deserialize_ports<'de, D>(deserializer: D) -> Result<Vec<Port>, D::Error>
where
    D: Deserializer<'de>,
{
    match PortsValue::deserialize(deserializer)? {
        PortsValue::Structured(ports) => Ok(ports),
        PortsValue::Display(display) => display
            .split(',')
            .map(str::trim)
            .filter(|port| !port.is_empty())
            .map(parse_port)
            .collect::<Result<Vec<_>, _>>()
            .map_err(serde::de::Error::custom),
    }
}

fn parse_port(value: &str) -> Result<Port, String> {
    let (mapping, protocol) = value
        .rsplit_once('/')
        .map(|(mapping, protocol)| (mapping, Some(protocol.to_string())))
        .unwrap_or((value, None));

    let (binding_address, host_port, container_port) =
        if let Some((host, container)) = mapping.rsplit_once("->") {
            let (binding_address, host_port) = parse_host_binding(host)?;
            (binding_address, host_port, parse_last_port(container)?)
        } else {
            (String::new(), None, parse_last_port(mapping)?)
        };

    Ok(Port {
        binding_address,
        host_port,
        container_port,
        protocol,
    })
}

fn parse_host_binding(value: &str) -> Result<(String, Option<u16>), String> {
    let value = value.trim();
    let (address, port) = if value.starts_with('[') {
        let close = value
            .find(']')
            .ok_or_else(|| format!("invalid binding address '{value}'"))?;
        let address = &value[1..close];
        let port = value[close + 1..].trim_start_matches(':');
        (address, port)
    } else if let Some((address, port)) = value.rsplit_once(':') {
        (address, port)
    } else {
        ("", value)
    };

    let port = port
        .parse::<u16>()
        .map_err(|error| format!("invalid host port '{value}': {error}"))?;
    Ok((address.to_string(), Some(port)))
}

fn parse_last_port(value: &str) -> Result<Option<u16>, String> {
    let port = value
        .trim()
        .rsplit(':')
        .next()
        .unwrap_or_default()
        .trim_matches(['[', ']']);
    if port.is_empty() {
        return Ok(None);
    }
    port.parse::<u16>()
        .map(Some)
        .map_err(|error| format!("invalid port '{value}': {error}"))
}

impl Container {
    pub fn state_label(&self) -> &str {
        match self.state {
            0 => "Created",
            1 => "Running",
            2 => "Running",
            3 => "Exited",
            4 => "Paused",
            5 => "Stopped",
            _ => "Unknown",
        }
    }

    pub fn is_running(&self) -> bool {
        self.state == 1 || self.state == 2
    }

    pub fn short_id(&self) -> &str {
        if self.id.len() > 12 {
            &self.id[..12]
        } else {
            &self.id
        }
    }
}

impl Image {
    pub fn display_name(&self) -> String {
        let repo = self.repository.as_deref().unwrap_or("<none>");
        let tag = self.tag.as_deref().unwrap_or("<none>");
        format!("{}:{}", repo, tag)
    }

    pub fn short_id(&self) -> &str {
        let id = self.id.strip_prefix("sha256:").unwrap_or(&self.id);
        if id.len() > 12 { &id[..12] } else { id }
    }

    pub fn human_size(&self) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;
        if self.size >= GB {
            format!("{:.1} GB", self.size as f64 / GB as f64)
        } else if self.size >= MB {
            format!("{:.1} MB", self.size as f64 / MB as f64)
        } else if self.size >= KB {
            format!("{:.1} KB", self.size as f64 / KB as f64)
        } else {
            format!("{} B", self.size)
        }
    }
}

/// Convert a unix timestamp to a compact relative time string (e.g. "2h", "3d", "1w")
pub fn relative_time(ts: i64) -> String {
    let now = chrono::Utc::now().timestamp();
    let diff = now - ts;
    if diff < 0 {
        return "now".into();
    }
    let secs = diff as u64;
    const MIN: u64 = 60;
    const HOUR: u64 = 3600;
    const DAY: u64 = 86400;
    const WEEK: u64 = 7 * DAY;
    const MONTH: u64 = 30 * DAY;
    const YEAR: u64 = 365 * DAY;
    if secs < MIN {
        format!("{}s", secs)
    } else if secs < HOUR {
        format!("{}m", secs / MIN)
    } else if secs < DAY {
        format!("{}h", secs / HOUR)
    } else if secs < WEEK {
        format!("{}d", secs / DAY)
    } else if secs < MONTH {
        format!("{}w", secs / WEEK)
    } else if secs < YEAR {
        format!("{}mo", secs / MONTH)
    } else {
        format!("{}y", secs / YEAR)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_current_container_output() {
        let json = r#"{
            "CreatedAt":"2026-09-24 13:43:19 -0400 EDT",
            "ID":"0db9fd023e25",
            "Image":"alpine:3.20",
            "Names":"example",
            "Ports":"127.0.0.1:8080->80/tcp, 443/tcp",
            "State":"created"
        }"#;

        let container: Container = serde_json::from_str(json).unwrap();

        assert_eq!(container.name, "example");
        assert_eq!(container.state, 0);
        assert_eq!(container.created_at, 1_790_271_799);
        assert_eq!(container.ports.len(), 2);
        assert_eq!(container.ports[0].binding_address, "127.0.0.1");
        assert_eq!(container.ports[0].host_port, Some(8080));
        assert_eq!(container.ports[0].container_port, Some(80));
        assert_eq!(container.ports[1].container_port, Some(443));
    }

    #[test]
    fn deserializes_current_image_output() {
        let json = r#"{
            "CreatedAt":"2026-04-16 19:53:26 -0400 EDT",
            "ID":"bf8527eb54c3",
            "Repository":"alpine",
            "Size":"7.81MB",
            "Tag":"3.20"
        }"#;

        let image: Image = serde_json::from_str(json).unwrap();

        assert_eq!(image.created, 1_776_383_606);
        assert_eq!(image.size, 8_189_379);
        assert_eq!(image.display_name(), "alpine:3.20");
    }

    #[test]
    fn keeps_legacy_container_output_compatible() {
        let json = r#"{
            "CreatedAt":1700000000,
            "ID":"abc",
            "Image":"alpine",
            "Name":"legacy",
            "Ports":[{"HostPort":8080,"ContainerPort":80,"Protocol":"tcp"}],
            "State":2,
            "StateChangedAt":1700000001
        }"#;

        let container: Container = serde_json::from_str(json).unwrap();

        assert_eq!(container.name, "legacy");
        assert!(container.is_running());
        assert_eq!(container.state_changed_at, 1_700_000_001);
    }

    #[test]
    fn deserializes_structured_numeric_port_protocols() {
        let json = r#"{
            "CreatedAt":1700000000,
            "ID":"abc",
            "Image":"alpine",
            "Name":"with-ports",
            "Ports":[{
                "BindingAddress":"0.0.0.0",
                "HostPort":8080,
                "ContainerPort":80,
                "Protocol":6
            }],
            "State":2,
            "StateChangedAt":1700000001
        }"#;

        let container: Container = serde_json::from_str(json).unwrap();

        assert_eq!(container.ports[0].binding_address, "0.0.0.0");
        assert_eq!(container.ports[0].protocol.as_deref(), Some("tcp"));
    }
}
