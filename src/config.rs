use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub create: CreateConfig,
    #[serde(default)]
    pub worktree: WorktreeConfig,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct CreateConfig {
    #[serde(default)]
    pub copy: Vec<String>,
    #[serde(default)]
    pub run: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorktreeConfig {
    /// Prefix prepended to worktree name (e.g. "wt-")
    #[serde(default)]
    pub prefix: String,
    /// Directory for worktrees, relative to repo parent (default: "../")
    #[serde(default = "default_dir")]
    pub dir: String,
}

impl Default for WorktreeConfig {
    fn default() -> Self {
        WorktreeConfig {
            prefix: String::new(),
            dir: default_dir(),
        }
    }
}

fn default_dir() -> String {
    "../".to_string()
}

impl Config {
    pub fn load(source_root: &Path) -> anyhow::Result<Config> {
        // Load global config first
        let global = load_global();
        // Load repo config
        let repo = load_repo(source_root)?;
        // Merge: repo overrides global
        Ok(merge(global, repo))
    }
}

fn load_global() -> Config {
    let home = std::env::var("HOME").ok();
    let path = home.map(|h| PathBuf::from(h).join(".config/wt/config.toml"));
    let path = match path {
        Some(p) if p.exists() => p,
        _ => return Config::default(),
    };
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return Config::default(),
    };
    toml::from_str(&text).unwrap_or_default()
}

fn load_repo(source_root: &Path) -> anyhow::Result<Option<Config>> {
    let path = source_root.join(".wt.toml");
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", path.display()))?;
    let cfg: Config = toml::from_str(&text)
        .map_err(|e| anyhow::anyhow!("failed to parse {}: {e}", path.display()))?;
    Ok(Some(cfg))
}

fn merge(mut base: Config, override_cfg: Option<Config>) -> Config {
    match override_cfg {
        Some(oc) => {
            // Worktree config: repo overrides global field-by-field
            if !oc.worktree.prefix.is_empty() || oc.worktree.dir != "../" {
                if !oc.worktree.prefix.is_empty() {
                    base.worktree.prefix = oc.worktree.prefix;
                }
                if oc.worktree.dir != "../" {
                    base.worktree.dir = oc.worktree.dir;
                }
            }
            // Create config: repo replaces global entirely if present
            if !oc.create.copy.is_empty() || !oc.create.run.is_empty() {
                base.create = oc.create;
            }
            base
        }
        None => base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn load_missing_file_returns_default() {
        let dir = TempDir::new().unwrap();
        let cfg = Config::load(dir.path()).unwrap();
        assert!(cfg.create.copy.is_empty());
        assert!(cfg.create.run.is_empty());
    }

    #[test]
    fn load_valid_config() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(".wt.toml"),
            r#"
[create]
copy = [".env", ".envrc"]
run = ["mise trust"]
"#,
        )
        .unwrap();
        let cfg = Config::load(dir.path()).unwrap();
        assert_eq!(cfg.create.copy, vec![".env", ".envrc"]);
        assert_eq!(cfg.create.run, vec!["mise trust"]);
    }

    #[test]
    fn load_empty_create_section() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".wt.toml"), "").unwrap();
        let cfg = Config::load(dir.path()).unwrap();
        assert!(cfg.create.copy.is_empty());
        assert!(cfg.create.run.is_empty());
    }

    #[test]
    fn load_malformed_returns_error() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(".wt.toml"),
            "[create\ncopy = not valid toml",
        )
        .unwrap();
        let result = Config::load(dir.path());
        assert!(result.is_err());
    }
}
