use crate::config::CreateConfig;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Run on-create hooks: copy files, then run commands.
pub fn run(source_root: &Path, new_root: &Path, cfg: &CreateConfig) {
    copy_files(source_root, new_root, &cfg.copy);
    run_commands(new_root, source_root, &cfg.run);
}

fn copy_files(source_root: &Path, new_root: &Path, patterns: &[String]) {
    for pattern in patterns {
        let full_pattern = source_root.join(pattern);
        let pattern_str = match full_pattern.to_str() {
            Some(s) => s,
            None => continue,
        };
        match glob::glob(pattern_str) {
            Ok(paths) => {
                for entry in paths {
                    match entry {
                        Ok(p) => {
                            if let Ok(rel) = p.strip_prefix(source_root) {
                                let dest = new_root.join(rel);
                                if let Some(parent) = dest.parent() {
                                    let _ = fs::create_dir_all(parent);
                                }
                                if let Err(e) = fs::copy(&p, &dest) {
                                    eprintln!("warning: failed to copy {}: {e}", p.display());
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("warning: glob error for pattern '{pattern}': {e}");
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("warning: invalid glob pattern '{pattern}': {e}");
            }
        }
    }
}

fn run_commands(new_root: &Path, source_root: &Path, commands: &[String]) {
    for cmd in commands {
        let status = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(new_root)
            .env("WT_NEW_WORKTREE", new_root)
            .env("WT_SOURCE_ROOT", source_root)
            .status();

        match status {
            Ok(s) if !s.success() => {
                eprintln!("warning: hook '{cmd}' exited with {s}");
            }
            Err(e) => {
                eprintln!("warning: failed to run hook '{cmd}': {e}");
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn copy_files_matches_and_copies() {
        let source = TempDir::new().unwrap();
        let dest = TempDir::new().unwrap();
        fs::write(source.path().join(".env"), "KEY=value").unwrap();
        fs::write(source.path().join(".env.local"), "LOCAL=1").unwrap();
        fs::write(source.path().join("README.md"), "skip").unwrap();

        let patterns = vec![".env".to_string(), ".env.local".to_string()];
        copy_files(source.path(), dest.path(), &patterns);

        assert_eq!(
            fs::read_to_string(dest.path().join(".env")).unwrap(),
            "KEY=value"
        );
        assert_eq!(
            fs::read_to_string(dest.path().join(".env.local")).unwrap(),
            "LOCAL=1"
        );
        assert!(!dest.path().join("README.md").exists());
    }

    #[test]
    fn copy_files_no_match_is_noop() {
        let source = TempDir::new().unwrap();
        let dest = TempDir::new().unwrap();
        fs::write(source.path().join("README.md"), "test").unwrap();

        let patterns = vec![".env".to_string()];
        copy_files(source.path(), dest.path(), &patterns);

        assert!(dest.path().read_dir().unwrap().next().is_none());
    }

    #[test]
    fn copy_files_preserves_relative_path() {
        let source = TempDir::new().unwrap();
        let dest = TempDir::new().unwrap();
        fs::create_dir_all(source.path().join("sub")).unwrap();
        fs::write(source.path().join("sub/.env"), "KEY=val").unwrap();

        let patterns = vec!["sub/.env".to_string()];
        copy_files(source.path(), dest.path(), &patterns);

        assert_eq!(
            fs::read_to_string(dest.path().join("sub/.env")).unwrap(),
            "KEY=val"
        );
    }
}
