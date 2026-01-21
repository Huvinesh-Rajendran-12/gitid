use crate::config::Config;
use crate::git::{self, RemoteUrl};
use crate::profile::{Platform, Profile};
use anyhow::Result;

/// Detection result with scoring
#[derive(Debug)]
pub struct DetectionResult {
    pub profile_name: String,
    pub score: u32,
    pub reason: String,
}

/// Detect the best matching profile for the current repository
pub fn detect_profile(config: &Config) -> Result<Option<DetectionResult>> {
    // Get all remotes
    let remotes = git::list_remotes()?;
    if remotes.is_empty() {
        return Ok(None);
    }

    let mut best_match: Option<DetectionResult> = None;

    // Check each remote
    for remote in remotes {
        if let Some(url_str) = git::get_remote_url(&remote)? {
            if let Some(remote_url) = RemoteUrl::parse(&url_str) {
                // Score each profile against this remote
                for (name, profile) in &config.profiles {
                    let score = score_profile(&remote_url, name, profile);
                    if score > 0 {
                        let reason = format_match_reason(&remote_url, profile);

                        if best_match.as_ref().map_or(true, |m| score > m.score) {
                            best_match = Some(DetectionResult {
                                profile_name: name.clone(),
                                score,
                                reason,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(best_match)
}

/// Score how well a profile matches a remote URL
fn score_profile(remote_url: &RemoteUrl, profile_name: &str, profile: &Profile) -> u32 {
    let mut score = 0u32;

    let profile_host = profile.default_host();
    let remote_host = &remote_url.host;

    // Check if the remote host matches the profile's SSH alias
    let ssh_alias = profile.ssh_host_alias(profile_name);
    if remote_host == &ssh_alias {
        // Direct alias match - highest score
        score += 100;
    }

    // Check platform-specific aliases for 'both' platform
    if matches!(profile.platform, Platform::Both) {
        let github_alias = format!("github-{}", profile_name);
        let gitlab_alias = format!("gitlab-{}", profile_name);
        if remote_host == &github_alias || remote_host == &gitlab_alias {
            score += 100;
        }
    }

    // Check if the remote host matches the profile's configured host
    if remote_host == profile_host {
        score += 50;
    }

    // Check platform compatibility
    let is_github = remote_host.contains("github");
    let is_gitlab = remote_host.contains("gitlab");

    match profile.platform {
        Platform::Github if is_github => score += 20,
        Platform::Gitlab if is_gitlab => score += 20,
        Platform::Both if is_github || is_gitlab => score += 15,
        _ => {}
    }

    // Check for custom host match (enterprise instances)
    if let Some(ref custom_host) = profile.host {
        if remote_host == custom_host || remote_host.contains(custom_host.as_str()) {
            score += 80;
        }
    }

    score
}

/// Format a human-readable reason for the match
fn format_match_reason(remote_url: &RemoteUrl, profile: &Profile) -> String {
    let host = &remote_url.host;
    let profile_host = profile.default_host();

    if host == profile_host {
        format!("Remote host '{}' matches profile host", host)
    } else if host.contains("github") && matches!(profile.platform, Platform::Github | Platform::Both)
    {
        format!("GitHub repository detected ({})", host)
    } else if host.contains("gitlab") && matches!(profile.platform, Platform::Gitlab | Platform::Both)
    {
        format!("GitLab repository detected ({})", host)
    } else {
        format!("Host '{}' matched", host)
    }
}

/// Detect profile and return matching information
pub fn detect_and_suggest(config: &Config) -> Result<Option<(String, String)>> {
    if let Some(result) = detect_profile(config)? {
        Ok(Some((result.profile_name, result.reason)))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_github_profile() {
        let remote_url = RemoteUrl {
            host: "github.com".to_string(),
        };

        let profile = Profile::new(
            "John Doe".to_string(),
            "john@example.com".to_string(),
            Platform::Github,
            "~/.ssh/id_ed25519".to_string(),
            None,
            None,
        );

        let score = score_profile(&remote_url, "personal", &profile);
        assert!(score > 0);
    }

    #[test]
    fn test_score_ssh_alias_match() {
        let remote_url = RemoteUrl {
            host: "github-work".to_string(),
        };

        let profile = Profile::new(
            "John Doe".to_string(),
            "john@company.com".to_string(),
            Platform::Github,
            "~/.ssh/id_work".to_string(),
            None,
            None,
        );

        let score = score_profile(&remote_url, "work", &profile);
        assert!(score >= 100);
    }
}
