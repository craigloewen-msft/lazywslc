use anyhow::{Context, Result};
use std::process::Stdio;

const WSLC: &str = "wslc.exe";

fn new_command(args: &[&str]) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(WSLC);
    cmd.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Prevent console window flash on Windows
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        cmd.as_std_mut().creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    cmd
}

pub async fn run_wslc(args: &[&str]) -> Result<String> {
    let output = new_command(args)
        .output()
        .await
        .with_context(|| format!("Failed to run wslc.exe {}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("wslc {} failed: {}", args.join(" "), stderr.trim());
    }

    let raw = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(strip_copyright_header(&raw))
}

pub async fn run_wslc_allow_failure(args: &[&str]) -> Result<String> {
    let output = new_command(args)
        .output()
        .await
        .with_context(|| format!("Failed to run wslc.exe {}", args.join(" ")))?;

    let raw = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(strip_copyright_header(&raw))
}

/// wslc.exe sometimes prints a copyright header to stdout; strip it.
fn strip_copyright_header(s: &str) -> String {
    let mut lines: Vec<&str> = s.lines().collect();

    while let Some(first) = lines.first() {
        let trimmed = first.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("Copyright")
            || trimmed.starts_with("For privacy")
        {
            lines.remove(0);
        } else {
            break;
        }
    }

    lines.join("\n")
}
