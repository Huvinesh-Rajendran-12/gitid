use crate::{git, scm::types::ProviderKind};

pub fn detect_from_url(url: &str) -> ProviderKind {
    let host = git::RemoteUrl::parse(url).map(|u| u.host.to_lowercase());
    match host.as_deref() {
        Some(h) if h.contains("github") => ProviderKind::Github,
        Some(h) if h.contains("gitlab") => ProviderKind::Gitlab,
        _ => ProviderKind::Unknown,
    }
}

pub fn detect_from_repo_origin() -> Option<(ProviderKind, String)> {
    let url = git::get_remote_url("origin").ok().flatten()?;
    Some((detect_from_url(&url), url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_github() {
        assert_eq!(
            detect_from_url("git@github.com:owner/repo.git"),
            ProviderKind::Github
        );
    }

    #[test]
    fn detects_gitlab() {
        assert_eq!(
            detect_from_url("https://gitlab.com/group/repo.git"),
            ProviderKind::Gitlab
        );
    }

    #[test]
    fn detects_unknown() {
        assert_eq!(
            detect_from_url("ssh://code.example.com/group/repo.git"),
            ProviderKind::Unknown
        );
    }
}
