use anyhow::{Context, Result};
use std::{env, process::Stdio};
use tokio::io::AsyncReadExt;
use tokio::time::{timeout, Duration};

const WSLC: &str = "wslc.exe";
const WSL: &str = "wsl.exe";

/// Maximum bytes to read from stdout before truncating (1 MB).
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;

/// Timeout for wslc commands (10 seconds).
const CMD_TIMEOUT: Duration = Duration::from_secs(10);

/// Starting through wslc-remote may need to recreate an NFS server and remount
/// the runtime backing store after WSL/runtime shutdown.
const REMOTE_START_TIMEOUT: Duration = Duration::from_secs(60);

fn wslc_remote_disabled() -> bool {
    env::var("LAZYWSLC_REMOTE")
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "off" | "no"
            )
        })
        .unwrap_or(false)
}

fn new_wsl_command() -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(WSL);

    if let Ok(distro) = env::var("LAZYWSLC_REMOTE_DISTRO") {
        let distro = distro.trim();
        if !distro.is_empty() {
            cmd.arg("-d").arg(distro);
        }
    }

    cmd
}

fn remote_distro_configured() -> bool {
    env::var("LAZYWSLC_REMOTE_DISTRO")
        .ok()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

/// Detect wslc-remote in the selected/default WSL distro.
///
/// Bare `wslc` is intentional: Microsoft's native binary is `wslc.exe`, while
/// wslc-remote installs the Linux wrapper as `wslc`.
///
/// An explicitly configured distro is an opt-in to wslc-remote. Do not
/// cold-start that distro merely to probe it before the real operation.
///
/// In auto-detect mode, a timeout is not evidence that wslc-remote is absent.
/// Falling back to native `wslc.exe` after a probe timeout could bypass NFS
/// recovery/teardown, so timeout and spawn errors are propagated instead.
async fn wslc_remote_available() -> Result<bool> {
    if wslc_remote_disabled() {
        return Ok(false);
    }

    if remote_distro_configured() {
        return Ok(true);
    }

    let mut cmd = new_wsl_command();
    cmd.args([
        "--",
        "bash",
        "-lc",
        "command -v wslc >/dev/null 2>&1 && wslc _help >/dev/null 2>&1",
    ])
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null());

    match timeout(Duration::from_secs(30), cmd.status()).await {
        Ok(Ok(status)) => Ok(status.success()),
        Ok(Err(err)) => Err(err).context("Failed to probe wslc-remote through WSL"),
        Err(_) => anyhow::bail!(
            "Timed out waiting for WSL while probing wslc-remote; refusing native fallback"
        ),
    }
}
/// Quote one argument for a POSIX shell command.
///
/// `wsl.exe` does not preserve the extra positional arguments after `bash -lc`
/// when it is launched through Windows `CreateProcess`/Rust `Command` the same
/// way a native Unix exec would. Build a single shell command instead, while
/// quoting every dynamic argument so container/volume names cannot become shell
/// syntax.
fn shell_quote(arg: &str) -> String {
    format!("'{}'", arg.replace('\'', "'\\''"))
}
/// Run a lifecycle-sensitive command through wslc-remote when it is installed
/// in WSL. Today this is used by container start and volume remove.
///
/// Fallback to native wslc.exe happens only when the wrapper is unavailable.
/// If wslc-remote exists but the operation fails, propagate that failure rather
/// than bypassing NFS recovery/teardown with a native command.
pub async fn run_wslc_remote_aware(args: &[&str]) -> Result<String> {
    if !wslc_remote_available().await? {
        return run_wslc(args).await;
    }

    let mut cmd = new_wsl_command();
    // `wsl.exe` launched through Windows CreateProcess does not reliably pass
    // the arguments after the `bash -lc` command string through as Bash's
    // positional parameters. Put the full operation in the command string and
    // shell-quote each dynamic argument instead.
    let mut shell_command = String::from("exec wslc");
    for arg in args {
        shell_command.push(' ');
        shell_command.push_str(&shell_quote(arg));
    }

    cmd.arg("--")
        .arg("bash")
        .arg("-lc")
        .arg(shell_command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        cmd.as_std_mut().creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let display = args.join(" ");
    let output = timeout(REMOTE_START_TIMEOUT, cmd.output())
        .await
        .with_context(|| format!("wslc-remote {display} timed out"))?
        .with_context(|| format!("Failed to run wslc-remote {display}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        let detail = if stderr.trim().is_empty() {
            stdout.trim()
        } else {
            stderr.trim()
        };
        anyhow::bail!("wslc-remote {display} failed: {detail}");
    }

    Ok(strip_copyright_header(&stdout))
}

/// Open an interactive shell in a running container.
///
/// This mirrors lazydocker's exec-shell behavior: ask /etc/passwd for the
/// current user's configured shell, using /bin/sh only as the bootstrap.
pub fn exec_shell(id: &str) -> Result<()> {
    let status = std::process::Command::new(WSLC)
        .args([
            "exec",
            "-i",
            "-t",
            id,
            "/bin/sh",
            "-c",
            "eval $(grep ^$(id -un): /etc/passwd | cut -d : -f 7-)",
        ])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("Failed to spawn wslc.exe exec for container {id}"))?;

    if !status.success() {
        anyhow::bail!("wslc exec shell exited with status {status}");
    }

    Ok(())
}

fn new_command(args: &[&str]) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(WSLC);
    cmd.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Prevent console window flash on Windows

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
            // Timeout expired â€” kill the child
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

    // Check exit status by examining stderr â€” if stdout has content, command likely succeeded
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
