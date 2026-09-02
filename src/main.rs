//! 程序入口
//!
//! 使用clap实现终端命令输入

mod commands;
mod error;
mod io;
mod task;
mod time;
mod todo;
mod tui;

#[cfg(test)]
mod tests;

use clap::{Parser, Subcommand};
use std::error::Error;

use crate::{
    error::AppError,
    io::storage::{TaskStore, recovered_from_backup_msg},
    task::Priority,
    todo::SortBy,
};

/// Welcome to rstodo.
/// Please enter a command to create or manage your tasks.
#[derive(Parser)]
struct Cli {
    #[arg(long, global = true)]
    file: Option<String>,
    #[command(subcommand)]
    command: Option<Commands>, // 子命令是可选的，可以不输入进入tui
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
        #[arg(short, long)]
        find: Option<String>,
    },
    Show {
        no: usize,
    },
    Status,
    Done {
        #[arg(required = true)]
        nos: Vec<usize>,
    },
    Undone {
        #[arg(required = true)]
        nos: Vec<usize>,
    },
    Undo {
        #[arg(short, long)]
        yes: bool,
    },
    Delete {
        nos: Vec<usize>,
        #[arg(long)]
        alldone: bool,
        #[arg(short, long)]
        yes: bool,
    },
}

/// 用户界面枚举
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum UserInterfaceTypes {
    Cli,
    Tui,
}

/// 程序入口
fn main() {
    let cli = Cli::parse();
    let ui_type = if cli.command.is_some() {
        UserInterfaceTypes::Cli
    } else {
        UserInterfaceTypes::Tui
    };
    let store = TaskStore::new(cli.file, ui_type);
    let result: Result<(), AppError> = if let Some(cmd) = cli.command {
        match cmd {
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
            Commands::List { sort, find } => commands::list(&store, sort, find),
            Commands::Show { no } => commands::show(no, &store),
            Commands::Status => commands::status(&store),
            Commands::Done { nos } => commands::done(nos, &store),
            Commands::Undone { nos } => commands::undone(nos, &store),
            Commands::Undo { yes } => commands::undo(&store, yes),
            Commands::Delete { nos, alldone, yes } => commands::delete(nos, &store, alldone, yes),
        }
    } else {
        tui::run(&store)
    };
    // 如果UI为CLI在指令执行完成后输出储存的提示
    if ui_type == UserInterfaceTypes::Cli && store.take_notice().is_some() {
        eprintln!(
            "{}",
            recovered_from_backup_msg(store.backup_path(), ui_type)
        );
    }
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
