use anyhow::{Context, Result, bail};
use std::process::Command;

/// Scope for git config operations
#[derive(Debug, Clone, Copy)]
pub enum ConfigScope {
    Local,
    Global,
}

impl ConfigScope {
    fn flag(&self) -> &str {
        match self {
            ConfigScope::Local => "--local",
            ConfigScope::Global => "--global",
        }
    }
}

/// Get a git config value
pub fn get_config(key: &str, scope: ConfigScope) -> Result<Option<String>> {
    let output = Command::new("git")
        .args(["config", scope.flag(), "--get", key])
        .output()
        .context("Failed to execute git config")?;

    if output.status.success() {
        let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(Some(value))
    } else {
        Ok(None)
    }
}

/// Set a git config value
pub fn set_config(key: &str, value: &str, scope: ConfigScope) -> Result<()> {
    let status = Command::new("git")
        .args(["config", scope.flag(), key, value])
        .status()
        .context("Failed to execute git config")?;

    if !status.success() {
        bail!("Failed to set git config {} = {}", key, value);
    }
    Ok(())
}

/// Unset a git config value
pub fn unset_config(key: &str, scope: ConfigScope) -> Result<()> {
    Command::new("git")
        .args(["config", scope.flag(), "--unset", key])
        .status()
        .context("Failed to execute git config")?;

    // Don't fail if the key doesn't exist
    Ok(())
}

/// Check if we're inside a git repository
pub fn is_git_repo() -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get the remote URL for a given remote name
pub fn get_remote_url(remote: &str) -> Result<Option<String>> {
    let output = Command::new("git")
        .args(["remote", "get-url", remote])
        .output()
        .context("Failed to execute git remote")?;

    if output.status.success() {
        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(Some(url))
    } else {
        Ok(None)
    }
}

/// List all remotes in the repository
pub fn list_remotes() -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["remote"])
        .output()
        .context("Failed to execute git remote")?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let remotes = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok(remotes)
}

/// Parsed remote URL information
#[derive(Debug, Clone)]
pub struct RemoteUrl {
    pub host: String,
}

impl RemoteUrl {
    /// Parse a git remote URL (SSH or HTTPS)
    pub fn parse(url: &str) -> Option<Self> {
        // SSH format: git@github.com:owner/repo.git
        // SSH with alias: git@github-work:owner/repo.git
        if url.starts_with("git@") {
            let without_prefix = url.strip_prefix("git@")?;
            let (host, _path) = without_prefix.split_once(':')?;
            return Some(RemoteUrl {
                host: host.to_string(),
            });
        }

        // HTTPS format: https://github.com/owner/repo.git
        if url.starts_with("https://") || url.starts_with("http://") {
            let without_scheme = url
                .strip_prefix("https://")
                .or_else(|| url.strip_prefix("http://"))?;

            let parts: Vec<&str> = without_scheme.splitn(2, '/').collect();
            if parts.is_empty() {
                return None;
            }

            return Some(RemoteUrl {
                host: parts[0].to_string(),
            });
        }

        None
    }
}

/// Apply a profile's git configuration
pub fn apply_profile(
    name: &str,
    email: &str,
    gpg_key: Option<&str>,
    scope: ConfigScope,
) -> Result<()> {
    set_config("user.name", name, scope)?;
    set_config("user.email", email, scope)?;

    if let Some(key) = gpg_key {
        set_config("user.signingkey", key, scope)?;
        set_config("commit.gpgsign", "true", scope)?;
    } else {
        // Remove GPG settings if no key is specified
        unset_config("user.signingkey", scope)?;
        unset_config("commit.gpgsign", scope)?;
    }

    Ok(())
}

/// Get current git user configuration
pub fn get_current_user(scope: ConfigScope) -> Result<(Option<String>, Option<String>)> {
    let name = get_config("user.name", scope)?;
    let email = get_config("user.email", scope)?;
    Ok((name, email))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ssh_url() {
        let url = RemoteUrl::parse("git@github.com:owner/repo.git").unwrap();
        assert_eq!(url.host, "github.com");
    }

    #[test]
    fn test_parse_https_url() {
        let url = RemoteUrl::parse("https://github.com/owner/repo.git").unwrap();
        assert_eq!(url.host, "github.com");
    }

    #[test]
    fn test_parse_ssh_url_with_alias() {
        let url = RemoteUrl::parse("git@github-work:company/project.git").unwrap();
        assert_eq!(url.host, "github-work");
    }
}
