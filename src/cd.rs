use std::path::Path;

/// Emit the target path to $WT_CD_FILE if the env var is set.
/// Leaves the file empty on error (caller decides whether to cd).
pub fn emit(path: &Path) -> anyhow::Result<()> {
    if let Ok(f) = std::env::var("WT_CD_FILE") {
        let p = Path::new(&f);
        std::fs::write(p, path.to_string_lossy().as_bytes())
            .map_err(|e| anyhow::anyhow!("failed to write {}: {}", f, e))?;
    }
    Ok(())
}

/// Check if $WT_CD_FILE is set (for testing / command dispatch).
pub fn is_enabled() -> bool {
    std::env::var("WT_CD_FILE").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn emit_writes_path_to_file() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path();
        unsafe { std::env::set_var("WT_CD_FILE", path) };
        let target = Path::new("/Users/darek/Projects/portal-feat-auth");
        emit(target).unwrap();
        let contents = std::fs::read_to_string(path).unwrap();
        assert_eq!(contents, "/Users/darek/Projects/portal-feat-auth");
        unsafe { std::env::remove_var("WT_CD_FILE") };
    }

    #[test]
    fn emit_noop_without_env() {
        unsafe { std::env::remove_var("WT_CD_FILE") };
        let result = emit(Path::new("/tmp/test"));
        assert!(result.is_ok());
    }

    #[test]
    fn is_enabled_true_when_set() {
        unsafe { std::env::set_var("WT_CD_FILE", "/tmp/test") };
        assert!(is_enabled());
        unsafe { std::env::remove_var("WT_CD_FILE") };
    }

    #[test]
    fn is_enabled_false_when_unset() {
        unsafe { std::env::remove_var("WT_CD_FILE") };
        assert!(!is_enabled());
    }
}
