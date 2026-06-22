# wt

A fast, opinionated git worktree manager CLI.

> AI-generated and highly opinionated. 

## Install

```bash
cargo install wt
wt shell-init    # adds the wt() wrapper to your shell
```

## Usage

```bash
wt mk feat-auth         # create worktree from remote main (default)
wt mk feat-auth -c      # create from current branch
wt mk feat-auth -m      # create from local main
wt rm feat-auth         # remove worktree (or fzf picker with no arg)
wt ls                   # list worktrees (current marked with *)
wt go feat              # cd into worktree by name (or fzf picker with no arg)
wt top                  # cd into main worktree
```

## How it works

Worktrees are placed in `../` (sibling to your repo) with the naming convention:

```
portal-api + feat-auth  →  ../portal-feat-auth
```

The last `-segment` of the repo name is stripped, then your branch name is appended.

### Config (`.wt.toml` in repo root)

```toml
[worktree]
prefix = "wt-"              # prepend to worktree name
dir = "../"                 # placement directory (relative to repo parent)

[create]
copy = [".env", ".envrc"]   # files to copy into new worktree
run = ["mise trust"]        # commands to run after creation
```

### Global config (`~/.config/wt/config.toml`)

Same format. Repo config overrides global, field by field.

## Shell wrapper

The `wt()` shell function wraps the binary to enable automatic `cd` into worktrees. Install it with `wt shell-init`. Remove your old `wt`/`wtm`/`wtp`/`wtr`/`wtl`/`cdtw` shell functions — `wt` replaces them all.

## License

MIT
