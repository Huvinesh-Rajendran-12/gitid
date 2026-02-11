use crate::scm::{
    command,
    provider::{ScmError, ScmProvider, ScmResult},
    types::{AuthStatus, Issue, Pipeline, ProviderKind, Review},
};

#[derive(Debug, Default)]
pub struct GitHubProvider;

impl ScmProvider for GitHubProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Github
    }

    fn auth_status(&self) -> ScmResult<AuthStatus> {
        let out = match command::run("gh", &["auth", "status", "--json", "hosts"]) {
            Ok(out) => out,
            Err(ScmError::CommandFailed(msg)) => {
                let m = msg.to_lowercase();
                if m.contains("not logged") || m.contains("authenticate") {
                    return Ok(AuthStatus {
                        provider: ProviderKind::Github,
                        authenticated: false,
                        host: None,
                        user: None,
                    });
                }
                return Err(ScmError::CommandFailed(msg));
            }
            Err(e) => return Err(e),
        };

        let v: serde_json::Value = serde_json::from_str(&out.stdout)
            .map_err(|e| crate::scm::provider::ScmError::CommandFailed(e.to_string()))?;

        let host = v
            .get("hosts")
            .and_then(|h| h.as_object())
            .and_then(|m| m.keys().next().cloned());

        let user = host.as_ref().and_then(|h| {
            v.get("hosts")
                .and_then(|hs| hs.get(h))
                .and_then(|e| e.get("user"))
                .and_then(|u| u.as_str())
                .map(str::to_string)
        });

        Ok(AuthStatus {
            provider: ProviderKind::Github,
            authenticated: host.is_some(),
            host,
            user,
        })
    }

    fn list_issues(&self) -> ScmResult<Vec<Issue>> {
        let out = command::run(
            "gh",
            &[
                "issue",
                "list",
                "--limit",
                "20",
                "--json",
                "number,title,state,url",
            ],
        )?;
        parse_issues(&out.stdout)
    }

    fn list_reviews(&self) -> ScmResult<Vec<Review>> {
        let out = command::run(
            "gh",
            &[
                "pr",
                "list",
                "--limit",
                "20",
                "--json",
                "number,title,state,url",
            ],
        )?;
        parse_reviews(&out.stdout)
    }

    fn list_pipelines(&self) -> ScmResult<Vec<Pipeline>> {
        let out = command::run(
            "gh",
            &[
                "run",
                "list",
                "--limit",
                "20",
                "--json",
                "databaseId,status,conclusion,url",
            ],
        )?;
        parse_pipelines(&out.stdout)
    }
}

fn parse_issues(json: &str) -> ScmResult<Vec<Issue>> {
    let arr: Vec<serde_json::Value> = serde_json::from_str(json)
        .map_err(|e| crate::scm::provider::ScmError::CommandFailed(e.to_string()))?;

    Ok(arr
        .into_iter()
        .map(|v| Issue {
            id: v
                .get("number")
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
            url: v.get("url").and_then(|s| s.as_str()).map(str::to_string),
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
                .get("number")
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
            url: v.get("url").and_then(|s| s.as_str()).map(str::to_string),
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
                .get("databaseId")
                .or_else(|| v.get("id"))
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
                .and_then(|s| s.as_str())
                .map(str::to_string),
            url: v.get("url").and_then(|s| s.as_str()).map(str::to_string),
        })
        .collect())
}
