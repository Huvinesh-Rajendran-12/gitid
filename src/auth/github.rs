use anyhow::{Context, Result, bail};
use std::process::{Command, Stdio};

/// Check if gh CLI is installed
fn is_gh_installed() -> bool {
    Command::new("gh")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Authenticate with GitHub CLI
pub fn authenticate(host: Option<&str>) -> Result<()> {
    if !is_gh_installed() {
        bail!("GitHub CLI (gh) is not installed. Install it from https://cli.github.com/");
    }

    let mut cmd = Command::new("gh");
    cmd.arg("auth").arg("login");

    // Add host flag for enterprise instances
    if let Some(h) = host {
        if h != "github.com" {
            cmd.arg("--hostname").arg(h);
        }
    }

    // Use SSH protocol
    cmd.arg("--git-protocol").arg("ssh");

    let status = cmd
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("Failed to run gh auth login")?;

    if !status.success() {
        bail!("GitHub authentication failed");
    }

    Ok(())
}
