use crate::git;
use anyhow::{bail, Context};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Base {
    RemoteMain,
    LocalMain,
    Current,
}

/// Resolve the flags into a Base. Default is RemoteMain if none set.
pub fn resolve_flag(remote_main: bool, local_main: bool, current: bool) -> anyhow::Result<Base> {
    let count = [remote_main, local_main, current].iter().filter(|&&b| b).count();
    if count > 1 {
        bail!("only one base flag may be specified (-r, -m, -c)");
    }
    if local_main { Ok(Base::LocalMain) }
    else if current { Ok(Base::Current) }
    else { Ok(Base::RemoteMain) }
}

impl Base {
    /// Resolve to a git ref string suitable for `git worktree add`.
    pub fn resolve(&self, cwd: Option<&Path>) -> anyhow::Result<String> {
        match self {
            Base::RemoteMain => resolve_remote_main(cwd),
            Base::LocalMain => resolve_local_main(cwd),
            Base::Current => git::current_branch(cwd),
        }
    }
}

fn resolve_remote_main(cwd: Option<&Path>) -> anyhow::Result<String> {
    let default = git::remote_default(cwd)?;
    git::fetch_remote("origin", &default, cwd)?;
    Ok(format!("origin/{default}"))
}

fn resolve_local_main(cwd: Option<&Path>) -> anyhow::Result<String> {
    let common = git::common_dir(cwd)?;
    let root = common.parent().unwrap_or(&common);
    let out = std::process::Command::new("git")
        .args(["symbolic-ref", "--short", "HEAD"])
        .current_dir(root)
        .output()
        .context("failed to run git symbolic-ref")?;

    if out.status.success() {
        let branch = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !branch.is_empty() {
            return Ok(branch);
        }
    }

    // Detached HEAD — try common branch names
    for candidate in &["main", "master"] {
        let ref_path = format!("refs/heads/{candidate}");
        if git::ref_exists(&ref_path, cwd) {
            return Ok(candidate.to_string());
        }
    }

    bail!("cannot determine local default branch; try -r or -c")
}
