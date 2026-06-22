use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn make_repo() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    let p = dir.path();
    git(p, &["init"]);
    git(p, &["config", "user.email", "test@test.com"]);
    git(p, &["config", "user.name", "Test"]);
    fs::write(p.join("README.md"), "# test").unwrap();
    git(p, &["add", "."]);
    git(p, &["commit", "-m", "init"]);
    dir
}

pub fn git(cwd: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {} failed", args.join(" "));
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

pub fn wt(args: &[&str]) -> assert_cmd::Command {
    let mut cmd = assert_cmd::Command::cargo_bin("wt").unwrap();
    cmd.args(args);
    cmd
}

pub fn wt_fail(args: &[&str]) -> assert_cmd::Command {
    let mut cmd = assert_cmd::Command::cargo_bin("wt").unwrap();
    cmd.args(args);
    cmd
}

pub fn expected_wt_path(repo: &Path, name: &str) -> PathBuf {
    let parent = repo.parent().unwrap();
    let repo_name = repo.file_name().unwrap().to_str().unwrap();
    parent.join(format!("{}-{}", repo_name, name))
}