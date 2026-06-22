mod common;

use common::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn mk_current_branch() {
    let repo = make_repo();
    let rp = repo_path(&repo);

    wt(&["mk", "feat", "-c"]).current_dir(&rp).assert().success();

    let wt_path = expected_wt_path(&rp, "feat");
    assert!(wt_path.exists());
    assert!(wt_path.join("README.md").exists());
}

#[test]
fn mk_remote_main_fails_without_remote() {
    let repo = make_repo();
    let rp = repo_path(&repo);

    wt_fail(&["mk", "feat"]).current_dir(&rp).assert().failure();
}

#[test]
fn mk_local_main() {
    let repo = make_repo();
    let rp = repo_path(&repo);

    wt(&["mk", "feat", "-l"]).current_dir(&rp).assert().success();

    let wt_path = expected_wt_path(&rp, "feat");
    assert!(wt_path.exists());
}

#[test]
fn mk_with_wt_toml_copies_env() {
    let repo = make_repo();
    let rp = repo_path(&repo);

    fs::write(
        rp.join(".wt.toml"),
        "[create]\ncopy = [\".env\", \".env.local\"]",
    )
    .unwrap();
    fs::write(rp.join(".env"), "KEY=value").unwrap();
    fs::write(rp.join(".env.local"), "LOCAL=1").unwrap();

    wt(&["mk", "feat", "-l"]).current_dir(&rp).assert().success();

    let wt_path = expected_wt_path(&rp, "feat");
    assert_eq!(
        fs::read_to_string(wt_path.join(".env")).unwrap(),
        "KEY=value"
    );
    assert_eq!(
        fs::read_to_string(wt_path.join(".env.local")).unwrap(),
        "LOCAL=1"
    );
}

#[test]
fn mk_with_wt_toml_run_hook() {
    let repo = make_repo();
    let rp = repo_path(&repo);

    fs::write(
        rp.join(".wt.toml"),
        "[create]\nrun = [\"touch hook-ran\"]",
    )
    .unwrap();

    wt(&["mk", "feat", "-l"]).current_dir(&rp).assert().success();

    let wt_path = expected_wt_path(&rp, "feat");
    assert!(wt_path.join("hook-ran").exists());
}

#[test]
fn ls_shows_worktrees() {
    let repo = make_repo();
    let rp = repo_path(&repo);

    wt(&["mk", "feat", "-l"]).current_dir(&rp).assert().success();

    let output = wt(&["ls"]).current_dir(&rp).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let wt_path = expected_wt_path(&rp, "feat");
    assert!(stdout.contains(wt_path.to_str().unwrap()));
}

#[test]
fn rm_removes_worktree() {
    let repo = make_repo();
    let rp = repo_path(&repo);

    wt(&["mk", "feat", "-l"]).current_dir(&rp).assert().success();
    wt(&["rm", "feat"]).current_dir(&rp).assert().success();

    let wt_path = expected_wt_path(&rp, "feat");
    assert!(!wt_path.exists());
}

#[test]
fn rm_refuses_main_worktree() {
    let repo = make_repo();
    let rp = repo_path(&repo);
    wt_fail(&["rm", "."]).current_dir(&rp).assert().failure();
}

#[test]
fn go_query_resolves() {
    let repo = make_repo();
    let rp = repo_path(&repo);

    wt(&["mk", "feat", "-l"]).current_dir(&rp).assert().success();

    let cdfile = tempfile::NamedTempFile::new().unwrap();
    unsafe { std::env::set_var("WT_CD_FILE", cdfile.path()); }

    wt(&["go", "feat"]).current_dir(&rp).assert().success();

    let content = fs::read_to_string(cdfile.path()).unwrap();
    let wt_path = expected_wt_path(&rp, "feat");
    assert!(content.contains(wt_path.to_str().unwrap()));

    unsafe { std::env::remove_var("WT_CD_FILE"); }
}

#[test]
fn shell_init_print() {
    let output = wt(&["shell-init", "--print"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("wt()"));
    assert!(stdout.contains("WT_CD_FILE"));
}

#[test]
fn shell_init_install_and_replace() {
    let tmp = TempDir::new().unwrap();
    let rc = tmp.path().join(".zshrc");

    wt(&["shell-init", "--shell", "zsh", "--rc-file", rc.to_str().unwrap()])
        .assert().success();

    let content1 = fs::read_to_string(&rc).unwrap();
    assert!(content1.contains("# >>> wt init >>>"));
    assert!(content1.contains("# <<< wt init <<<"));

    wt(&["shell-init", "--shell", "zsh", "--rc-file", rc.to_str().unwrap()])
        .assert().success();

    let content2 = fs::read_to_string(&rc).unwrap();
    assert_eq!(content2.matches("# >>> wt init >>>").count(), 1);
}
