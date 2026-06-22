use crate::{cd, git};
use anyhow::bail;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

/// `wt go [<query>]` — cd into a worktree by name or fzf picker.
pub fn run(query: Option<&str>) -> anyhow::Result<()> {
    let worktrees = git::worktree_list(None)?;

    let selected = match query {
        Some(q) => select_by_query(&worktrees, q)?,
        None => select_by_fzf(&worktrees)?,
    };

    cd::emit(&selected.path)?;
    Ok(())
}

/// `wt top` — cd into the main worktree.
pub fn run_top() -> anyhow::Result<()> {
    let common = git::common_dir(None)?;
    let root = git::worktree_root_for_common(&common);
    cd::emit(&root)?;
    Ok(())
}

/// Select a worktree by substring match on path or branch.
pub(crate) fn select_by_query<'a>(
    worktrees: &'a [git::Worktree],
    query: &str,
) -> anyhow::Result<&'a git::Worktree> {
    let q_lower = query.to_lowercase();
    let matches: Vec<&git::Worktree> = worktrees
        .iter()
        .filter(|wt| {
            let path_match = wt.path.to_string_lossy().to_lowercase().contains(&q_lower);
            let branch_match = wt
                .branch
                .as_ref()
                .map(|b| b.to_lowercase().contains(&q_lower))
                .unwrap_or(false);
            path_match || branch_match
        })
        .collect();

    match matches.len() {
        0 => bail!("no worktree matches '{query}'"),
        1 => Ok(matches[0]),
        _ => {
            eprintln!("ambiguous query '{query}', matches:");
            for wt in &matches {
                let branch = wt.branch.as_deref().unwrap_or("(detached)");
                eprintln!("  {} ({})", wt.path.display(), branch);
            }
            bail!("please provide a more specific query");
        }
    }
}

/// Select a worktree via fzf interactive picker.
pub(crate) fn select_by_fzf<'a>(
    worktrees: &'a [git::Worktree],
) -> anyhow::Result<&'a git::Worktree> {
    if worktrees.is_empty() {
        bail!("no worktrees found");
    }

    let input: Vec<String> = worktrees
        .iter()
        .map(|wt| {
            let branch = wt.branch.as_deref().unwrap_or("(detached)");
            format!("{} ({})", wt.path.display(), branch)
        })
        .collect();
    let input_str = input.join("\n");

    let mut child = Command::new("fzf")
        .args(["--height", "40%", "--reverse"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to run fzf: {e}\nfzf is required for interactive selection"))?;

    if let Some(ref mut stdin) = child.stdin {
        stdin.write_all(input_str.as_bytes())?;
    }

    let output = child.wait_with_output()?;

    if !output.status.success() {
        bail!("fzf cancelled");
    }

    let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if selected.is_empty() {
        bail!("fzf cancelled");
    }

    let path_str = selected.split_once(" (").map(|(p, _)| p).unwrap_or(&selected);
    let path = PathBuf::from(path_str);

    worktrees
        .iter()
        .find(|wt| wt.path == path)
        .ok_or_else(|| anyhow::anyhow!("selected worktree not found"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_worktree(path: &str, branch: Option<&str>) -> git::Worktree {
        git::Worktree {
            path: std::path::PathBuf::from(path),
            branch: branch.map(|b| format!("refs/heads/{b}")),
            head: "abc123".to_string(),
        }
    }

    #[test]
    fn select_by_query_exact_path() {
        let wts = vec![
            make_worktree("/tmp/wt-feat", Some("feature")),
            make_worktree("/tmp/wt-bugfix", Some("bugfix")),
        ];
        let result = select_by_query(&wts, "feat").unwrap();
        assert_eq!(result.path, std::path::PathBuf::from("/tmp/wt-feat"));
    }

    #[test]
    fn select_by_query_branch_match() {
        let wts = vec![
            make_worktree("/tmp/wt-feat", Some("feature")),
            make_worktree("/tmp/wt-bugfix", Some("bugfix")),
        ];
        let result = select_by_query(&wts, "bugfix").unwrap();
        assert_eq!(result.path, std::path::PathBuf::from("/tmp/wt-bugfix"));
    }

    #[test]
    fn select_by_query_no_match() {
        let wts = vec![make_worktree("/tmp/wt-feat", Some("feature"))];
        let result = select_by_query(&wts, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn select_by_query_ambiguous() {
        let wts = vec![
            make_worktree("/tmp/wt-feat-a", Some("feature-a")),
            make_worktree("/tmp/wt-feat-b", Some("feature-b")),
        ];
        let result = select_by_query(&wts, "feat");
        assert!(result.is_err());
    }
}
