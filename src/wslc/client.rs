use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::time::{timeout, Duration};

const WSLC: &str = "wslc.exe";

/// Maximum bytes to read from stdout before truncating (1 MB).
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;

/// Timeout for wslc commands (10 seconds).
const CMD_TIMEOUT: Duration = Duration::from_secs(10);

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

/// Spawn a child process and read stdout up to `max_bytes`, with a timeout.
/// Kills the child if the timeout expires or the output limit is reached.
async fn run_bounded(args: &[&str], max_bytes: usize, time_limit: Duration) -> Result<(Vec<u8>, Vec<u8>, bool)> {
    let mut child = new_command(args)
        .spawn()
        .with_context(|| format!("Failed to spawn wslc.exe {}", args.join(" ")))?;

    let mut stdout = child.stdout.take().expect("stdout piped");
    let mut stderr = child.stderr.take().expect("stderr piped");

    let read_task = async {
        let mut stdout_buf = Vec::with_capacity(max_bytes.min(64 * 1024));
        let mut stderr_buf = Vec::with_capacity(4096);
        let mut tmp = [0u8; 8192];

        loop {
            tokio::select! {
                result = stdout.read(&mut tmp) => {
                    match result {
                        Ok(0) => break,
                        Ok(n) => {
                            let remaining = max_bytes.saturating_sub(stdout_buf.len());
                            let to_copy = n.min(remaining);
                            stdout_buf.extend_from_slice(&tmp[..to_copy]);
                            if stdout_buf.len() >= max_bytes {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
        }

        // Drain stderr (limited to 64KB)
        let mut stderr_tmp = [0u8; 4096];
        loop {
            match stderr.read(&mut stderr_tmp).await {
                Ok(0) => break,
                Ok(n) => {
                    if stderr_buf.len() < 65536 {
                        stderr_buf.extend_from_slice(&stderr_tmp[..n.min(65536 - stderr_buf.len())]);
                    }
                }
                Err(_) => break,
            }
        }

        (stdout_buf, stderr_buf)
    };

    let success = match timeout(time_limit, read_task).await {
        Ok((stdout_buf, stderr_buf)) => {
            // Reading finished within time limit; wait briefly for process exit
            let _ = timeout(Duration::from_secs(2), child.wait()).await;
            return Ok((stdout_buf, stderr_buf, true));
        }
        Err(_) => {
            // Timeout expired — kill the child
            let _ = child.kill().await;
            false
        }
    };

    Ok((Vec::new(), Vec::new(), success))
}

pub async fn run_wslc(args: &[&str]) -> Result<String> {
    let (stdout_buf, stderr_buf, success) = run_bounded(args, MAX_OUTPUT_BYTES, CMD_TIMEOUT).await?;

    if !success {
        anyhow::bail!("wslc {} timed out", args.join(" "));
    }

    // Check exit status by examining stderr — if stdout has content, command likely succeeded
    if stdout_buf.is_empty() && !stderr_buf.is_empty() {
        let stderr = String::from_utf8_lossy(&stderr_buf);
        anyhow::bail!("wslc {} failed: {}", args.join(" "), stderr.trim());
    }

    let raw = String::from_utf8_lossy(&stdout_buf).to_string();
    Ok(strip_copyright_header(&raw))
}

pub async fn run_wslc_allow_failure(args: &[&str]) -> Result<String> {
    let (stdout_buf, _stderr_buf, _success) = run_bounded(args, MAX_OUTPUT_BYTES, CMD_TIMEOUT).await?;

    let raw = String::from_utf8_lossy(&stdout_buf).to_string();
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
