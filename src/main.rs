//! 程序入口
//!
//! 使用clap实现终端命令输入
mod commands;
mod error;
mod io;
mod task;
#[cfg(test)]
mod test_helpers;
mod time;
mod todo;

use clap::{Parser, Subcommand};
use io::storage::TaskStore;
use std::error::Error;

use crate::task::Priority;
use crate::todo::SortBy;

/// Welcome to rstodo.
/// Please enter a command to create or manage your tasks.
#[derive(Parser)]
struct Cli {
    #[arg(long, global = true)]
    file: Option<String>,
    #[command(subcommand)]
    command: Commands,
}

/// 子命令结构体
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
    Status,
    Done {
        nos: Vec<usize>,
    },
    Undone {
        nos: Vec<usize>,
    },
    Undo {
        #[arg(short, long)]
        yes: bool,
    },
    Delete {
        nos: Vec<usize>,
    },
}

/// 程序入口
fn main() {
    let cli = Cli::parse();
    let store = TaskStore::new(cli.file);
    let result = match cli.command {
        Commands::Add {
            content,
            description,
            deadline,
            priority,
        } => commands::add(content, &store, description, deadline, priority),
        Commands::Change {
            no,
            content,
            description,
            deadline,
            priority,
        } => commands::change(no, &store, content, description, deadline, priority),
        Commands::List { sort } => commands::list(&store, sort),
        Commands::Show { no } => commands::show(no, &store),
        Commands::Status => commands::status(&store),
        Commands::Done { nos } => commands::done(nos, &store),
        Commands::Undone { nos } => commands::undone(nos, &store),
        Commands::Undo { yes } => commands::undo(&store, yes),
        Commands::Delete { nos } => commands::delete(nos, &store),
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
