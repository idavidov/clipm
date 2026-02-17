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
        Command::Store { label } => commands::store(label),
        Command::Get { id } => commands::get(id),
        Command::List { limit, offset, label, days } => commands::list(limit, offset, label.as_deref(), days),
        Command::Search { query, limit, days } => commands::search(&query, limit, days),
        Command::Label { id, label } => commands::label(id, label),
        Command::Delete { id } => commands::delete(id),
        Command::Clear { force } => commands::clear(force),
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
