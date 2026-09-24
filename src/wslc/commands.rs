use super::client::{run_wslc, run_wslc_allow_failure};
use super::types::*;
use anyhow::{Context, Result};
use serde::de::DeserializeOwned;

pub async fn list_containers() -> Result<Vec<Container>> {
    let output = run_wslc(&["list", "--all", "--format", "json"]).await?;
    parse_json_records(&output).context("Failed to parse container list")
}

pub async fn list_images() -> Result<Vec<Image>> {
    let output = run_wslc(&["images", "--format", "json"]).await?;
    parse_json_records(&output).context("Failed to parse image list")
}

pub async fn list_volumes() -> Result<Vec<Volume>> {
    let output = run_wslc(&["volume", "list", "--format", "json"]).await?;
    parse_json_records(&output).context("Failed to parse volume list")
}

fn parse_json_records<T: DeserializeOwned>(output: &str) -> Result<Vec<T>> {
    let trimmed = output.trim();
    if trimmed.is_empty() || trimmed == "[]" {
        return Ok(Vec::new());
    }

    if trimmed.starts_with('[') {
        return serde_json::from_str(trimmed).context("invalid JSON array");
    }

    trimmed
        .lines()
        .filter(|line| !line.trim().is_empty())
        .enumerate()
        .map(|(index, line)| {
            serde_json::from_str(line)
                .with_context(|| format!("invalid JSON object on line {}", index + 1))
        })
        .collect()
}

pub async fn inspect_object(id: &str) -> Result<String> {
    run_wslc(&["inspect", id]).await
}

pub async fn container_logs(id: &str, tail: u32) -> Result<String> {
    let tail_str = tail.to_string();
    run_wslc_allow_failure(&["logs", "--tail", &tail_str, id]).await
}

pub async fn container_stats(id: &str) -> Result<String> {
    run_wslc_allow_failure(&["stats", "--format", "json", id]).await
}

pub async fn start_container(id: &str) -> Result<String> {
    run_wslc(&["start", id]).await
}

pub async fn stop_container(id: &str) -> Result<String> {
    run_wslc(&["stop", id]).await
}

pub async fn kill_container(id: &str) -> Result<String> {
    run_wslc(&["kill", id]).await
}

pub async fn remove_container(id: &str) -> Result<String> {
    run_wslc(&["remove", id]).await
}

pub async fn remove_image(id: &str) -> Result<String> {
    run_wslc(&["rmi", id]).await
}

pub async fn remove_volume(name: &str) -> Result<String> {
    run_wslc(&["volume", "remove", name]).await
}

pub async fn pull_image(name: &str) -> Result<String> {
    run_wslc(&["pull", name]).await
}

pub async fn prune_images() -> Result<String> {
    run_wslc(&["image", "prune"]).await
}

/// Remove all stopped containers. Returns count of removed containers.
pub async fn prune_containers(stopped_ids: &[String]) -> Result<usize> {
    let mut removed = 0;
    for id in stopped_ids {
        if remove_container(id).await.is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_array_output() {
        let output = r#"[{"Driver":"guest","Name":"one"}]"#;
        let volumes: Vec<Volume> = parse_json_records(output).unwrap();
        assert_eq!(volumes.len(), 1);
        assert_eq!(volumes[0].name, "one");
    }

    #[test]
    fn parses_newline_delimited_json_output() {
        let output = "{\"Driver\":\"guest\",\"Name\":\"one\"}\n\
                      {\"Driver\":\"guest\",\"Name\":\"two\"}\n";
        let volumes: Vec<Volume> = parse_json_records(output).unwrap();
        assert_eq!(volumes.len(), 2);
        assert_eq!(volumes[1].name, "two");
    }
}
