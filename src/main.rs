use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod paths;
mod config;
mod cd;
mod git;
mod base;
mod hooks;
mod create;
mod list;
mod go;
mod remove;
mod shell_init;

#[derive(Parser)]
#[command(version, about = "Worktree manager", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a worktree
    Create {
        /// New branch / worktree name
        name: String,
        /// Base on origin/default branch (default)
        #[arg(short, long, conflicts_with_all = &["local_main", "current"])]
        remote_main: bool,
        /// Base on local default branch
        #[arg(short, long, conflicts_with_all = &["remote_main", "current"])]
        local_main: bool,
        /// Base on current branch
        #[arg(short, long, conflicts_with_all = &["remote_main", "local_main"])]
        current: bool,
        /// Force creation
        #[arg(short, long)]
        force: bool,
    },
    /// Remove a worktree
    Rm {
        /// Name or path substring to match
        query: Option<String>,
        /// Force removal
        #[arg(short, long)]
        force: bool,
    },
    /// List worktrees
    Ls,
    /// Cd into a worktree by name or fzf picker
    Go {
        /// Name or path substring to match
        query: Option<String>,
    },
    /// Cd into the main / top worktree
    Top,
    /// Install the wt() shell wrapper
    ShellInit {
        /// Shell to install for (zsh or bash)
        #[arg(long)]
        shell: Option<String>,
        /// RC file path to write to
        #[arg(long)]
        rc_file: Option<PathBuf>,
        /// Print wrapper to stdout instead of writing
        #[arg(long)]
        print: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create { name, remote_main, local_main, current, force } => {
            let base = base::resolve_flag(remote_main, local_main, current)?;
            create::run(&name, base, force)?;
        }
        Commands::Rm { query, force } => remove::run(query.as_deref(), force)?,
        Commands::Ls => list::run()?,
        Commands::Go { query } => go::run(query.as_deref())?,
        Commands::Top => go::run_top()?,
        Commands::ShellInit { shell, rc_file, print } => {
            shell_init::run(shell.as_deref(), rc_file.as_deref(), print)?;
        }
    }

    Ok(())
}
