use anyhow::Result;
use super::client::{run_wslc, run_wslc_allow_failure};
use super::types::*;

pub async fn list_containers() -> Result<Vec<Container>> {
    let output = run_wslc(&["list", "--all", "--format", "json"]).await?;
    let trimmed = output.trim();
    if trimmed.is_empty() || trimmed == "[]" {
        return Ok(Vec::new());
    }
    let containers: Vec<Container> = serde_json::from_str(trimmed)?;
    Ok(containers)
}

pub async fn list_images() -> Result<Vec<Image>> {
    let output = run_wslc(&["images", "--format", "json"]).await?;
    let trimmed = output.trim();
    if trimmed.is_empty() || trimmed == "[]" {
        return Ok(Vec::new());
    }
    let images: Vec<Image> = serde_json::from_str(trimmed)?;
    Ok(images)
}

pub async fn list_volumes() -> Result<Vec<Volume>> {
    let output = run_wslc(&["volume", "list", "--format", "json"]).await?;
    let trimmed = output.trim();
    if trimmed.is_empty() || trimmed == "[]" {
        return Ok(Vec::new());
    }
    let volumes: Vec<Volume> = serde_json::from_str(trimmed)?;
    Ok(volumes)
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
