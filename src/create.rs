use crate::{base::Base, cd, config, git, hooks, paths};
use anyhow::Context;

pub fn run(name: &str, base: Base, force: bool) -> anyhow::Result<()> {
    // 1. Resolve source worktree root
    let source = git::top_level(None)
        .context("not in a git repo")?;

    // 2. Compute target path
    let target = paths::worktree_target(&source, name);

    // 3. Resolve base ref
    let base_ref = base.resolve(Some(&source))?;

    // 4. Create worktree
    git::worktree_add(&target, name, &base_ref, force, Some(&source))
        .context("failed to create worktree")?;

    // 5. Run on-create hooks
    let cfg = config::Config::load(&source)?;
    hooks::run(&source, &target, &cfg.create);

    // 6. Emit cd target
    cd::emit(&target)?;

    println!("created worktree '{}' at {}", name, target.display());
    Ok(())
}
