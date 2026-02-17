use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "clipm", version, about = "CLI clipboard manager for macOS")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Save current clipboard to history
    Store {
        /// Optional label for the entry
        #[arg(short, long)]
        label: Option<String>,
        /// Content type: text or password
        #[arg(short = 't', long = "type", default_value = "text")]
        content_type: String,
    },
    /// Copy entry to clipboard (default: most recent)
    Get {
        /// Entry ID (defaults to most recent)
        id: Option<i64>,
    },
    /// Show clipboard history as a table
    List {
        /// Maximum number of entries to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
        /// Number of entries to skip
        #[arg(short, long, default_value = "0")]
        offset: usize,
        /// Filter by label
        #[arg(short = 'L', long)]
        label: Option<String>,
        /// Filter to entries from the last N days
        #[arg(short, long)]
        days: Option<u32>,
        /// Filter by content type: text or password
        #[arg(short = 't', long = "type")]
        content_type: Option<String>,
    },
    /// Full-text search clipboard history
    Search {
        /// Search query
        query: String,
        /// Maximum number of results
        #[arg(short, long, default_value = "20")]
        limit: usize,
        /// Filter to entries from the last N days
        #[arg(short, long)]
        days: Option<u32>,
        /// Filter by content type: text or password
        #[arg(short = 't', long = "type")]
        content_type: Option<String>,
    },
    /// Add or update a label on an existing entry
    Label {
        /// Entry ID
        id: i64,
        /// Label text (omit to remove label)
        label: Option<String>,
    },
    /// Delete a single entry
    Delete {
        /// Entry ID to delete
        id: i64,
    },
    /// Clear all clipboard history
    Clear {
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}
