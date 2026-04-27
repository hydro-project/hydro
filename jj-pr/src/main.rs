mod cli;
mod gh;
mod jj;
mod pr_dag;
mod style;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Log => cmd_log(),
        Command::Sync { dry_run } => cmd_sync(dry_run),
        Command::Create(args) => cmd_create(args),
        Command::Import { dry_run } => cmd_import(dry_run),
    }
}

fn cmd_log() -> Result<()> {
    let jj_state = jj::load_state()?;
    let gh_state = gh::load_prs()?;
    let dag = pr_dag::build(&jj_state, &gh_state)?;
    pr_dag::render_log(&dag)?;
    Ok(())
}

fn cmd_sync(dry_run: bool) -> Result<()> {
    let jj_state = jj::load_state()?;
    let gh_state = gh::load_prs()?;
    let dag = pr_dag::build(&jj_state, &gh_state)?;
    let actions = pr_dag::plan_sync(&dag, &jj_state, &gh_state)?;

    if actions.is_empty() {
        eprintln!("Nothing to sync.");
        return Ok(());
    }

    for action in &actions {
        eprintln!("{action}");
    }

    if !dry_run {
        pr_dag::execute_sync(&actions)?;
    } else {
        eprintln!("\n{}", style::warn("Dry run: no changes applied."));
    }

    Ok(())
}

fn cmd_create(args: cli::CreateArgs) -> Result<()> {
    let jj_state = jj::load_state()?;
    let gh_state = gh::load_prs()?;
    let dag = pr_dag::build(&jj_state, &gh_state)?;
    pr_dag::create_pr(&dag, &jj_state, &gh_state, &args)?;
    Ok(())
}

fn cmd_import(dry_run: bool) -> Result<()> {
    let jj_state = jj::load_state()?;
    let gh_prs = gh::load_prs()?;
    pr_dag::import_prs(&jj_state, &gh_prs, dry_run)?;
    Ok(())
}
