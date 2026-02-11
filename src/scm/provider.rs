use crate::scm::types::{AuthStatus, Issue, Pipeline, ProviderKind, Release, Review};
use thiserror::Error;

pub type ScmResult<T> = Result<T, ScmError>;

#[derive(Debug, Error)]
pub enum ScmError {
    #[error("CLI not installed: {0}")]
    CliMissing(String),
    #[error("Not authenticated: {0}")]
    NotAuthenticated(String),
    #[error("Repository context missing")]
    RepoContextMissing,
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Rate limited")]
    RateLimited,
    #[error("Command failed: {0}")]
    CommandFailed(String),
    #[error("Not implemented: {0}")]
    NotImplemented(&'static str),
}

pub trait ScmProvider {
    fn kind(&self) -> ProviderKind;
    fn auth_status(&self) -> ScmResult<AuthStatus>;

    fn list_issues(&self) -> ScmResult<Vec<Issue>> {
        Err(ScmError::NotImplemented("list_issues"))
    }
    fn list_reviews(&self) -> ScmResult<Vec<Review>> {
        Err(ScmError::NotImplemented("list_reviews"))
    }
    fn list_pipelines(&self) -> ScmResult<Vec<Pipeline>> {
        Err(ScmError::NotImplemented("list_pipelines"))
    }
    fn list_releases(&self) -> ScmResult<Vec<Release>> {
        Err(ScmError::NotImplemented("list_releases"))
    }
}
