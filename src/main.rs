mod commands;
mod error;
mod storage;
mod task;
mod time;

use clap::{Parser, Subcommand};
use storage::FilePath;

use crate::task::Priority;

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
    List,
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
    let path = FilePath::new(None);
    let result = match cli.command {
        Commands::Add {
            content,
            description,
            deadline,
            priority,
        } => commands::add_task(content, &path, description, deadline, priority),
        Commands::Change {
            no,
            content,
            description,
            deadline,
            priority,
        } => commands::change_task(no, &path, content, description, deadline, priority),
        Commands::List => commands::list_task(&path),
        Commands::Show { no } => commands::show_details(no, &path),
        Commands::Done { no } => commands::complete_task(no, &path),
        Commands::Undone { no } => commands::incomplete_task(no, &path),
        Commands::Delete { no } => commands::delete_task(no, &path),
    };
    if let Err(e) = result {
        eprintln!("Error:{}", e);
        std::process::exit(1);
    }
}
