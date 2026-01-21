use crate::profile::Profile;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_profile: Option<String>,
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
}

impl Config {
    /// Load config from the default location (~/.config/gitid/config.toml)
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Config::default());
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))
    }

    /// Save config to the default location
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }

        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;

        fs::write(&path, content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))
    }

    /// Get the config file path
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not determine config directory")?;
        Ok(config_dir.join("gitid").join("config.toml"))
    }

    /// Add a profile to the config
    pub fn add_profile(&mut self, name: String, profile: Profile) -> Result<()> {
        profile.validate()?;
        self.profiles.insert(name, profile);
        Ok(())
    }

    /// Remove a profile from the config
    pub fn remove_profile(&mut self, name: &str) -> Option<Profile> {
        // If this was the default profile, clear it
        if self.default_profile.as_deref() == Some(name) {
            self.default_profile = None;
        }
        self.profiles.remove(name)
    }

    /// Get a profile by name
    pub fn get_profile(&self, name: &str) -> Option<&Profile> {
        self.profiles.get(name)
    }

    /// Check if a profile exists
    pub fn has_profile(&self, name: &str) -> bool {
        self.profiles.contains_key(name)
    }

    /// Get all profile names sorted alphabetically
    pub fn profile_names(&self) -> Vec<&String> {
        let mut names: Vec<_> = self.profiles.keys().collect();
        names.sort();
        names
    }

    /// Initialize config directory and create empty config if not exists
    pub fn init() -> Result<bool> {
        let path = Self::config_path()?;

        if path.exists() {
            return Ok(false);
        }

        let config = Config::default();
        config.save()?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::Platform;

    #[test]
    fn test_config_serialization() {
        let mut config = Config::default();
        config.default_profile = Some("personal".to_string());

        let profile = Profile::new(
            "John Doe".to_string(),
            "john@example.com".to_string(),
            Platform::Github,
            "~/.ssh/id_ed25519".to_string(),
            None,
            None,
        );
        config.profiles.insert("personal".to_string(), profile);

        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("default_profile"));
        assert!(toml_str.contains("[profiles.personal]"));
    }

    #[test]
    fn test_config_deserialization() {
        let toml_str = r#"
default_profile = "work"

[profiles.work]
name = "John Doe"
email = "john@company.com"
platform = "github"
ssh_key = "~/.ssh/id_work"
gpg_key = "ABCD1234"
"#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.default_profile, Some("work".to_string()));
        assert!(config.profiles.contains_key("work"));

        let profile = config.profiles.get("work").unwrap();
        assert_eq!(profile.name, "John Doe");
        assert_eq!(profile.gpg_key, Some("ABCD1234".to_string()));
    }
}
