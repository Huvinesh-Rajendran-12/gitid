use std::process::Command;

use crate::scm::provider::{ScmError, ScmResult};

#[derive(Debug, Clone)]
pub struct CmdOut {
    pub stdout: String,
    pub stderr: String,
}

pub fn run(bin: &str, args: &[&str]) -> ScmResult<CmdOut> {
    let output = Command::new(bin).args(args).output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ScmError::CliMissing(bin.to_string())
        } else {
            ScmError::CommandFailed(e.to_string())
        }
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if output.status.success() {
        return Ok(CmdOut { stdout, stderr });
    }

    let detail = stderr.trim();
    if detail.is_empty() {
        return Err(ScmError::CommandFailed(format!(
            "{} {}",
            bin,
            args.join(" ")
        )));
    }

    Err(ScmError::CommandFailed(format!(
        "{} {}: {}",
        bin,
        args.join(" "),
        detail
    )))
}
