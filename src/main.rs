mod auth;
mod cli;
mod config;
mod detect;
mod git;
mod profile;
mod prompt;
mod ssh;
mod ssh_keys;

use anyhow::{Context, Result, bail};
use clap::Parser;
use cli::{Cli, Commands};
use colored::Colorize;
use config::Config;
use git::ConfigScope;
use inquire::{Confirm, Select, Text};
use profile::{Platform, Profile};

fn main() {
    if let Err(e) = run() {
        eprintln!("{}: {}", "Error".red().bold(), e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => cmd_init(),
        Commands::Add {
            name,
            user_name,
            email,
            platform,
            ssh_key,
            gpg_key,
            host,
        } => cmd_add(name, user_name, email, platform, ssh_key, gpg_key, host),
        Commands::Remove {
            name,
            force,
            clean_ssh,
        } => cmd_remove(name, force, clean_ssh),
        Commands::List => cmd_list(),
        Commands::Use { name, global } => cmd_use(name, global),
        Commands::Auth { name } => cmd_auth(name),
        Commands::Current { porcelain } => cmd_current(porcelain),
        Commands::Detect { auto } => cmd_detect(auto),
        Commands::SshSync => cmd_ssh_sync(),
    }
}

fn cmd_init() -> Result<()> {
    let created = Config::init()?;
    let path = Config::config_path()?;

    if created {
        println!(
            "{} Created config at {}",
            "Success:".green().bold(),
            path.display()
        );
    } else {
        println!("Config already exists at {}", path.display());
    }

    Ok(())
}

fn cmd_add(
    name: Option<String>,
    user_name: Option<String>,
    email: Option<String>,
    platform: Option<String>,
    ssh_key: Option<String>,
    gpg_key: Option<String>,
    host: Option<String>,
) -> Result<()> {
    let mut config = Config::load()?;

    // Get profile name
    let name = match name {
        Some(n) => n,
        None => Text::new("Profile name:")
            .with_help_message("e.g., 'work', 'personal', 'client-acme'")
            .prompt()?,
    };

    if config.has_profile(&name) {
        bail!(
            "Profile '{}' already exists. Use a different name or remove it first.",
            name
        );
    }

    // Get user name
    let user_name = match user_name {
        Some(n) => n,
        None => Text::new("Git user name:")
            .with_help_message("This will be used for commit author")
            .prompt()?,
    };

    // Get email
    let email = match email {
        Some(e) => e,
        None => Text::new("Git email:")
            .with_help_message("This will be used for commit author")
            .prompt()?,
    };

    // Get platform
    let platform: Platform = match platform {
        Some(p) => p.parse()?,
        None => {
            let options = vec!["github", "gitlab", "both"];
            let selection = Select::new("Platform:", options)
                .with_help_message("Select the Git hosting platform")
                .prompt()?;
            selection.parse()?
        }
    };

    // Get SSH key
    let ssh_key = match ssh_key {
        Some(k) => k,
        None => select_or_create_ssh_key(&name, &email)?,
    };

    // Get GPG key (optional)
    let gpg_key = match gpg_key {
        Some(k) => Some(k),
        None => {
            let input = Text::new("GPG signing key (optional):")
                .with_help_message("Press Enter to skip")
                .prompt()?;
            if input.is_empty() {
                None
            } else {
                Some(input)
            }
        }
    };

    // Get custom host (optional)
    let host = match host {
        Some(h) => Some(h),
        None => {
            let needs_custom = Confirm::new("Use custom host?")
                .with_help_message("For GitHub Enterprise or self-hosted GitLab")
                .with_default(false)
                .prompt()?;

            if needs_custom {
                let h = Text::new("Custom host:")
                    .with_help_message("e.g., 'github.company.com' or 'gitlab.myorg.com'")
                    .prompt()?;
                if h.is_empty() {
                    None
                } else {
                    Some(h)
                }
            } else {
                None
            }
        }
    };

    let profile = Profile::new(user_name, email, platform, ssh_key, gpg_key, host);
    profile.validate()?;

    config.add_profile(name.clone(), profile)?;
    config.save()?;

    println!();
    println!(
        "{} Added profile '{}'",
        "Success:".green().bold(),
        name.cyan()
    );
    println!("Run {} to sync SSH config", "gitid ssh-sync".yellow());

    Ok(())
}

/// Interactive SSH key selection or creation
fn select_or_create_ssh_key(profile_name: &str, email: &str) -> Result<String> {
    let existing_keys = ssh_keys::discover_keys()?;

    // Build options list
    let mut options: Vec<String> = existing_keys
        .iter()
        .map(|k| format!("{} ({})", k.path_display(), k.key_type))
        .collect();

    options.push("+ Generate new SSH key".to_string());
    options.push("+ Enter path manually".to_string());

    let selection = Select::new("SSH key:", options.clone())
        .with_help_message("Select an existing key or create a new one")
        .prompt()?;

    if selection == "+ Generate new SSH key" {
        // Generate a new key
        println!("Generating new ed25519 SSH key...");
        let key = ssh_keys::generate_key(profile_name, email)?;

        println!(
            "{} Generated SSH key: {}",
            "Success:".green().bold(),
            key.path_display()
        );

        // Show the public key
        let public_key = ssh_keys::read_public_key(&key)?;
        println!();
        println!("{}", "Public key (add this to GitHub/GitLab):".yellow());
        println!("{}", public_key.trim());
        println!();

        Ok(key.path_display())
    } else if selection == "+ Enter path manually" {
        let default_path = format!("~/.ssh/id_ed25519_{}", profile_name);
        let path = Text::new("SSH key path:")
            .with_default(&default_path)
            .prompt()?;
        Ok(path)
    } else {
        // Find the selected key
        let idx = options.iter().position(|o| o == &selection).unwrap();
        Ok(existing_keys[idx].path_display())
    }
}

fn cmd_remove(name: Option<String>, force: bool, clean_ssh: bool) -> Result<()> {
    let mut config = Config::load()?;

    if config.profiles.is_empty() {
        bail!("No profiles configured");
    }

    // Get profile name (interactive if not provided)
    let name = match name {
        Some(n) => n,
        None => {
            let profiles: Vec<String> = config.profile_names().into_iter().cloned().collect();
            Select::new("Select profile to remove:", profiles)
                .prompt()?
        }
    };

    if !config.has_profile(&name) {
        bail!("Profile '{}' not found", name);
    }

    if !force {
        let confirmed = Confirm::new(&format!("Remove profile '{}'?", name))
            .with_default(false)
            .prompt()?;

        if !confirmed {
            println!("Cancelled");
            return Ok(());
        }
    }

    config.remove_profile(&name);
    config.save()?;

    println!("{} Removed profile '{}'", "Success:".green().bold(), name);

    if clean_ssh {
        ssh::sync_ssh_config(&config)?;
        println!("SSH config updated");
    }

    Ok(())
}

fn cmd_list() -> Result<()> {
    let config = Config::load()?;

    if config.profiles.is_empty() {
        println!("No profiles configured");
        println!("Run {} to add a profile", "gitid add".yellow());
        return Ok(());
    }

    // Get current profile if in a git repo
    let current = if git::is_git_repo() {
        prompt::get_current_profile(&config)?
    } else {
        None
    };

    println!("{}", "Profiles:".bold());
    println!();

    for name in config.profile_names() {
        if let Some(profile) = config.get_profile(name) {
            let is_current = current.as_ref() == Some(name);
            let marker = if is_current {
                "*".green().bold().to_string()
            } else {
                " ".to_string()
            };

            let default_marker = if config.default_profile.as_ref() == Some(name) {
                " (default)".dimmed().to_string()
            } else {
                String::new()
            };

            println!("{} {}{}", marker, name.cyan().bold(), default_marker);
            println!("    Name:     {}", profile.name);
            println!("    Email:    {}", profile.email);
            println!("    Platform: {}", profile.platform);
            println!("    SSH Key:  {}", profile.ssh_key);

            if let Some(ref gpg) = profile.gpg_key {
                println!("    GPG Key:  {}", gpg);
            }
            if let Some(ref host) = profile.host {
                println!("    Host:     {}", host);
            }
            println!();
        }
    }

    Ok(())
}

fn cmd_use(name: Option<String>, global: bool) -> Result<()> {
    let config = Config::load()?;

    if config.profiles.is_empty() {
        bail!("No profiles configured. Run 'gitid add' first.");
    }

    // Get profile name (interactive if not provided)
    let name = match name {
        Some(n) => n,
        None => {
            let profiles: Vec<String> = config.profile_names().into_iter().cloned().collect();
            Select::new("Select profile:", profiles)
                .with_help_message("Use arrow keys to navigate, Enter to select")
                .prompt()?
        }
    };

    let profile = config
        .get_profile(&name)
        .context(format!("Profile '{}' not found", name))?;

    let scope = if global {
        ConfigScope::Global
    } else {
        if !git::is_git_repo() {
            bail!("Not in a git repository. Use --global to set globally.");
        }
        ConfigScope::Local
    };

    // Apply git configuration
    git::apply_profile(
        &profile.name,
        &profile.email,
        profile.gpg_key.as_deref(),
        scope,
    )?;

    let scope_str = if global { "globally" } else { "locally" };
    println!(
        "{} Switched to profile '{}' {}",
        "Success:".green().bold(),
        name.cyan(),
        scope_str
    );
    println!("  Name:  {}", profile.name);
    println!("  Email: {}", profile.email);

    if profile.gpg_key.is_some() {
        println!("  GPG signing: enabled");
    }

    Ok(())
}

fn cmd_auth(name: Option<String>) -> Result<()> {
    let config = Config::load()?;

    if config.profiles.is_empty() {
        bail!("No profiles configured. Run 'gitid add' first.");
    }

    // Get profile name (interactive if not provided)
    let name = match name {
        Some(n) => n,
        None => {
            let profiles: Vec<String> = config.profile_names().into_iter().cloned().collect();
            Select::new("Select profile to authenticate:", profiles).prompt()?
        }
    };

    let profile = config
        .get_profile(&name)
        .context(format!("Profile '{}' not found", name))?;

    println!(
        "Authenticating CLI tools for profile '{}'...",
        name.cyan()
    );
    println!();

    auth::authenticate(&name, profile)?;

    println!();
    println!(
        "{} Authentication complete for '{}'",
        "Success:".green().bold(),
        name
    );

    Ok(())
}

fn cmd_current(porcelain: bool) -> Result<()> {
    let config = Config::load()?;

    if porcelain {
        prompt::output_porcelain(&config)
    } else {
        prompt::output_human(&config)
    }
}

fn cmd_detect(auto: bool) -> Result<()> {
    if !git::is_git_repo() {
        bail!("Not in a git repository");
    }

    let config = Config::load()?;

    match detect::detect_and_suggest(&config)? {
        Some((profile_name, reason)) => {
            println!(
                "{} Detected profile: {}",
                "Match:".green().bold(),
                profile_name.cyan().bold()
            );
            println!("  Reason: {}", reason);

            if auto {
                // Auto-apply
                if let Some(profile) = config.get_profile(&profile_name) {
                    git::apply_profile(
                        &profile.name,
                        &profile.email,
                        profile.gpg_key.as_deref(),
                        ConfigScope::Local,
                    )?;
                    println!();
                    println!(
                        "{} Applied profile '{}'",
                        "Success:".green().bold(),
                        profile_name
                    );
                }
            } else {
                // Ask for confirmation using inquire
                let confirmed = Confirm::new("Apply this profile?")
                    .with_default(true)
                    .prompt()?;

                if confirmed {
                    if let Some(profile) = config.get_profile(&profile_name) {
                        git::apply_profile(
                            &profile.name,
                            &profile.email,
                            profile.gpg_key.as_deref(),
                            ConfigScope::Local,
                        )?;
                        println!(
                            "{} Applied profile '{}'",
                            "Success:".green().bold(),
                            profile_name
                        );
                    }
                } else {
                    println!("Cancelled");
                }
            }
        }
        None => {
            println!("No matching profile detected for this repository");

            // Show remote info for debugging
            if let Some(url) = git::get_remote_url("origin")? {
                println!("  Remote origin: {}", url);
            }

            if !config.profiles.is_empty() {
                println!();
                let apply_manually = Confirm::new("Would you like to select a profile manually?")
                    .with_default(true)
                    .prompt()?;

                if apply_manually {
                    let profiles: Vec<String> =
                        config.profile_names().into_iter().cloned().collect();
                    let name = Select::new("Select profile:", profiles).prompt()?;

                    if let Some(profile) = config.get_profile(&name) {
                        git::apply_profile(
                            &profile.name,
                            &profile.email,
                            profile.gpg_key.as_deref(),
                            ConfigScope::Local,
                        )?;
                        println!(
                            "{} Applied profile '{}'",
                            "Success:".green().bold(),
                            name
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

fn cmd_ssh_sync() -> Result<()> {
    let config = Config::load()?;

    if config.profiles.is_empty() {
        println!("No profiles to sync");
        return Ok(());
    }

    let (count, was_update) = ssh::sync_ssh_config(&config)?;

    let action = if was_update { "Updated" } else { "Added" };
    println!(
        "{} {} SSH config with {} profile(s)",
        "Success:".green().bold(),
        action,
        count
    );

    let path = ssh::ssh_config_path()?;
    println!("  File: {}", path.display());

    // Show the generated aliases
    println!();
    println!("SSH Host aliases:");
    for name in config.profile_names() {
        if let Some(profile) = config.get_profile(name) {
            let alias = profile.ssh_host_alias(name);
            println!("  {} -> {}", alias.cyan(), profile.default_host());
        }
    }

    Ok(())
}
