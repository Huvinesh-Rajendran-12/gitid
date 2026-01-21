use crate::config::Config;
use crate::git::{self, ConfigScope};
use anyhow::Result;

/// Get the current profile name based on git config
pub fn get_current_profile(config: &Config) -> Result<Option<String>> {
    // First try local config
    let (name, email) = git::get_current_user(ConfigScope::Local)?;

    // If no local config, try global
    let (name, email) = if name.is_none() && email.is_none() {
        git::get_current_user(ConfigScope::Global)?
    } else {
        (name, email)
    };

    // Match against profiles
    if let (Some(name), Some(email)) = (name, email) {
        for (profile_name, profile) in &config.profiles {
            if profile.name == name && profile.email == email {
                return Ok(Some(profile_name.clone()));
            }
        }
    }

    Ok(None)
}

/// Output current profile for shell prompt (porcelain mode)
pub fn output_porcelain(config: &Config) -> Result<()> {
    if let Some(profile_name) = get_current_profile(config)? {
        println!("{}", profile_name);
    }
    Ok(())
}

/// Output current profile with formatting (human-readable mode)
pub fn output_human(config: &Config) -> Result<()> {
    if !git::is_git_repo() {
        println!("Not in a git repository");
        return Ok(());
    }

    if let Some(profile_name) = get_current_profile(config)? {
        if let Some(profile) = config.get_profile(&profile_name) {
            println!("Current profile: {}", profile_name);
            println!("  Name:  {}", profile.name);
            println!("  Email: {}", profile.email);
            println!("  Platform: {}", profile.platform);
        }
    } else {
        // Show git config even if no profile matches
        let (name, email) = git::get_current_user(ConfigScope::Local)?;
        let (global_name, global_email) = git::get_current_user(ConfigScope::Global)?;

        let name = name.or(global_name);
        let email = email.or(global_email);

        if name.is_some() || email.is_some() {
            println!("Current git identity (no matching profile):");
            if let Some(n) = name {
                println!("  Name:  {}", n);
            }
            if let Some(e) = email {
                println!("  Email: {}", e);
            }
        } else {
            println!("No git identity configured");
        }
    }

    Ok(())
}
