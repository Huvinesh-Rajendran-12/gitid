pub mod github;
pub mod gitlab;

use crate::profile::{Platform, Profile};
use anyhow::Result;

/// Authenticate CLI tools for a profile based on its platform
pub fn authenticate(_profile_name: &str, profile: &Profile) -> Result<()> {
    let host = profile.host.as_deref();

    match profile.platform {
        Platform::Github => {
            github::authenticate(host)?;
        }
        Platform::Gitlab => {
            gitlab::authenticate(host)?;
        }
        Platform::Both => {
            println!("Authenticating GitHub...");
            github::authenticate(host)?;
            println!("\nAuthenticating GitLab...");
            gitlab::authenticate(host)?;
        }
    }

    Ok(())
}
