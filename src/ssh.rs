use crate::config::Config;
use crate::profile::{Platform, Profile};
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

const MANAGED_START: &str = "# === GITID MANAGED START ===";
const MANAGED_END: &str = "# === GITID MANAGED END ===";

/// Get the SSH config file path
pub fn ssh_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".ssh").join("config"))
}

/// Generate SSH Host entry for a profile
fn generate_host_entry(profile_name: &str, profile: &Profile) -> String {
    let alias = profile.ssh_host_alias(profile_name);
    let hostname = profile.default_host();
    let ssh_key = &profile.ssh_key;

    let mut entry = format!(
        "Host {}\n  HostName {}\n  User git\n  IdentityFile {}\n  IdentitiesOnly yes\n",
        alias, hostname, ssh_key
    );

    // For 'both' platform, generate entries for both GitHub and GitLab
    if matches!(profile.platform, Platform::Both) {
        // Also generate specific aliases for github and gitlab
        let github_alias = format!("github-{}", profile_name);
        let gitlab_alias = format!("gitlab-{}", profile_name);

        entry.push_str(&format!(
            "\nHost {}\n  HostName github.com\n  User git\n  IdentityFile {}\n  IdentitiesOnly yes\n",
            github_alias, ssh_key
        ));
        entry.push_str(&format!(
            "\nHost {}\n  HostName gitlab.com\n  User git\n  IdentityFile {}\n  IdentitiesOnly yes\n",
            gitlab_alias, ssh_key
        ));
    }

    entry
}

/// Generate the managed block content for all profiles
pub fn generate_managed_block(config: &Config) -> String {
    let mut block = String::new();
    block.push_str(MANAGED_START);
    block.push('\n');

    let mut profile_names: Vec<_> = config.profiles.keys().collect();
    profile_names.sort();

    for name in profile_names {
        if let Some(profile) = config.profiles.get(name) {
            block.push_str(&generate_host_entry(name, profile));
        }
    }

    block.push_str(MANAGED_END);
    block
}

/// Read the current SSH config content
fn read_ssh_config() -> Result<String> {
    let path = ssh_config_path()?;
    if path.exists() {
        fs::read_to_string(&path)
            .with_context(|| format!("Failed to read SSH config: {}", path.display()))
    } else {
        Ok(String::new())
    }
}

/// Write the SSH config content
fn write_ssh_config(content: &str) -> Result<()> {
    let path = ssh_config_path()?;

    // Ensure .ssh directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create .ssh directory: {}", parent.display()))?;
    }

    fs::write(&path, content)
        .with_context(|| format!("Failed to write SSH config: {}", path.display()))
}

/// Sync SSH config with all profiles
/// Returns (added_count, updated)
pub fn sync_ssh_config(config: &Config) -> Result<(usize, bool)> {
    let current_content = read_ssh_config()?;
    let new_block = generate_managed_block(config);

    let profile_count = config.profiles.len();

    // Check if managed block exists
    if let (Some(start_idx), Some(end_idx)) = (
        current_content.find(MANAGED_START),
        current_content.find(MANAGED_END),
    ) {
        // Replace existing managed block
        let end_idx = end_idx + MANAGED_END.len();
        let mut new_content = String::new();
        new_content.push_str(&current_content[..start_idx]);
        new_content.push_str(&new_block);

        // Preserve any content after the managed block
        if end_idx < current_content.len() {
            new_content.push_str(&current_content[end_idx..]);
        }

        write_ssh_config(&new_content)?;
        Ok((profile_count, true))
    } else {
        // Append new managed block
        let mut new_content = current_content;

        // Add newlines if the file doesn't end with one
        if !new_content.is_empty() && !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        if !new_content.is_empty() {
            new_content.push('\n');
        }

        new_content.push_str(&new_block);
        new_content.push('\n');

        write_ssh_config(&new_content)?;
        Ok((profile_count, false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_host_entry() {
        let profile = Profile::new(
            "John Doe".to_string(),
            "john@example.com".to_string(),
            Platform::Github,
            "~/.ssh/id_ed25519_work".to_string(),
            None,
            None,
        );

        let entry = generate_host_entry("work", &profile);
        assert!(entry.contains("Host github-work"));
        assert!(entry.contains("HostName github.com"));
        assert!(entry.contains("IdentityFile ~/.ssh/id_ed25519_work"));
    }

    #[test]
    fn test_generate_managed_block() {
        let mut config = Config::default();
        config.profiles.insert(
            "work".to_string(),
            Profile::new(
                "John Doe".to_string(),
                "john@company.com".to_string(),
                Platform::Github,
                "~/.ssh/id_work".to_string(),
                None,
                None,
            ),
        );

        let block = generate_managed_block(&config);
        assert!(block.starts_with(MANAGED_START));
        assert!(block.ends_with(MANAGED_END));
        assert!(block.contains("Host github-work"));
    }
}
