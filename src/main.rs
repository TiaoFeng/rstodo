mod commands;
mod storage;
mod task;
mod time;

use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use storage::FilePath;
use time::parse_deadline_input;

#[derive(Parser)]
#[command(name = "todo")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Add {
        content: String,
        #[arg(short, long)]
        deadline: Option<String>,
    },
    Change {
        id: usize,
        #[arg(short, long)]
        content: Option<String>,
        #[arg(short, long)]
        deadline: Option<Option<String>>,
    },
    List,
    Done {
        id: usize,
    },
    Undone {
        id: usize,
    },
    Delete {
        id: usize,
    },
}

fn main() {
    let cli = Cli::parse();
    let path = FilePath::new(None);
    let result = match cli.command {
        Commands::Add { content, deadline } => {
            let parsed_deadline: Option<DateTime<Utc>> = match deadline {
                Some(s) => Some(parse_deadline_input(&s).unwrap()),
                None => None,
            };
            commands::add_task(content, &path, parsed_deadline)
        }
        Commands::Change {
            id,
            content,
            deadline,
        } => commands::change_task(id, &path, content, deadline),
        Commands::List => {
            commands::list_task(&path);
            Ok(())
        }
        Commands::Done { id } => commands::complete_task(id, &path),
        Commands::Undone { id } => commands::incomplete_task(id, &path),
        Commands::Delete { id } => commands::delete_task(id, &path),
    };
    if let Err(e) = result {
        eprintln!("Error:{}", e);
        std::process::exit(1);
    }
}
