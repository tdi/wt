use std::path::{Path, PathBuf};

/// Strip the final "-<segment>" from a repo basename.
pub fn strip_suffix(repo: &str) -> &str {
    match repo.rsplit_once('-') {
        Some((head, _)) => head,
        None => repo,
    }
}

/// Compose the worktree target path.
pub fn worktree_target(source_root: &Path, name: &str) -> PathBuf {
    let parent = source_root.parent().expect("source root has no parent");
    let repo = source_root
        .file_name()
        .and_then(|s| s.to_str())
        .expect("source root has no filename");
    let stripped = strip_suffix(repo);
    parent.join(format!("{}-{}", stripped, name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_suffix_strips_last_segment() {
        assert_eq!(strip_suffix("portal-api"), "portal");
    }

    #[test]
    fn strip_suffix_no_hyphen() {
        assert_eq!(strip_suffix("infra"), "infra");
    }

    #[test]
    fn strip_suffix_multiple_hyphens() {
        assert_eq!(strip_suffix("a-b-c"), "a-b");
    }

    #[test]
    fn strip_suffix_single_segment_after_hyphen() {
        assert_eq!(strip_suffix("my-repo"), "my");
    }

    #[test]
    fn worktree_target_basic() {
        let root = Path::new("/Users/darek/Projects/portal-api");
        let target = worktree_target(root, "feat-auth");
        assert_eq!(target, PathBuf::from("/Users/darek/Projects/portal-feat-auth"));
    }

    #[test]
    fn worktree_target_no_suffix_strip() {
        let root = Path::new("/Users/darek/Projects/infra");
        let target = worktree_target(root, "tweak");
        assert_eq!(target, PathBuf::from("/Users/darek/Projects/infra-tweak"));
    }

    #[test]
    fn worktree_target_multiple_hyphens() {
        let root = Path::new("/Users/darek/Projects/a-b-c");
        let target = worktree_target(root, "feat");
        assert_eq!(target, PathBuf::from("/Users/darek/Projects/a-b-feat"));
    }
}
