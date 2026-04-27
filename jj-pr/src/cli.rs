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
    },
    /// Create a new PR or update an existing one
    Track(TrackArgs),
    /// Import existing GitHub PRs by stamping PR trailers on local commits
    Import {
        /// Show what would be done without doing it
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(clap::Args, Clone)]
pub struct TrackArgs {
    /// Existing PR number to update (omit to create a new PR)
    #[arg(long, conflicts_with = "bookmark")]
    pub pr: Option<u64>,

    /// Bookmark name (creates one if not specified; incompatible with --pr)
    #[arg(short, long)]
    pub bookmark: Option<String>,

    /// Revision (default: @ or bookmark target if -b is set)
    #[arg(short, long)]
    pub revision: Option<String>,

    /// PR title (used when creating)
    #[arg(short, long)]
    pub title: Option<String>,

    /// PR description/body (used when creating)
    #[arg(long)]
    pub body: Option<String>,
}
