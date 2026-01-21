# gitid

A CLI tool for seamlessly managing multiple Git identities across GitHub and GitLab on a single machine.

## Overview

`gitid` solves the challenge of switching between different Git identities (work vs. personal accounts, multiple organizations, etc.). It automatically manages SSH keys, Git configuration, and authentication for different accounts and platforms.

**Use cases:**
- Developers with multiple GitHub/GitLab accounts
- Teams using GitHub Enterprise or self-hosted GitLab instances
- Organizations requiring different SSH keys per account
- Projects requiring GPG-signed commits with specific keys

## Features

✨ **Core Features:**
- 🔑 **Multiple Identity Management** - Configure and switch between unlimited Git identities
- 🔐 **SSH Key Management** - Automatic SSH key discovery, generation, and configuration
- 🤖 **Auto-Detection** - Intelligently detect the correct profile based on repository remote URL
- 🏢 **Enterprise Support** - Configure custom hosts for GitHub Enterprise and self-hosted GitLab
- 📝 **GPG Signing** - Optional GPG key configuration per profile
- 🔗 **CLI Integration** - Seamless integration with `gh` and `glab` CLIs
- 🔄 **Flexible Scoping** - Apply profiles globally or per-repository
- 🎯 **Shell Prompt Integration** - Machine-readable output for shell prompt customization

## Installation

### Prerequisites
- Rust 1.70+
- Git
- SSH (for key management)
- Optional: `gh` CLI (for GitHub authentication) and `glab` CLI (for GitLab authentication)

### Build from Source

```bash
cargo build --release
sudo cp target/release/gitid /usr/local/bin/
```

Or install directly:

```bash
cargo install --path .
```

## Quick Start

### Initialize gitid

```bash
gitid init
```

This creates the configuration directory at `~/.config/gitid/` and generates an empty config file.

### Add Your First Identity

```bash
gitid add
```

You'll be prompted to enter:
- **Profile name** (e.g., "work", "personal")
- **Email address**
- **Git platform** (GitHub, GitLab, or Both)
- **SSH key** (auto-discovered keys or generate new)
- **GPG key** (optional)
- **Custom host** (optional, for enterprise instances)

### Switch to a Profile

```bash
# Set globally (affects all repositories)
gitid use work

# Set for current repository only
gitid use work --local
```

### View Your Profiles

```bash
# List all configured profiles
gitid list

# Show currently active profile
gitid current
```

### Auto-Detect Profile

```bash
# Detect the best matching profile for current repository
gitid detect
```

The detection algorithm scores profiles based on SSH alias, host, and platform matches.

## Usage

### Commands

#### `gitid init`
Initialize gitid configuration directory and create an empty config file.

```bash
gitid init
```

#### `gitid add`
Add a new Git identity profile interactively.

```bash
gitid add
```

#### `gitid remove`
Remove an existing profile.

```bash
gitid remove
```

You'll be prompted to select a profile to delete.

#### `gitid list`
List all configured profiles with their current status.

```bash
gitid list
```

Output shows profile name, email, platform, SSH key, and whether it's currently active.

#### `gitid use`
Switch to a profile for Git operations.

```bash
# Set globally (all repositories)
gitid use work

# Set for current repository only (local)
gitid use personal --local

# Reset to default/unset
gitid use --unset
```

#### `gitid current`
Display the currently active Git identity.

```bash
# Human-readable output (default)
gitid current

# Machine-readable output for shell prompts
gitid current --porcelain
```

#### `gitid detect`
Auto-detect and display the recommended profile for the current repository based on its remote URL.

```bash
gitid detect
```

Shows the detected profile with a scoring explanation (SSH alias matches, host matches, etc.).

#### `gitid auth`
Authenticate CLI tools (`gh` for GitHub, `glab` for GitLab) for a profile.

```bash
# Authenticate GitHub CLI
gitid auth github

# Authenticate GitLab CLI with custom host
gitid auth gitlab --host gitlab.company.com
```

#### `gitid ssh-sync`
Synchronize SSH config file with all profiles. This updates `~/.ssh/config` with host aliases for each profile's SSH key.

```bash
gitid ssh-sync
```

## Configuration

Configuration is stored in `~/.config/gitid/config.toml` in TOML format.

### Example Configuration

```toml
[[profiles]]
name = "work"
email = "user@company.com"
platform = "github"
ssh_key = "~/.ssh/github_work"
gpg_key = "1A2B3C4D5E6F7890"

[[profiles]]
name = "personal"
email = "user@gmail.com"
platform = "github"
ssh_key = "~/.ssh/github_personal"

[[profiles]]
name = "gitlab"
email = "user@gitlab.example.com"
platform = "gitlab"
ssh_key = "~/.ssh/gitlab_key"
host = "gitlab.company.com"
```

### Profile Fields

- **name** (required) - Unique identifier for the profile
- **email** (required) - Git commit email address
- **platform** (required) - `github`, `gitlab`, or `both`
- **ssh_key** (required) - Path to SSH private key
- **gpg_key** (optional) - GPG key ID for commit signing
- **host** (optional) - Custom hostname for enterprise instances

## Workflow Examples

### Example 1: Switch Between Work and Personal Accounts

```bash
# Add work profile
gitid add
# Name: work, Email: john.doe@company.com, Platform: GitHub

# Add personal profile
gitid add
# Name: personal, Email: john@example.com, Platform: GitHub

# Use work profile globally
gitid use work

# Work on personal project locally
cd ~/projects/personal-project
gitid use personal --local

# Check current profile
gitid current
# Output: personal (local)

# Verify git config
git config user.name    # john
git config user.email   # john@example.com
```

### Example 2: Enterprise GitHub Instance

```bash
# Add enterprise profile with custom host
gitid add
# Name: enterprise-work
# Email: john@enterprise.com
# Platform: GitHub
# SSH Key: ~/.ssh/github_enterprise
# Custom Host: github.enterprise.com

# SSH config is automatically generated with:
# Host github-enterprise-work
#   HostName github.enterprise.com
#   User git
#   IdentityFile ~/.ssh/github_enterprise
```

### Example 3: Auto-Detect Profile in Repository

```bash
cd ~/projects/work-repo
# Remote: git@github.com:company/repo.git

gitid detect
# Output: Detected profile "work" based on GitHub remote URL

gitid use work
```

### Example 4: GPG-Signed Commits

```bash
# Add profile with GPG key
gitid add
# Name: secure-work
# Email: secure@company.com
# Platform: GitHub
# SSH Key: ~/.ssh/github_secure
# GPG Key: 1A2B3C4D5E6F7890

gitid use secure-work

# Git will now sign commits automatically
git commit -m "Important change"
# Commit will be signed with the specified GPG key
```

## Shell Integration

### Show Current Profile in Prompt

Use the porcelain output for shell prompt integration:

```bash
# Bash
export PS1='[\$(gitid current --porcelain)] $ '

# Zsh
setopt PROMPT_SUBST
PROMPT='[$(gitid current --porcelain)] $ '

# Fish
function fish_prompt
    echo -n "["(gitid current --porcelain)"] $ "
end
```

## SSH Configuration

### Auto-Generated SSH Config

When using `gitid ssh-sync`, SSH host aliases are automatically created:

```bash
gitid ssh-sync
```

This generates entries in `~/.ssh/config` like:

```
# gitid: work
Host github-work
  HostName github.com
  User git
  IdentityFile ~/.ssh/github_work

# gitid: personal
Host github-personal
  HostName github.com
  User git
  IdentityFile ~/.ssh/github_personal
```

You can clone repositories using these aliases:

```bash
# Instead of: git clone git@github.com:user/repo.git
# Use:        git clone git@github-work:user/repo.git
```

## Troubleshooting

### Profile Not Being Applied

1. Verify the profile exists: `gitid list`
2. Check if profile is set: `gitid current`
3. For local repository: Ensure you're in the correct directory
4. Verify git config: `git config user.email`

### SSH Key Issues

1. Check SSH key path exists: `ls -la ~/.ssh/`
2. Verify key permissions: `chmod 600 ~/.ssh/key_name`
3. Generate new key if needed: `ssh-keygen -t ed25519`
4. Sync SSH config: `gitid ssh-sync`

### Authentication Issues

1. Ensure `gh` or `glab` CLI is installed
2. Authenticate with: `gitid auth github` or `gitid auth gitlab`
3. Verify authentication: `gh auth status` or `glab auth status`

## Development

### Build

```bash
cargo build
```

### Run Tests

```bash
cargo test
```

### Run

```bash
cargo run -- --help
```

## Architecture

- **config.rs** - Configuration management and persistence
- **profile.rs** - Profile data structures and validation
- **git.rs** - Git operations wrapper
- **ssh.rs** - SSH config file management
- **ssh_keys.rs** - SSH key discovery and generation
- **detect.rs** - Profile auto-detection logic
- **auth/** - CLI authentication (GitHub, GitLab)
- **prompt.rs** - Current profile display and queries
- **cli.rs** - Command-line interface definitions

## License

This project is open source and available under the MIT License.

## Contributing

Contributions are welcome! Please feel free to submit pull requests for bug fixes, improvements, or new features.

## Support

For issues, questions, or suggestions, please open an issue on the project repository.
