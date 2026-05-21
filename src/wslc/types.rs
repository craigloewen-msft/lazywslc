use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Container {
    pub created_at: i64,
    pub id: String,
    pub image: String,
    pub name: String,
    pub ports: Vec<Port>,
    pub state: u8,
    pub state_changed_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Port {
    #[serde(default)]
    pub host_port: Option<u16>,
    #[serde(default)]
    pub container_port: Option<u16>,
    #[serde(default)]
    pub protocol: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Image {
    pub created: i64,
    pub id: String,
    pub repository: Option<String>,
    pub size: u64,
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
