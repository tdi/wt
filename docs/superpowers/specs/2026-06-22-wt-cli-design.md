# `wt` — worktree manager (design)

**Date:** 2026-06-22
**Status:** Approved

## Overview

A single Rust binary `wt` that manages git worktrees, replacing the existing
`wt` / `wtm` / `wtp` / `wtr` / `wtl` / `cdtw` zsh functions in `~/.zshrc`. A
thin `wt()` shell wrapper provides automatic `cd` into created or selected
worktrees (a child process cannot `cd` its parent shell on its own).

## Goals

- One unified CLI surface for all worktree operations.
- Replicate the existing naming/placement convention exactly so behavior is
  unchanged for the user's current repos.
- Make on-create setup (copy `.env`, `mise trust`, …) declarative via a
  repo-level config file instead of hardcoded per-function.
- Keep the automatic `cd` UX the shell functions provided.

## Non-goals (out of scope)

- Global / per-user config (`~/.config/wt`).
- Named hook profiles.
- Customizable naming or path-placement templates (convention is hardcoded;
  revisit only if a repo needs otherwise).
- `git2` FFI — we shell out to `git` to match current behavior and keep the
  build simple.

## Commands

| Command | Description | Replaces |
|---|---|---|
| `wt create <name> [-r\|-m\|-c] [-f]` | Create a worktree. Default: fetch and base off `origin/<default>`. | `wt`, `wtm` |
| `wt rm [<query>] [-f]` | Remove a worktree. No arg → interactive fzf picker. With arg → match by name/path substring. `-f` forces removal of dirty worktrees. | `wtr` |
| `wt ls` | List worktrees (current marked). | `wtl` |
| `wt go [<query>]` | Cd into a worktree by partial name/path match. No arg → fzf picker. | `wtp` |
| `wt top` | Cd into the main / top worktree (the common dir's worktree). | `cdtw` |

### `wt create <name>`

**Arguments and flags**

- `<name>` (required): the new branch name and worktree suffix.
- `-r, --remote-main` (default): base the new branch on the remote default
  branch.
- `-m, --local-main`: base the new branch on the local default branch.
- `-c, --current`: base the new branch on the branch currently checked out in
  the invoking worktree.
- `-f, --force`: pass through to `git worktree add --force`.

Only one base flag may be supplied; supplying more than one is an error.

**Behavior**

1. Resolve the source worktree root via `git rev-parse --show-toplevel`.
2. Compute the worktree name and target path (see "Placement & naming").
3. Resolve the base ref:
   - `--remote-main`: detect remote default branch via
     `git symbolic-ref --short refs/remotes/origin/HEAD`; fall back to parsing
     `git remote show origin`. Run `git fetch origin <default>`; verify
     `refs/remotes/origin/<default>` exists. Base on `origin/<default>`.
   - `--local-main`: resolve the common dir (`git rev-parse --git-common-dir`),
     read its `HEAD` (`git --git-dir=<common> symbolic-ref --short HEAD`); if
     detached, fall back to trying `main` then `master`. Base on that local
     ref.
   - `--current`: base on `git rev-parse --abbrev-ref HEAD` of the invoking
     worktree.
4. `git worktree add "<target>" -b "<name>" "<base>"`.
5. Run on-create hooks (see ".wt.toml").
6. Emit the target path for the shell wrapper (see "Shell integration").

### `wt rm [<query>] [-f]`

- No `<query>`: list non-main worktrees via fzf; remove the selection.
- With `<query>`: select the worktree whose path or branch contains the query
  (substring, case-insensitive). If multiple match, refuse and print the
  matches; require a more specific query.
- `-f`: pass `--force` to `git worktree remove`.
- Never remove the main (common-dir) worktree; refuse with an error.

### `wt ls`

- `git worktree list --porcelain`, rendered as a readable table. Mark the
  current worktree (the one whose path equals `git rev-parse --show-toplevel`)
  with `*`.

### `wt go [<query>]`

- No `<query>`: fzf over all worktrees; emit the selection's path.
- With `<query>`: match by substring (case-insensitive) on path or branch.
  Ambiguous → error listing matches. Unambiguous → emit the path.
- Pure path emitter; the shell wrapper performs the actual `cd`.

### `wt top`

- Resolve the common dir, derive its worktree root (the dir containing
  `.git`), emit it. The shell wrapper cds.

## Placement & naming

Unchanged from the current zsh functions:

- Source root: `S = $(git rev-parse --show-toplevel)`
- Parent: `P = dirname S`
- Repo basename: `B = basename S`
- Stripped repo: `R = B` with the final `-<segment>` removed
  (`portal-api` → `portal`). If `B` contains no `-`, `R = B`.
- Target: `P/R-<name>`

Examples:
- `/Users/darek/Projects/portal-api` + name `feat-auth`
  → `/Users/darek/Projects/portal-feat-auth`
- `/Users/darek/Projects/infra` + name `tweak`
  → `/Users/darek/Projects/infra-tweak`

## On-create hooks: `.wt.toml`

A TOML file at the **source worktree root** (the main worktree). Committed to
the repo so the team shares the same setup.

```toml
# .wt.toml
[create]
# Globs relative to the source worktree root. Matched files are copied
# into the new worktree root. Patterns use gitignore-style globbing via
# the `glob` crate. Hidden files (e.g. .env) are matched.
copy = [".env", ".env.local", ".envrc"]

# Commands run in the new worktree root after copying, sequentially.
# Each entry is run via `sh -c`. A non-zero exit is reported as a warning
# but does NOT abort: the worktree is created and the shell still cds.
run = ["mise trust"]
```

Semantics:
- Missing `.wt.toml` → no hooks run (the worktree is still created normally).
- `copy` globs are expanded against the source root. Files that match are
  copied, preserving the relative path within the root. No-op if a pattern
  matches nothing.
- `run` commands execute with cwd = new worktree root. `WT_NEW_WORKTREE` and
  `WT_SOURCE_ROOT` env vars are set for the command's use.
- Failures in `run` print a warning to stderr but do not change the exit code
  of `wt create` (the worktree itself succeeded).

## Shell integration

Replaces the six existing functions in `~/.zshrc` with a single `wt()`:

```sh
# wt — worktree manager wrapper (cd support)
wt() {
  local cdfile; cdfile=$(mktemp -t wt.cd)
  WT_CD_FILE=$cdfile command wt "$@"
  local rc=$?
  [[ -s $cdfile ]] && cd "$(cat "$cdfile")"
  rm -f "$cdfile"
  return $rc
}
```

The binary checks `$WT_CD_FILE`. For `create`, `go`, and `top` on success it
writes exactly one absolute path (trailing newline) to that file. For all
other commands, or on failure, it leaves the file empty. All human-facing
output (progress, errors, lists) goes to stdout/stderr as normal — the temp
file is a side channel only.

## Architecture

```
src/
  main.rs        clap arg parsing, command dispatch
  git.rs         git shell-out helpers (rev-parse, worktree add/remove/list,
                 symbolic-ref, fetch, common-dir)
  paths.rs       naming/placement logic (strip suffix, target path)
  base.rs        base-ref resolution (remote-main / local-main / current)
  config.rs      .wt.toml load + parse (toml crate)
  hooks.rs       run copy + run hooks for `create`
  create.rs      create command
  remove.rs      rm command (fzf selection)
  list.rs        ls command
  go.rs          go + top commands (path emission)
  cd.rs          WT_CD_FILE emission helper
  error.rs       error type (anyhow or thiserror)
```

### Dependencies (Cargo.toml)

- `clap` (derive) — present.
- `toml` — config parsing.
- `glob` — `.wt.toml` copy patterns.
- `anyhow` — error handling (or `thiserror` if structured errors preferred).
- `tempfile` — only if we decide not to use `mktemp -t` from the shell; the
  shell wrapper owns the temp file, so this is likely unnecessary.

### External tool dependencies (runtime)

- `git` (required).
- `fzf` (required for interactive `wt rm` and `wt go` with no query; the
  non-interactive path works without it).
- `sh -c` (for `run` hooks; standard).

## Error handling

- Not in a git repo → clear error, exit 1.
- Remote default branch cannot be detected (for `--remote-main`) → error with
  guidance to use `-m` or `-c`, exit 1.
- Worktree add fails (branch exists, path exists, etc.) → surface git's error,
  exit 1.
- `.wt.toml` present but malformed → error pointing at the file, exit 1 (do
  not create the worktree in a half-configured state).
- `run` hook failures → warning on stderr, exit 0 (worktree succeeded).

## Testing

- Unit tests for pure logic:
  - `paths`: suffix stripping (`portal-api` → `portal`, `infra` → `infra`,
    `a-b-c` → `a-b`), target path composition.
  - `config`: parse a sample `.wt.toml`; missing file → default empty config;
    malformed → error.
  - `base`: local-main ref resolution logic (mocked git output).
- Integration tests (shell out to a temp git repo created in `tempfile`):
  - `create --current` produces a sibling worktree dir with the right name.
  - `create --remote-main` fetches and bases correctly (using a local "remote"
    repo fixture).
  - `.wt.toml` `copy` copies matching files; `run` executes and is observable
    via a sentinel file the command writes.
  - `rm` removes the worktree; refuses the main worktree.
- CLI smoke tests via `assert_cmd` + `predicates` (optional; add if low-friction).

## Migration

1. Build and install the binary (`cargo install --path .` or symlink target).
2. Add the `wt()` wrapper to `~/.zshrc`.
3. Remove the old `wt`, `wtm`, `wtp`, `wtr`, `wtl`, `cdtw` functions.
4. Add a `.wt.toml` to any repo where you want the automatic `.env` copy and
   `mise trust`.

## Open questions

None at design time. The chosen defaults (`--remote-main` default, repo-level
`.wt.toml`, `WT_CD_FILE` cd mechanism) were confirmed during brainstorming.
