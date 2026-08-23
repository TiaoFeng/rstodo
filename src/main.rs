mod commands;
mod error;
mod io;
mod task;
mod time;

use clap::{Parser, Subcommand};
use io::storage::TaskStore;
use std::error::Error;

use crate::commands::SortBy;
use crate::task::Priority;

#[derive(Parser)]
struct Cli {
    #[arg(long, global = true)]
    file: Option<String>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Add {
        content: String,
        #[arg(short = 'D', long)]
        description: Option<String>,
        #[arg(short, long)]
        deadline: Option<String>,
        #[arg(short, long)]
        priority: Option<Priority>,
    },
    Change {
        no: usize,
        #[arg(short, long)]
        content: Option<String>,
        #[arg(short = 'D', long)]
        description: Option<Option<String>>,
        #[arg(short, long)]
        deadline: Option<Option<String>>,
        #[arg(short, long)]
        priority: Option<Option<Priority>>,
    },
    List {
        sort: Option<SortBy>,
    },
    Show {
        no: usize,
    },
    Done {
        no: usize,
    },
    Undone {
        no: usize,
    },
    Delete {
        no: usize,
    },
}

fn main() {
    let cli = Cli::parse();
    let store = TaskStore::new(cli.file);
    let result = match cli.command {
        Commands::Add {
            content,
            description,
            deadline,
            priority,
        } => commands::add_task(content, &store, description, deadline, priority),
        Commands::Change {
            no,
            content,
            description,
            deadline,
            priority,
        } => commands::change_task(no, &store, content, description, deadline, priority),
        Commands::List { sort } => commands::list_task(&store, sort),
        Commands::Show { no } => commands::show_details(no, &store),
        Commands::Done { no } => commands::complete_task(no, &store),
        Commands::Undone { no } => commands::incomplete_task(no, &store),
        Commands::Delete { no } => commands::delete_task(no, &store),
    };
    if let Err(apperr) = result {
        eprintln!("Error: {}", apperr);
        let mut source = apperr.source();
        while let Some(src) = source {
            eprintln!("Caused by: {}", src);
            source = src.source();
        }
        std::process::exit(1);
    }
}
