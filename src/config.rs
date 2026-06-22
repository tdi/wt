use anyhow::Context;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Default, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub create: CreateConfig,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct CreateConfig {
    #[serde(default)]
    pub copy: Vec<String>,
    #[serde(default)]
    pub run: Vec<String>,
}

impl Config {
    pub fn load(root: &Path) -> anyhow::Result<Config> {
        let path = root.join(".wt.toml");
        if !path.exists() {
            return Ok(Config::default());
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let cfg: Config = toml::from_str(&text)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(cfg)
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
