mod auth;
mod cli;
mod config;
mod detect;
mod git;
mod profile;
mod prompt;
#[allow(dead_code)]
mod scm;
mod ssh;
mod ssh_keys;

use anyhow::{bail, Context, Result};
use clap::Parser;
use cli::{
    AuthCommands, Cli, Commands, ScmCiCommands, ScmCommands, ScmIssueCommands, ScmReviewCommands,
};
use colored::Colorize;
use config::Config;
use git::ConfigScope;
use inquire::{Confirm, Select, Text};
use profile::{Platform, Profile};
use scm::detect::detect_from_repo_origin;
use scm::github::GitHubProvider;
use scm::gitlab::GitLabProvider;
use scm::provider::{ScmError, ScmProvider};
use scm::types::ProviderKind;

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
        Commands::Auth { command } => match command {
            AuthCommands::Login { name } => cmd_auth(name),
            AuthCommands::Status => cmd_auth_status(),
        },
        Commands::Current { porcelain } => cmd_current(porcelain),
        Commands::Detect { auto } => cmd_detect(auto),
        Commands::SshSync => cmd_ssh_sync(),
        Commands::Scm { command } => cmd_scm(command),
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
            Select::new("Select profile to remove:", profiles).prompt()?
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

    println!("Authenticating CLI tools for profile '{}'...", name.cyan());
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

fn cmd_auth_status() -> Result<()> {
    use std::collections::HashMap;

    let config = Config::load()?;

    if config.profiles.is_empty() {
        bail!("No profiles configured. Run 'gitid add' first.");
    }

    // Check which platforms are needed
    let needs_gh = config.profile_names().iter().any(|n| {
        config
            .get_profile(n)
            .is_some_and(|p| matches!(p.platform, Platform::Github | Platform::Both))
    });
    let needs_glab = config.profile_names().iter().any(|n| {
        config
            .get_profile(n)
            .is_some_and(|p| matches!(p.platform, Platform::Gitlab | Platform::Both))
    });

    // Fetch auth state once per CLI tool, keyed by host
    let gh_hosts = if needs_gh {
        fetch_gh_hosts()
    } else {
        GhAuthResult::Hosts(HashMap::new())
    };
    let glab_status = if needs_glab {
        fetch_glab_status()
    } else {
        GlabAuthResult::Status {
            authenticated: false,
            host: None,
        }
    };

    // Compute column widths
    let names: Vec<&String> = config.profile_names().into_iter().collect();
    let name_w = names.iter().map(|n| n.len()).max().unwrap_or(7).max(7);
    let plat_w = 8;

    println!(
        "{:<name_w$}  {:<plat_w$}  {:<20}  Authenticated",
        "Profile", "Platform", "Host",
        name_w = name_w,
        plat_w = plat_w,
    );

    for name in &names {
        let profile = match config.get_profile(name) {
            Some(p) => p,
            None => continue,
        };

        let host = profile.default_host();
        let platform_str = format!("{}", profile.platform);

        let auth_cell = match profile.platform {
            Platform::Github => format_gh_auth(&gh_hosts, host),
            Platform::Gitlab => format_glab_auth(&glab_status, host),
            Platform::Both => {
                let gh = format_gh_auth(&gh_hosts, host);
                let gl = format_glab_auth(&glab_status, host);
                format!("gh: {} | glab: {}", gh, gl)
            }
        };

        println!(
            "{:<name_w$}  {:<plat_w$}  {:<20}  {}",
            name.cyan(),
            platform_str,
            host,
            auth_cell,
            name_w = name_w,
            plat_w = plat_w,
        );
    }

    Ok(())
}

/// Cached GitHub auth: map of host → username
enum GhAuthResult {
    Hosts(std::collections::HashMap<String, Option<String>>),
    NotInstalled,
    Error,
}

/// Cached GitLab auth
enum GlabAuthResult {
    Status {
        authenticated: bool,
        host: Option<String>,
    },
    NotInstalled,
    Error,
}

fn fetch_gh_hosts() -> GhAuthResult {
    use std::collections::HashMap;

    match scm::command::run("gh", &["auth", "status", "--json", "hosts"]) {
        Ok(out) => {
            let v: serde_json::Value = match serde_json::from_str(&out.stdout) {
                Ok(v) => v,
                Err(_) => return GhAuthResult::Hosts(HashMap::new()),
            };
            let mut hosts = HashMap::new();
            if let Some(obj) = v.get("hosts").and_then(|h| h.as_object()) {
                for (host, entry) in obj {
                    let user = entry
                        .get("user")
                        .and_then(|u| u.as_str())
                        .map(str::to_string);
                    hosts.insert(host.clone(), user);
                }
            }
            GhAuthResult::Hosts(hosts)
        }
        Err(ScmError::CliMissing(_)) => GhAuthResult::NotInstalled,
        Err(ScmError::CommandFailed(msg)) => {
            let m = msg.to_lowercase();
            if m.contains("not logged") || m.contains("authenticate") {
                GhAuthResult::Hosts(std::collections::HashMap::new())
            } else {
                GhAuthResult::Error
            }
        }
        Err(_) => GhAuthResult::Error,
    }
}

fn fetch_glab_status() -> GlabAuthResult {
    match scm::command::run("glab", &["auth", "status"]) {
        Ok(out) => {
            let text = format!("{}\n{}", out.stdout, out.stderr);
            let authenticated = !text.to_lowercase().contains("not logged");
            let host = text
                .lines()
                .find_map(|l| l.split("Host:").nth(1))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            GlabAuthResult::Status {
                authenticated,
                host,
            }
        }
        Err(ScmError::CliMissing(_)) => GlabAuthResult::NotInstalled,
        Err(ScmError::CommandFailed(msg)) => {
            let m = msg.to_lowercase();
            if m.contains("not logged") || m.contains("authenticate") {
                GlabAuthResult::Status {
                    authenticated: false,
                    host: None,
                }
            } else {
                GlabAuthResult::Error
            }
        }
        Err(_) => GlabAuthResult::Error,
    }
}

fn format_gh_auth(result: &GhAuthResult, host: &str) -> String {
    match result {
        GhAuthResult::Hosts(hosts) => match hosts.get(host) {
            Some(Some(user)) => format!("{}", format!("yes ({})", user).green()),
            Some(None) => format!("{}", "yes".green()),
            None => format!("{}", "no".red()),
        },
        GhAuthResult::NotInstalled => "not installed".dimmed().to_string(),
        GhAuthResult::Error => format!("{}", "error".red()),
    }
}

fn format_glab_auth(result: &GlabAuthResult, host: &str) -> String {
    match result {
        GlabAuthResult::Status {
            authenticated,
            host: auth_host,
        } => {
            let host_matches = auth_host
                .as_ref()
                .map_or(*authenticated, |h| h == host);
            if *authenticated && host_matches {
                format!("{}", "yes".green())
            } else {
                format!("{}", "no".red())
            }
        }
        GlabAuthResult::NotInstalled => "not installed".dimmed().to_string(),
        GlabAuthResult::Error => format!("{}", "error".red()),
    }
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
                        println!("{} Applied profile '{}'", "Success:".green().bold(), name);
                    }
                }
            }
        }
    }

    Ok(())
}

fn cmd_scm(command: ScmCommands) -> Result<()> {
    match command {
        ScmCommands::Status => cmd_scm_status(),
        ScmCommands::Issue { command } => cmd_scm_issue(command),
        ScmCommands::Review { command } => cmd_scm_review(command),
        ScmCommands::Ci { command } => cmd_scm_ci(command),
    }
}

fn cmd_scm_issue(command: ScmIssueCommands) -> Result<()> {
    match command {
        ScmIssueCommands::List => cmd_scm_issue_list(),
    }
}

fn cmd_scm_review(command: ScmReviewCommands) -> Result<()> {
    match command {
        ScmReviewCommands::List => cmd_scm_review_list(),
    }
}

fn cmd_scm_ci(command: ScmCiCommands) -> Result<()> {
    match command {
        ScmCiCommands::List => cmd_scm_ci_list(),
    }
}

fn get_provider_context() -> Result<(ProviderKind, String)> {
    detect_from_repo_origin().context(
        "Could not detect provider from 'origin' remote. Ensure you're in a git repo with origin.",
    )
}

fn cmd_scm_status() -> Result<()> {
    let (kind, remote) = get_provider_context()?;

    println!("Remote: {}", remote);

    match kind {
        ProviderKind::Github => print_auth_status(GitHubProvider, "github", "gh"),
        ProviderKind::Gitlab => print_auth_status(GitLabProvider, "gitlab", "glab"),
        ProviderKind::Unknown => bail!("Unsupported provider in origin remote"),
    }
}

fn print_auth_status<P: ScmProvider>(provider: P, label: &str, cli: &str) -> Result<()> {
    println!("Provider: {} ({})", label, cli);
    match provider.auth_status() {
        Ok(status) => {
            println!(
                "Authenticated: {}",
                if status.authenticated { "yes" } else { "no" }
            );
            if let Some(host) = status.host {
                println!("Host: {}", host);
            }
            Ok(())
        }
        Err(ScmError::NotAuthenticated(_)) => {
            println!("Authenticated: no");
            println!("Next: {} auth login", cli);
            Ok(())
        }
        Err(e) => Err(map_scm_err(e, cli)),
    }
}

fn ensure_authenticated<P: ScmProvider>(provider: P, cli: &str) -> Result<()> {
    match provider.auth_status() {
        Ok(status) if status.authenticated => Ok(()),
        Ok(_) | Err(ScmError::NotAuthenticated(_)) => {
            bail!("Not authenticated. Run: {} auth login", cli)
        }
        Err(e) => Err(map_scm_err(e, cli)),
    }
}

fn cmd_scm_issue_list() -> Result<()> {
    let (kind, _) = get_provider_context()?;

    let issues = match kind {
        ProviderKind::Github => {
            ensure_authenticated(GitHubProvider, "gh")?;
            GitHubProvider
                .list_issues()
                .map_err(|e| map_scm_err(e, "gh"))?
        }
        ProviderKind::Gitlab => {
            ensure_authenticated(GitLabProvider, "glab")?;
            GitLabProvider
                .list_issues()
                .map_err(|e| map_scm_err(e, "glab"))?
        }
        ProviderKind::Unknown => bail!("Unsupported provider in origin remote"),
    };

    print_issues(&issues);
    Ok(())
}

fn cmd_scm_review_list() -> Result<()> {
    let (kind, _) = get_provider_context()?;

    let reviews = match kind {
        ProviderKind::Github => {
            ensure_authenticated(GitHubProvider, "gh")?;
            GitHubProvider
                .list_reviews()
                .map_err(|e| map_scm_err(e, "gh"))?
        }
        ProviderKind::Gitlab => {
            ensure_authenticated(GitLabProvider, "glab")?;
            GitLabProvider
                .list_reviews()
                .map_err(|e| map_scm_err(e, "glab"))?
        }
        ProviderKind::Unknown => bail!("Unsupported provider in origin remote"),
    };

    print_reviews(&reviews);
    Ok(())
}

fn cmd_scm_ci_list() -> Result<()> {
    let (kind, _) = get_provider_context()?;

    let pipelines = match kind {
        ProviderKind::Github => {
            ensure_authenticated(GitHubProvider, "gh")?;
            GitHubProvider
                .list_pipelines()
                .map_err(|e| map_scm_err(e, "gh"))?
        }
        ProviderKind::Gitlab => {
            ensure_authenticated(GitLabProvider, "glab")?;
            GitLabProvider
                .list_pipelines()
                .map_err(|e| map_scm_err(e, "glab"))?
        }
        ProviderKind::Unknown => bail!("Unsupported provider in origin remote"),
    };

    print_pipelines(&pipelines);
    Ok(())
}

fn print_issues(issues: &[scm::types::Issue]) {
    if issues.is_empty() {
        println!("No issues found");
        return;
    }
    for i in issues {
        println!("#{} [{}] {}", i.id, i.state, i.title);
        if let Some(url) = &i.url {
            println!("  {}", url);
        }
    }
}

fn print_reviews(reviews: &[scm::types::Review]) {
    if reviews.is_empty() {
        println!("No reviews found");
        return;
    }
    for r in reviews {
        println!("#{} [{}] {}", r.id, r.state, r.title);
        if let Some(url) = &r.url {
            println!("  {}", url);
        }
    }
}

fn print_pipelines(pipelines: &[scm::types::Pipeline]) {
    if pipelines.is_empty() {
        println!("No pipelines found");
        return;
    }
    for p in pipelines {
        let conclusion = p
            .conclusion
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or("-");
        println!("#{} [{} / {}]", p.id, p.status, conclusion);
        if let Some(url) = &p.url {
            println!("  {}", url);
        }
    }
}

fn map_scm_err(err: ScmError, cli: &str) -> anyhow::Error {
    match err {
        ScmError::NotAuthenticated(_) => {
            anyhow::anyhow!("Not authenticated. Run: {} auth login", cli)
        }
        ScmError::CliMissing(_) => anyhow::anyhow!("{} CLI not installed", cli),
        _ => anyhow::anyhow!(err.to_string()),
    }
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
