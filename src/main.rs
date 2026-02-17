mod cli;
mod clipboard;
mod commands;
mod db;
mod models;

use clap::Parser;
use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Store { label, content_type } => commands::store(label, &content_type),
        Command::Get { id } => commands::get(id),
        Command::List { limit, offset, label, days, content_type } => {
            commands::list(limit, offset, label.as_deref(), days, content_type.as_deref())
        }
        Command::Search { query, limit, days, content_type } => {
            commands::search(&query, limit, days, content_type.as_deref())
        }
        Command::Label { id, label } => commands::label(id, label),
        Command::Delete { id } => commands::delete(id),
        Command::Clear { force } => commands::clear(force),
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
