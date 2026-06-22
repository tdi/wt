use anyhow::Context;
use std::fs;
use std::path::{Path, PathBuf};

const WT_INIT_START: &str = "# >>> wt init >>>";
const WT_INIT_END: &str = "# <<< wt init <<<";

const WRAPPER: &str = "wt() {\n  local cdfile; cdfile=$(mktemp -t wt.cd)\n  WT_CD_FILE=$cdfile command wt \"$@\"\n  local rc=$?\n  [[ -s $cdfile ]] && cd \"$(cat \"$cdfile\")\"\n  rm -f \"$cdfile\"\n  return $rc\n}";

const FULL_BLOCK: &str = "# >>> wt init >>>\nwt() {\n  local cdfile; cdfile=$(mktemp -t wt.cd)\n  WT_CD_FILE=$cdfile command wt \"$@\"\n  local rc=$?\n  [[ -s $cdfile ]] && cd \"$(cat \"$cdfile\")\"\n  rm -f \"$cdfile\"\n  return $rc\n}\n# <<< wt init <<<";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shell {
    Zsh,
    Bash,
}

impl Shell {
    pub fn from_str(s: &str) -> Option<Shell> {
        match s {
            "zsh" => Some(Shell::Zsh),
            "bash" => Some(Shell::Bash),
            _ => None,
        }
    }
}

/// Detect the current shell from $SHELL.
pub fn detect_shell() -> Option<Shell> {
    std::env::var("SHELL")
        .ok()
        .and_then(|s| Shell::from_str(s.rsplit('/').next()?))
}

/// Get the default RC file for a shell.
pub fn default_rc(shell: Shell) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
    let rc_name = match shell {
        Shell::Zsh => ".zshrc",
        Shell::Bash => ".bashrc",
    };
    PathBuf::from(home).join(rc_name)
}

/// Install the wrapper into an RC file, idempotently.
pub fn install_to_rc(rc_file: &Path) -> anyhow::Result<()> {
    let content = if rc_file.exists() {
        fs::read_to_string(rc_file)
            .with_context(|| format!("failed to read {}", rc_file.display()))?
    } else {
        String::new()
    };

    let new_content = if let Some((start, end)) = find_markers(&content) {
        let before = &content[..start];
        let after = &content[end..];
        format!("{before}{FULL_BLOCK}{after}")
    } else {
        let prefix = if content.is_empty() { "" } else { "\n" };
        format!("{content}{prefix}{FULL_BLOCK}\n")
    };

    fs::write(rc_file, &new_content)
        .with_context(|| format!("failed to write {}", rc_file.display()))?;

    println!("installed wt() wrapper to {}", rc_file.display());
    Ok(())
}

/// Find the start and end positions of the marker block.
fn find_markers(content: &str) -> Option<(usize, usize)> {
    let start = content.find(WT_INIT_START)?;
    let end = content.find(WT_INIT_END)?;
    if end <= start {
        return None;
    }
    let end_pos = end + WT_INIT_END.len();
    let end_pos = if end_pos < content.len() && content.as_bytes()[end_pos] == b'\n' {
        end_pos + 1
    } else {
        end_pos
    };
    Some((start, end_pos))
}

pub fn run(
    shell: Option<&str>,
    rc_file: Option<&Path>,
    print_only: bool,
) -> anyhow::Result<()> {
    if print_only {
        print!("{WRAPPER}");
        return Ok(());
    }

    let shell = shell
        .and_then(Shell::from_str)
        .or_else(detect_shell)
        .context("cannot detect shell; pass --shell zsh or --shell bash")?;

    let rc = match rc_file {
        Some(p) => p.to_path_buf(),
        None => default_rc(shell),
    };

    install_to_rc(&rc)?;

    if cfg!(target_os = "macos") && shell == Shell::Bash && rc_file.is_none() {
        eprintln!(
            "note: macOS login shells source ~/.bash_profile, not ~/.bashrc.\n\
             Add `source ~/.bashrc` to your ~/.bash_profile to load the wt() wrapper."
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn shell_from_str() {
        assert_eq!(Shell::from_str("zsh"), Some(Shell::Zsh));
        assert_eq!(Shell::from_str("bash"), Some(Shell::Bash));
        assert_eq!(Shell::from_str("fish"), None);
    }

    #[test]
    fn find_markers_present() {
        let content = format!("some preamble\n{FULL_BLOCK}\nsome trailing\n");
        let result = find_markers(&content);
        assert!(result.is_some());
        let (start, end) = result.unwrap();
        assert!(start > 0);
        assert!(end > start);
    }

    #[test]
    fn find_markers_absent() {
        assert!(find_markers("no markers here").is_none());
    }

    #[test]
    fn install_new_file() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path();
        fs::remove_file(path).unwrap();
        install_to_rc(path).unwrap();
        let content = fs::read_to_string(path).unwrap();
        assert!(content.contains(WT_INIT_START));
        assert!(content.contains(WT_INIT_END));
        assert!(content.contains("WT_CD_FILE"));
    }

    #[test]
    fn install_existing_file_without_markers() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path();
        fs::write(path, "existing content\n").unwrap();
        install_to_rc(path).unwrap();
        let content = fs::read_to_string(path).unwrap();
        assert!(content.starts_with("existing content\n"));
        assert!(content.contains(WT_INIT_START));
    }

    #[test]
    fn install_replaces_existing_block() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path();
        let old = format!("before\n{FULL_BLOCK}\nafter\n");
        fs::write(path, &old).unwrap();
        install_to_rc(path).unwrap();
        let content = fs::read_to_string(path).unwrap();
        assert!(content.starts_with("before\n"));
        assert!(content.ends_with("after\n"));
        assert!(content.contains(WT_INIT_START));
        assert!(content.contains("WT_CD_FILE"));
        assert_eq!(content.matches(WT_INIT_START).count(), 1);
    }
}
