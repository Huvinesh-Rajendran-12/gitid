use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Github,
    Gitlab,
    Both,
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::Github => write!(f, "github"),
            Platform::Gitlab => write!(f, "gitlab"),
            Platform::Both => write!(f, "both"),
        }
    }
}

impl std::str::FromStr for Platform {
    type Err = ProfileError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "github" => Ok(Platform::Github),
            "gitlab" => Ok(Platform::Gitlab),
            "both" => Ok(Platform::Both),
            _ => Err(ProfileError::InvalidPlatform(s.to_string())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub email: String,
    pub platform: Platform,
    pub ssh_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpg_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
}

#[derive(Debug, Error)]
pub enum ProfileError {
    #[error("Invalid platform: {0}. Must be 'github', 'gitlab', or 'both'")]
    InvalidPlatform(String),
    #[error("Profile name cannot be empty")]
    EmptyName,
    #[error("Email cannot be empty")]
    EmptyEmail,
    #[error("SSH key path cannot be empty")]
    EmptySshKey,
}

impl Profile {
    pub fn new(
        name: String,
        email: String,
        platform: Platform,
        ssh_key: String,
        gpg_key: Option<String>,
        host: Option<String>,
    ) -> Self {
        Self {
            name,
            email,
            platform,
            ssh_key,
            gpg_key,
            host,
        }
    }

    pub fn validate(&self) -> Result<(), ProfileError> {
        if self.name.trim().is_empty() {
            return Err(ProfileError::EmptyName);
        }
        if self.email.trim().is_empty() {
            return Err(ProfileError::EmptyEmail);
        }
        if self.ssh_key.trim().is_empty() {
            return Err(ProfileError::EmptySshKey);
        }
        Ok(())
    }

    pub fn default_host(&self) -> &str {
        if let Some(ref host) = self.host {
            host.as_str()
        } else {
            match self.platform {
                Platform::Github | Platform::Both => "github.com",
                Platform::Gitlab => "gitlab.com",
            }
        }
    }

    /// Generate SSH Host alias for this profile (e.g., "github-work")
    pub fn ssh_host_alias(&self, profile_name: &str) -> String {
        let platform_prefix = match self.platform {
            Platform::Github => "github",
            Platform::Gitlab => "gitlab",
            Platform::Both => "git",
        };
        format!("{}-{}", platform_prefix, profile_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_parsing() {
        assert_eq!("github".parse::<Platform>().unwrap(), Platform::Github);
        assert_eq!("gitlab".parse::<Platform>().unwrap(), Platform::Gitlab);
        assert_eq!("both".parse::<Platform>().unwrap(), Platform::Both);
        assert_eq!("GITHUB".parse::<Platform>().unwrap(), Platform::Github);
    }

    #[test]
    fn test_profile_validation() {
        let profile = Profile::new(
            "John Doe".to_string(),
            "john@example.com".to_string(),
            Platform::Github,
            "~/.ssh/id_ed25519".to_string(),
            None,
            None,
        );
        assert!(profile.validate().is_ok());
    }

    #[test]
    fn test_profile_validation_empty_name() {
        let profile = Profile::new(
            "".to_string(),
            "john@example.com".to_string(),
            Platform::Github,
            "~/.ssh/id_ed25519".to_string(),
            None,
            None,
        );
        assert!(matches!(profile.validate(), Err(ProfileError::EmptyName)));
    }
}
