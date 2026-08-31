//! 命令模块
//!
//! 包含用户可执行的添加、删除、修改、输出、展示、恢复等功能
//!
//! 目前所有业务代码已经移动至`src/todo.rs`
//! commands.rs中仅仅保留调用和输出代码
use std::io::{self, BufRead, Write};

use chrono::Utc;

use crate::error::AppError;
use crate::io::{
    cli_print::{
        tasks_status_table::status_table,
        tasks_table::{list_table, show_table},
    },
    storage::TaskStore,
};
use crate::task::Priority;
use crate::time::parse_deadline_input;
use crate::todo::*;

pub fn add(
    content: String,
    store: &TaskStore,
    description: Option<String>,
    deadline: Option<String>,
    priority: Option<Priority>,
) -> Result<(), AppError> {
    let parsed_deadline = match deadline {
        Some(s) => Some(parse_deadline_input(&s)?),
        None => None,
    };
    add_task(store, content, description, parsed_deadline, priority)
}

pub fn list(store: &TaskStore, sort: Option<SortBy>, find: Option<String>) -> Result<(), AppError> {
    let tasks_list = list_tasks(store, sort, find)?;
    match tasks_list {
        None => {
            println!("+_+ No tasks");
        }
        Some(tasks) => {
            println!("{}", list_table(&tasks, Utc::now()));
        }
    }
    Ok(())
}

pub fn show(no: usize, store: &TaskStore) -> Result<(), AppError> {
    let task_show = show_details(no, store)?;
    match task_show {
        None => {
            println!("+_+ No tasks");
            Ok(())
        }
        Some(task) => {
            println!("{}", show_table(&task, no, Utc::now()));
            println!("-Description-");
            match task.description() {
                None => println!("+_+ No description"),
                Some(desc) => println!("{}", desc),
            }
            Ok(())
        }
    }
}

pub fn done(nos: Vec<usize>, store: &TaskStore) -> Result<(), AppError> {
    complete_task(nos, store)
}

pub fn undone(nos: Vec<usize>, store: &TaskStore) -> Result<(), AppError> {
    incomplete_task(nos, store)
}

pub fn delete(
    nos: Vec<usize>,
    store: &TaskStore,
    alldone: bool,
    yes: bool,
) -> Result<(), AppError> {
    // 不能没有参数
    if nos.is_empty() && !alldone {
        return Err(AppError::NothingToDelete);
    }
    // 不能同时选择删除序号和全部
    if !nos.is_empty() && alldone {
        return Err(AppError::DeleteConflictOperations);
    }
    if alldone {
        let delete_tasks = match delete_alldone_preview(store) {
            Ok(tasks) => tasks,
            Err(AppError::NothingToDelete) => {
                println!("+_+ Nothing to delete"); // 将错误降级为普通提醒，与undo一致
                return Ok(());
            }
            Err(err) => return Err(err),
        };
        println!("The following items will be deleted:");
        println!(
            "{}",
            list_table(&with_display_no(&delete_tasks), Utc::now()) // 现在打印需要传入&[(usize, Task)]
        ); // 复用了list_table，实际上由于都是已完成的永远不会着色

        if !yes
            && !confirm(
                &mut io::stdin().lock(),
                "Confirm delete all done tasks? [y/N] ",
            )
        {
            println!(">_< Delete cancelled.");
            return Ok(());
        }
        let count = delete_tasks.len();
        delete_alldone_apply(store, &delete_tasks)?;
        println!("Deleted {} done task(s) >>>", count);
        Ok(())
    } else {
        delete_task(nos, store)
    }
}

pub fn change(
    no: usize,
    store: &TaskStore,
    content: Option<String>,
    description: Option<Option<String>>,
    deadline: Option<Option<String>>,
    priority: Option<Option<Priority>>,
) -> Result<(), AppError> {
    let deadline = match deadline {
        None => None,
        Some(None) => Some(None),
        Some(Some(t)) => Some(Some(parse_deadline_input(&t)?)),
    };
    let taskupdate = TaskUpdate::new(content, description, deadline, priority);
    taskupdate.change_task(no, store)
}

pub fn undo(store: &TaskStore, yes: bool) -> Result<(), AppError> {
    let backup_tasks = match undo_task_preview(store) {
        Ok(tasks) => tasks,
        Err(AppError::NothingToUndo) => {
            println!("+_+ Nothing to undo");
            return Ok(());
        }
        Err(e) => return Err(e),
    };
    println!("The list will be restored to:");
    if backup_tasks.is_empty() {
        println!("+_+ No tasks")
    } else {
        println!(
            "{}",
            list_table(&with_display_no(&backup_tasks), Utc::now()) // 现在打印需要传入&[(usize, Task)]
        );
    }

    if !yes && !confirm(&mut io::stdin().lock(), "Confirm undo? [y/N] ") {
        println!(">_< Undo cancelled.");
        return Ok(());
    }
    undo_task_apply(store, &backup_tasks)?;
    println!("Undo >>>");
    Ok(())
}

/// 二次确认函数
fn confirm(read: &mut dyn BufRead, prompt: &str) -> bool {
    print!("{}", prompt);
    let _ = io::stdout().flush();
    let mut input = String::new();
    match read.read_line(&mut input) {
        Ok(0) => false,
        Ok(_) => matches!(input.trim().to_lowercase().as_str(), "y" | "yes"),
        Err(_) => false,
    }
}

pub fn status(store: &TaskStore) -> Result<(), AppError> {
    let task_status = TaskStatus::collect(store, Utc::now())?;
    println!("{}", status_table(&task_status));
    Ok(())
}
