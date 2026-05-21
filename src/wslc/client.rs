use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::process::Command;

const WSLC: &str = "wslc.exe";

pub async fn run_wslc(args: &[&str]) -> Result<String> {
    let output = Command::new(WSLC)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .with_context(|| format!("Failed to run wslc.exe {}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("wslc {} failed: {}", args.join(" "), stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub async fn run_wslc_allow_failure(args: &[&str]) -> Result<String> {
    let output = Command::new(WSLC)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .with_context(|| format!("Failed to run wslc.exe {}", args.join(" ")))?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
