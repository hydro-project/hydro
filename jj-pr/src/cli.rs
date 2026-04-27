use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "jj-pr", about = "Sync jj bookmarks with GitHub PRs")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Display PRs as a graph
    Log,
    /// Reconcile local jj state with GitHub
    Sync {
        /// Show what would be done without doing it
        #[arg(long)]
        dry_run: bool,
        /// Show detailed output
        #[arg(long)]
        verbose: bool,
    },
    /// Create a new PR from a bookmark
    Create(CreateArgs),
    /// Import existing GitHub PRs by stamping PR trailers on local commits
    Import {
        /// Show what would be done without doing it
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(clap::Args, Clone)]
pub struct CreateArgs {
    /// Bookmark name (creates one if not specified)
    #[arg(short, long)]
    pub bookmark: Option<String>,

    /// Revision to create the PR from (default: @ or bookmark target if -b is set)
    #[arg(short, long)]
    pub revision: Option<String>,

    /// PR title
    #[arg(short, long)]
    pub title: Option<String>,

    /// PR description/body
    #[arg(long)]
    pub body: Option<String>,
}
