# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`gitid` is a Rust CLI tool for managing multiple Git identities (SSH keys, git config, GPG signing) across GitHub and GitLab on a single machine. Config is stored at `~/.config/gitid/config.toml`.

## Build & Development Commands

```bash
cargo build                  # Build debug
cargo build --release        # Build release
cargo test                   # Run all tests
cargo test <test_name>       # Run a single test
cargo clippy --all-targets --all-features -- -D warnings  # Lint
cargo fmt --all -- --check   # Check formatting
cargo fmt --all              # Fix formatting
cargo run -- <subcommand>    # Run locally (e.g., cargo run -- list)
```

Pre-commit hooks enforce `fmt`, `clippy`, and `test` on commit/push.

## Code Preferences (from AGENTS.md)

- Keep implementations minimal — lowest practical LOC
- Avoid bloat and unnecessary abstractions
- Prefer simple, direct solutions over broad scaffolding
- Think before coding; choose the smallest robust approach

## Architecture

Single-crate Rust binary (`src/main.rs` is the entrypoint). All command handlers live in `main.rs` as `cmd_*` functions dispatched via clap's `Commands` enum defined in `cli.rs`.

Key modules:
- **config.rs** — TOML config load/save/init (`~/.config/gitid/config.toml`)
- **profile.rs** — `Profile` struct, `Platform` enum, validation
- **git.rs** — Git config read/write, `ConfigScope` (Local/Global)
- **ssh.rs** — SSH config file sync (reads/writes `~/.ssh/config` managed block)
- **ssh_keys.rs** — SSH key discovery and ed25519 key generation
- **detect.rs** — Profile auto-detection by scoring against repo remote URL
- **prompt.rs** — Current profile resolution and `--porcelain` output
- **auth/** — `gh`/`glab` CLI auth wrappers (`auth/github.rs`, `auth/gitlab.rs`)
- **scm/** — Read-only SCM integration via `gh`/`glab` CLIs:
  - `scm/provider.rs` — `ScmProvider` trait + `ScmError` enum
  - `scm/github.rs` / `scm/gitlab.rs` — Provider implementations
  - `scm/detect.rs` — Provider detection from git remote URL
  - `scm/command.rs` — Safe command runner with timeout/stderr handling
  - `scm/types.rs` — Normalized models (`Issue`, `Review`, `Pipeline`, `AuthStatus`, `ProviderKind`)

## Key Patterns

- Interactive prompts use the `inquire` crate (Select, Text, Confirm)
- Error handling: `anyhow::Result` for commands, `thiserror` for `ScmError`
- Colored terminal output via the `colored` crate
- SCM commands follow a detect-provider → check-auth → call-provider → print-result flow
- SSH config uses a managed block delimited by `# gitid:` comments
