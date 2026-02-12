use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "gitid")]
#[command(
    author,
    version,
    about = "Manage multiple Git identities across GitHub and GitLab"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize gitid configuration directory and create empty config
    Init,

    /// Add a new profile interactively
    Add {
        /// Profile name (e.g., 'work', 'personal')
        name: Option<String>,

        /// Git user name
        #[arg(long)]
        user_name: Option<String>,

        /// Git email
        #[arg(long)]
        email: Option<String>,

        /// Platform: github, gitlab, or both
        #[arg(long)]
        platform: Option<String>,

        /// Path to SSH private key
        #[arg(long)]
        ssh_key: Option<String>,

        /// GPG signing key ID (optional)
        #[arg(long)]
        gpg_key: Option<String>,

        /// Custom host for enterprise instances (optional)
        #[arg(long)]
        host: Option<String>,
    },

    /// Remove a profile
    Remove {
        /// Profile name to remove (interactive if not provided)
        name: Option<String>,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,

        /// Also remove SSH config entry
        #[arg(long)]
        clean_ssh: bool,
    },

    /// List all configured profiles
    List,

    /// Switch to a profile
    Use {
        /// Profile name to switch to (interactive if not provided)
        name: Option<String>,

        /// Apply globally instead of to current repository
        #[arg(short, long)]
        global: bool,
    },

    /// Authenticate CLI tools (gh/glab) for a profile
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },

    /// Show current active profile
    Current {
        /// Machine-readable output for shell prompts
        #[arg(long)]
        porcelain: bool,
    },

    /// Auto-detect appropriate profile from repository remote
    Detect {
        /// Automatically apply detected profile without prompting
        #[arg(short, long)]
        auto: bool,
    },

    /// Sync SSH config with all profiles
    #[command(name = "ssh-sync")]
    SshSync,

    /// Direct SCM operations via gh/glab
    Scm {
        #[command(subcommand)]
        command: ScmCommands,
    },
}

#[derive(Subcommand)]
pub enum AuthCommands {
    /// Authenticate CLI tools (gh/glab) for a profile
    Login {
        /// Profile name to authenticate (interactive if not provided)
        name: Option<String>,
    },
    /// Show authentication status for all profiles
    Status,
}

#[derive(Subcommand)]
pub enum ScmCommands {
    /// Show provider detection and auth status
    Status,

    /// Issue operations
    Issue {
        #[command(subcommand)]
        command: ScmIssueCommands,
    },

    /// Review operations (PR/MR)
    Review {
        #[command(subcommand)]
        command: ScmReviewCommands,
    },

    /// CI operations
    Ci {
        #[command(subcommand)]
        command: ScmCiCommands,
    },
}

#[derive(Subcommand)]
pub enum ScmIssueCommands {
    /// List issues
    List,
}

#[derive(Subcommand)]
pub enum ScmReviewCommands {
    /// List reviews (PR/MR)
    List,
}

#[derive(Subcommand)]
pub enum ScmCiCommands {
    /// List CI pipelines/runs
    List,
}
