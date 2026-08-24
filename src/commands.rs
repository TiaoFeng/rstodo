//! 命令模块
//!
//! 包含用户可执行的添加、删除、修改、输出、展示、恢复等功能
use std::cmp::Ordering;
use std::io::{self, BufRead, Write};

use clap::ValueEnum;

use crate::error::AppError;
use crate::io::cli_print::show_table;
use crate::io::{cli_print::list_table, storage::TaskStore};
use crate::task::{Priority, Task};
use crate::time::parse_deadline_input;

/// 定义了用户可选择的两种排序方式，按照时间或优先级
#[derive(ValueEnum, Clone)]
pub enum SortBy {
    #[value(alias = "d")]
    Deadline,
    #[value(alias = "p")]
    Priority,
}

pub fn add_task(
    content: String,
    path: &TaskStore,
    description: Option<String>,
    deadline: Option<String>,
    priority: Option<Priority>,
) -> Result<(), AppError> {
    let parsed_deadline = match deadline {
        Some(s) => Some(parse_deadline_input(&s)?),
        None => None,
    };
    path.update(|tasks| {
        let new_id: usize = tasks.iter().map(|t| t.id()).max().unwrap_or(0) + 1;
        let new_task: Task = Task::new(
            new_id,
            content,
            description,
            parsed_deadline,
            priority.unwrap_or_default(),
        );
        tasks.push(new_task);
        Ok(())
    })
}

pub fn list_task(path: &TaskStore, sort: Option<SortBy>) -> Result<(), AppError> {
    match sort {
        None => {
            let tasks = path.load()?;
            print_task_list(&tasks);
            Ok(())
        }
        Some(order) => path.update(|tasks| {
            sort_tasks(tasks, order);
            print_task_list(tasks);
            Ok(())
        }),
    }
}

fn print_task_list(tasks: &[Task]) {
    if tasks.is_empty() {
        println!("+_+ No tasks");
    } else {
        println!("{}", list_table(tasks));
    }
}

fn sort_tasks(tasks: &mut [Task], order: SortBy) {
    match order {
        SortBy::Deadline => tasks.sort_by(|a, b| match (a.deadline(), b.deadline()) {
            (Some(d1), Some(d2)) => d1.cmp(&d2),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }),
        SortBy::Priority => tasks.sort_by_key(|t| t.priority()),
    }
}

pub fn show_details(no: usize, path: &TaskStore) -> Result<(), AppError> {
    let tasks = path.load()?;
    if tasks.is_empty() {
        println!("+_+ No tasks");
        return Ok(());
    }

    let task = tasks
        .get(no.checked_sub(1).ok_or(AppError::TaskNotFound { no })?)
        .ok_or(AppError::TaskNotFound { no })?;

    let table = show_table(task, no);
    println!("{}", table);

    println!("-Description-");
    if task.description().is_none() {
        println!("+_+ No description");
    } else {
        println!("{}", task.description().unwrap());
    }
    Ok(())
}

pub fn complete_task(no: usize, path: &TaskStore) -> Result<(), AppError> {
    path.update(|tasks| {
        let idx = no.checked_sub(1).ok_or(AppError::TaskNotFound { no })?;
        let task = tasks.get_mut(idx).ok_or(AppError::TaskNotFound { no })?;
        task.complete();
        Ok(())
    })
}

pub fn incomplete_task(no: usize, path: &TaskStore) -> Result<(), AppError> {
    path.update(|tasks| {
        let idx = no.checked_sub(1).ok_or(AppError::TaskNotFound { no })?;
        let task = tasks.get_mut(idx).ok_or(AppError::TaskNotFound { no })?;
        task.incomplete();
        Ok(())
    })
}

pub fn delete_task(no: usize, path: &TaskStore) -> Result<(), AppError> {
    path.update(|tasks| {
        let idx = no.checked_sub(1).ok_or(AppError::TaskNotFound { no })?;
        if idx >= tasks.len() {
            return Err(AppError::TaskNotFound { no });
        }
        tasks.remove(idx);
        Ok(())
    })
}

pub fn change_task(
    no: usize,
    path: &TaskStore,
    content: Option<String>,
    description: Option<Option<String>>,
    deadline: Option<Option<String>>,
    priority: Option<Option<Priority>>,
) -> Result<(), AppError> {
    if content.is_none() && deadline.is_none() && description.is_none() && priority.is_none() {
        return Err(AppError::NothingToChange);
    }
    path.update(|tasks| {
        let idx = no.checked_sub(1).ok_or(AppError::TaskNotFound { no })?;
        let task = tasks.get_mut(idx).ok_or(AppError::TaskNotFound { no })?;
        if let Some(c) = content {
            task.set_content(c);
        }
        match deadline {
            Some(Some(s)) => {
                task.set_deadline(Some(parse_deadline_input(&s)?));
            }
            Some(None) => task.set_deadline(None),
            None => {}
        }
        match description {
            Some(Some(s)) => {
                task.set_description(Some(s));
            }
            Some(None) => task.set_description(None),
            None => {}
        }
        match priority {
            Some(Some(p)) => task.set_priority(p),
            Some(None) => task.set_priority(Priority::default()),
            None => {}
        }
        Ok(())
    })
}

pub fn undo_task(path: &TaskStore, yes: bool) -> Result<(), AppError> {
    let backup_tasks = match path.load_backup() {
        Ok(tasks) => tasks,
        Err(AppError::NothingToUndo) => {
            println!("+_+ Nothing to undo");
            return Ok(());
        }
        Err(e) => return Err(e),
    };
    let current = path.load()?;
    if backup_tasks == current {
        println!("+_+ Nothing to undo");
        return Ok(());
    }
    println!("The list will be restored to:");
    println!("{}", list_table(&backup_tasks));
    if !yes && !confirm(&mut io::stdin().lock()) {
        println!(">_< Undo cancelled.");
        return Ok(());
    }
    path.restore_backup()?;
    println!("Undo >>>");
    Ok(())
}

fn confirm(read: &mut dyn BufRead) -> bool {
    print!("Confirm undo? [y/N] ");
    let _ = io::stdout().flush();
    let mut input = String::new();
    match read.read_line(&mut input) {
        Ok(0) => false,
        Ok(_) => matches!(input.trim().to_lowercase().as_str(), "y" | "yes"),
        Err(_) => false,
    }
}

/// 单元测试
#[cfg(test)]
mod commands_test {
    use super::*;
    use crate::time::to_utc;
    use chrono::NaiveDateTime;
    use std::{fs, process};

    fn temp_path(name: &str) -> String {
        std::env::temp_dir()
            .join(format!("{}_rstodo_test_{}.json", process::id(), name))
            .to_string_lossy()
            .to_string()
    }

    fn set_test_task(path: &TaskStore) {
        add_task(
            "task1".into(),
            path,
            Some("desc1".into()),
            Some("2000-01-01T12:00:00".into()),
            Some(Priority::High),
        )
        .unwrap();
        add_task("task2".into(), path, None, None, None).unwrap();
        add_task(
            "task3".into(),
            path,
            None,
            Some("2000-01-02T08:00:00".into()),
            Some(Priority::Medium),
        )
        .unwrap();
    }

    #[test]
    fn test_add() {
        let file = temp_path("add");
        let path = TaskStore::new(Some(file.clone()));
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());

        // 测试1：全子项写入
        add_task(
            "test_add1".to_string(),
            &path,
            Some("about_assert_add_test".to_string()),
            Some("2000-01-01T12:00:00".to_string()),
            Some(Priority::High),
        )
        .unwrap();

        let tasks = path.load().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id(), 1);
        assert_eq!(tasks[0]._content(), "test_add1");
        assert_eq!(
            tasks[0].description(),
            Some("about_assert_add_test".to_string())
        );
        assert_eq!(tasks[0].priority(), Priority::High);
        assert!(!tasks[0]._completed());

        let expected = to_utc(
            &NaiveDateTime::parse_from_str("2000-01-01T12:00:00", "%Y-%m-%dT%H:%M:%S").unwrap(),
        )
        .unwrap();
        assert_eq!(tasks[0].deadline(), Some(expected));
        // 测试2：默认项测试
        add_task("test_add2".to_string(), &path, None, None, None).unwrap();
        let tasks = path.load().unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[1].id(), 2);
        assert_eq!(tasks[1]._content(), "test_add2".to_string());
        assert_eq!(tasks[1].priority(), Priority::Low);
        assert!(tasks[1].deadline().is_none());
        assert!(tasks[1].description().is_none());
        assert!(!tasks[1]._completed());
        // 测试3：deadline自动补全测试
        add_task(
            "test_add3".to_string(),
            &path,
            None,
            Some("2000-01-02".to_string()),
            None,
        )
        .unwrap();
        let tasks = path.load().unwrap();
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[2].id(), 3);
        assert_eq!(tasks[2]._content(), "test_add3".to_string());
        assert_eq!(tasks[2].priority(), Priority::Low);
        assert!(tasks[2].description().is_none());
        assert!(!tasks[2]._completed());

        let expected = to_utc(
            &NaiveDateTime::parse_from_str("2000-01-02T23:59:59", "%Y-%m-%dT%H:%M:%S").unwrap(),
        )
        .unwrap();
        assert_eq!(tasks[2].deadline(), Some(expected));
        // 测试4：错误数据测试
        assert!(
            add_task(
                "test_add3".to_string(),
                &path,
                None,
                Some("not-a-date".to_string()),
                None
            )
            .is_err()
        );
        assert_eq!(path.load().unwrap().len(), 3);
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());
    }

    #[test]
    fn test_list_show() {
        let file = temp_path("list_show");
        let path = TaskStore::new(Some(file.clone()));
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());

        assert!(list_task(&path, None).is_ok());
        set_test_task(&path);
        assert!(list_task(&path, Some(SortBy::Priority)).is_ok());
        println!();
        assert!(list_task(&path, Some(SortBy::Deadline)).is_ok());
        println!();
        assert!(show_details(2, &path).is_ok());
        println!();

        assert!(show_details(0, &path).is_err());
        assert!(show_details(99, &path).is_err());
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());
    }

    #[test]
    fn test_delete() {
        let file = temp_path("delete");
        let path = TaskStore::new(Some(file.clone()));
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());

        assert!(delete_task(1, &path).is_err());

        set_test_task(&path);
        delete_task(2, &path).unwrap();
        let tasks = path.load().unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0]._content(), "task1".to_string());
        assert_eq!(tasks[1]._content(), "task3".to_string());

        assert!(delete_task(0, &path).is_err());
        assert!(delete_task(99, &path).is_err());
        assert_eq!(path.load().unwrap().len(), 2);
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());
    }

    #[test]
    fn test_change() {
        let file = temp_path("change");
        let path = TaskStore::new(Some(file.clone()));
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());

        assert!(change_task(1, &path, None, None, None, None).is_err());
        assert!(change_task(1, &path, Some("change_task1".to_string()), None, None, None).is_err());

        set_test_task(&path);
        // 测试1：改content和desc
        change_task(
            1,
            &path,
            Some("change_task2".to_string()),
            Some(Some("new_desc2".to_string())),
            None,
            None,
        )
        .unwrap();
        let tasks = path.load().unwrap();
        assert_eq!(tasks[0]._content(), "change_task2".to_string());
        assert_eq!(tasks[0].description(), Some("new_desc2".to_string()));

        // 测试2：清空desc
        change_task(1, &path, None, Some(None), None, None).unwrap();
        assert!(path.load().unwrap()[0].description().is_none());

        // 测试3：清空deadline
        change_task(1, &path, None, None, Some(None), None).unwrap();
        assert!(path.load().unwrap()[0].deadline().is_none());

        // 测试4：设置deadline
        change_task(
            1,
            &path,
            None,
            None,
            Some(Some("2000-2-1".to_string())),
            None,
        )
        .unwrap();
        let expected = to_utc(
            &NaiveDateTime::parse_from_str("2000-02-01T23:59:59", "%Y-%m-%dT%H:%M:%S").unwrap(),
        )
        .unwrap();
        assert_eq!(path.load().unwrap()[0].deadline(), Some(expected));

        // 测试5：清空priority
        change_task(3, &path, None, None, None, Some(None)).unwrap();
        assert_eq!(path.load().unwrap()[2].priority(), Priority::Low);

        // 测试6：添加priority
        change_task(3, &path, None, None, None, Some(Some(Priority::Medium))).unwrap();
        assert_eq!(path.load().unwrap()[2].priority(), Priority::Medium);

        // 测试7：非法数据
        let tasks = path.load().unwrap();
        assert!(
            change_task(
                1,
                &path,
                None,
                None,
                Some(Some("not-a-date".to_string())),
                None
            )
            .is_err()
        );
        assert_eq!(path.load().unwrap(), tasks);
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());
    }

    #[test]
    fn test_sort() {
        let file = temp_path("sort");
        let path = TaskStore::new(Some(file.clone()));
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());

        set_test_task(&path);

        list_task(&path, Some(SortBy::Deadline)).unwrap();
        let tasks = path.load().unwrap();
        let expected = to_utc(
            &NaiveDateTime::parse_from_str("2000-01-01T12:00:00", "%Y-%m-%dT%H:%M:%S").unwrap(),
        )
        .unwrap();
        assert_eq!(tasks[0].deadline(), Some(expected));
        assert!(tasks[2].deadline().is_none());

        list_task(&path, Some(SortBy::Priority)).unwrap();
        let tasks = path.load().unwrap();
        assert_eq!(tasks[0].priority(), Priority::High);
        assert_eq!(tasks[1].priority(), Priority::Medium);
        assert_eq!(tasks[2].priority(), Priority::Low);
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());
    }

    #[test]
    fn test_done_undone() {
        let file = temp_path("done_undone");
        let path = TaskStore::new(Some(file.clone()));
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());

        set_test_task(&path);

        assert!(complete_task(0, &path).is_err());
        assert!(complete_task(99, &path).is_err());

        complete_task(1, &path).unwrap();
        assert!(path.load().unwrap()[0]._completed());
        incomplete_task(1, &path).unwrap();
        assert!(!path.load().unwrap()[0]._completed());
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());
    }

    #[test]
    fn test_undo() {
        let file = temp_path("test_undo");
        let path = TaskStore::new(Some(file.clone()));
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());

        set_test_task(&path);
        add_task("undo_test_content1".to_string(), &path, None, None, None).unwrap();
        assert_eq!(path.load().unwrap()[3]._content(), "undo_test_content1");
        assert_eq!(path.load_backup().unwrap()[1]._content(), "task2");
        assert_eq!(path.load_backup().unwrap()[2]._content(), "task3");
        assert_eq!(path.load_backup().unwrap().len(), 3);

        undo_task(&path, true).unwrap();
        assert_eq!(path.load().unwrap()[1]._content(), "task2");
        assert_eq!(path.load().unwrap()[2]._content(), "task3");
        assert_eq!(path.load().unwrap().len(), 3);

        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());
    }
}
