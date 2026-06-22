use anyhow::{bail, Context};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Run a git command and return trimmed stdout.
pub fn run(args: &[&str], cwd: Option<&Path>) -> anyhow::Result<String> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let out = cmd.output().with_context(|| {
        let display: String = args.iter().copied().collect::<Vec<_>>().join(" ");
        format!("failed to run git {display}")
    })?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let cmd_str = args.iter().copied().collect::<Vec<_>>().join(" ");
        bail!("git {cmd_str} failed: {stderr}");
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Resolve the worktree root (git rev-parse --show-toplevel).
pub fn top_level(cwd: Option<&Path>) -> anyhow::Result<PathBuf> {
    let out = run(&["rev-parse", "--show-toplevel"], cwd)?;
    Ok(PathBuf::from(out))
}

/// Resolve the common dir (absolute).
pub fn common_dir(cwd: Option<&Path>) -> anyhow::Result<PathBuf> {
    let out = run(&["rev-parse", "--git-common-dir"], cwd)?;
    let p = PathBuf::from(&out);
    if p.is_absolute() {
        return Ok(p);
    }
    let base = cwd.unwrap_or(Path::new("."));
    Ok(base.join(p))
}

/// Get the current branch name.
pub fn current_branch(cwd: Option<&Path>) -> anyhow::Result<String> {
    run(&["rev-parse", "--abbrev-ref", "HEAD"], cwd)
}

/// Detect the remote default branch.
pub fn remote_default(cwd: Option<&Path>) -> anyhow::Result<String> {
    let result = run(&["symbolic-ref", "--quiet", "--short", "refs/remotes/origin/HEAD"], cwd);
    if let Ok(ref_name) = result {
        let def = ref_name.strip_prefix("origin/").unwrap_or(&ref_name);
        if !def.is_empty() && def != "(unknown)" {
            return Ok(def.to_string());
        }
    }
    let out = run(&["remote", "show", "origin"], cwd)?;
    for line in out.lines() {
        if let Some(branch) = line.strip_prefix("  HEAD branch: ") {
            let branch = branch.trim();
            if !branch.is_empty() && branch != "(unknown)" {
                return Ok(branch.to_string());
            }
        }
    }
    bail!("cannot detect remote default branch; try -m or -c")
}

/// Fetch the remote default branch.
pub fn fetch_remote(remote: &str, default: &str, cwd: Option<&Path>) -> anyhow::Result<()> {
    run(&["fetch", remote, default, "--quiet"], cwd)?;
    let ref_path = format!("refs/remotes/{remote}/{default}");
    if !run(&["show-ref", "--verify", "--quiet", &ref_path], cwd).is_ok() {
        bail!("{remote}/{default} not found after fetch");
    }
    Ok(())
}

/// Check if a ref exists.
pub fn ref_exists(ref_path: &str, cwd: Option<&Path>) -> bool {
    run(&["show-ref", "--verify", "--quiet", ref_path], cwd).is_ok()
}

/// Check if a worktree path is the main worktree (contains .git as a dir, not a file).
pub fn is_main_worktree(path: &Path) -> bool {
    path.join(".git").is_dir()
}

/// Get the worktree root for a common dir.
pub fn worktree_root_for_common(common_dir: &Path) -> PathBuf {
    common_dir.parent()
        .unwrap_or(common_dir)
        .to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper: create a temp git repo with an initial commit.
    fn make_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        let p = dir.path();
        run(&["init"], Some(p)).unwrap();
        run(&["config", "user.email", "test@test.com"], Some(p)).unwrap();
        run(&["config", "user.name", "Test"], Some(p)).unwrap();
        fs::write(p.join("README.md"), "# test").unwrap();
        run(&["add", "."], Some(p)).unwrap();
        run(&["commit", "-m", "init", "--allow-empty"], Some(p)).unwrap();
        dir
    }

    #[test]
    fn top_level_returns_repo_root() {
        let repo = make_repo();
        let root = top_level(Some(repo.path())).unwrap();
        assert_eq!(root, fs::canonicalize(repo.path()).unwrap());
    }

    #[test]
    fn common_dir_returns_git_dir() {
        let repo = make_repo();
        let cd = common_dir(Some(repo.path())).unwrap();
        assert_eq!(cd, repo.path().join(".git"));
    }

    #[test]
    fn current_branch_returns_main_or_master() {
        let repo = make_repo();
        let branch = current_branch(Some(repo.path())).unwrap();
        assert!(branch == "main" || branch == "master");
    }

    #[test]
    fn is_main_worktree_true_for_dir_git() {
        let repo = make_repo();
        assert!(is_main_worktree(repo.path()));
    }

    #[test]
    fn is_main_worktree_false_for_worktree() {
        let repo = make_repo();
        let wt_dir = repo.path().join("wt-feature");
        run(&["worktree", "add", "-b", "feature", wt_dir.to_str().unwrap()], Some(repo.path())).unwrap();
        assert!(!is_main_worktree(&wt_dir));
    }

    #[test]
    fn worktree_root_for_common_points_to_repo_root() {
        let repo = make_repo();
        let cd = common_dir(Some(repo.path())).unwrap();
        let root = worktree_root_for_common(&cd);
        assert_eq!(root, repo.path());
    }
}
