use anyhow::{Context, Result, bail};
use std::process::{Command, Stdio};

/// Check if glab CLI is installed
fn is_glab_installed() -> bool {
    Command::new("glab")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Authenticate with GitLab CLI
pub fn authenticate(host: Option<&str>) -> Result<()> {
    if !is_glab_installed() {
        bail!("GitLab CLI (glab) is not installed. Install it from https://gitlab.com/gitlab-org/cli");
    }

    let mut cmd = Command::new("glab");
    cmd.arg("auth").arg("login");

    // Add hostname flag for self-hosted instances
    if let Some(h) = host {
        if h != "gitlab.com" {
            cmd.arg("--hostname").arg(h);
        }
    }

    let status = cmd
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("Failed to run glab auth login")?;

    if !status.success() {
        bail!("GitLab authentication failed");
    }

    Ok(())
}
