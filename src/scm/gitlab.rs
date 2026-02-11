use crate::scm::{
    command,
    provider::{ScmError, ScmProvider, ScmResult},
    types::{AuthStatus, Issue, Pipeline, ProviderKind, Review},
};

#[derive(Debug, Default)]
pub struct GitLabProvider;

impl ScmProvider for GitLabProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Gitlab
    }

    fn auth_status(&self) -> ScmResult<AuthStatus> {
        let out = match command::run("glab", &["auth", "status"]) {
            Ok(out) => out,
            Err(ScmError::CommandFailed(msg)) => {
                let m = msg.to_lowercase();
                if m.contains("not logged") || m.contains("authenticate") {
                    return Ok(AuthStatus {
                        provider: ProviderKind::Gitlab,
                        authenticated: false,
                        host: None,
                        user: None,
                    });
                }
                return Err(ScmError::CommandFailed(msg));
            }
            Err(e) => return Err(e),
        };

        let text = format!("{}\n{}", out.stdout, out.stderr);
        let authenticated = !text.to_lowercase().contains("not logged");
        Ok(AuthStatus {
            provider: ProviderKind::Gitlab,
            authenticated,
            host: extract_host(&text),
            user: None,
        })
    }

    fn list_issues(&self) -> ScmResult<Vec<Issue>> {
        let out = command::run("glab", &["issue", "list", "--output", "json"])?;
        parse_issues(&out.stdout)
    }

    fn list_reviews(&self) -> ScmResult<Vec<Review>> {
        let out = command::run("glab", &["mr", "list", "--output", "json"])?;
        parse_reviews(&out.stdout)
    }

    fn list_pipelines(&self) -> ScmResult<Vec<Pipeline>> {
        let out = command::run("glab", &["ci", "list", "--output", "json"])?;
        parse_pipelines(&out.stdout)
    }
}

fn extract_host(text: &str) -> Option<String> {
    text.lines()
        .find_map(|l| l.split("Host:").nth(1))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn parse_issues(json: &str) -> ScmResult<Vec<Issue>> {
    let arr: Vec<serde_json::Value> = serde_json::from_str(json)
        .map_err(|e| crate::scm::provider::ScmError::CommandFailed(e.to_string()))?;

    Ok(arr
        .into_iter()
        .map(|v| Issue {
            id: v
                .get("iid")
                .or_else(|| v.get("id"))
                .and_then(|n| n.as_i64())
                .unwrap_or_default()
                .to_string(),
            title: v
                .get("title")
                .and_then(|s| s.as_str())
                .unwrap_or_default()
                .to_string(),
            state: v
                .get("state")
                .and_then(|s| s.as_str())
                .unwrap_or_default()
                .to_string(),
            url: v
                .get("web_url")
                .or_else(|| v.get("url"))
                .and_then(|s| s.as_str())
                .map(str::to_string),
        })
        .collect())
}

fn parse_reviews(json: &str) -> ScmResult<Vec<Review>> {
    let arr: Vec<serde_json::Value> = serde_json::from_str(json)
        .map_err(|e| crate::scm::provider::ScmError::CommandFailed(e.to_string()))?;

    Ok(arr
        .into_iter()
        .map(|v| Review {
            id: v
                .get("iid")
                .or_else(|| v.get("id"))
                .and_then(|n| n.as_i64())
                .unwrap_or_default()
                .to_string(),
            title: v
                .get("title")
                .and_then(|s| s.as_str())
                .unwrap_or_default()
                .to_string(),
            state: v
                .get("state")
                .and_then(|s| s.as_str())
                .unwrap_or_default()
                .to_string(),
            url: v
                .get("web_url")
                .or_else(|| v.get("url"))
                .and_then(|s| s.as_str())
                .map(str::to_string),
        })
        .collect())
}

fn parse_pipelines(json: &str) -> ScmResult<Vec<Pipeline>> {
    let arr: Vec<serde_json::Value> = serde_json::from_str(json)
        .map_err(|e| crate::scm::provider::ScmError::CommandFailed(e.to_string()))?;

    Ok(arr
        .into_iter()
        .map(|v| Pipeline {
            id: v
                .get("id")
                .or_else(|| v.get("pipeline_id"))
                .and_then(|n| n.as_i64())
                .unwrap_or_default()
                .to_string(),
            status: v
                .get("status")
                .and_then(|s| s.as_str())
                .unwrap_or_default()
                .to_string(),
            conclusion: v
                .get("conclusion")
                .or_else(|| v.get("result"))
                .and_then(|s| s.as_str())
                .map(str::to_string),
            url: v
                .get("web_url")
                .or_else(|| v.get("url"))
                .and_then(|s| s.as_str())
                .map(str::to_string),
        })
        .collect())
}
