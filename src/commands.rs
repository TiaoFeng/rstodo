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
use crate::todo::{
    SortBy, TaskStatus, TaskUpdate, add_task, complete_task, delete_task, incomplete_task,
    list_tasks, show_details, undo_task_apply, undo_task_preview,
};

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

pub fn list(store: &TaskStore, sort: Option<SortBy>) -> Result<(), AppError> {
    let tasks_list = list_tasks(store, sort)?;
    match tasks_list {
        None => {
            println!("+_+ No tasks");
            Ok(())
        }
        Some(tasks) => {
            println!("{}", list_table(&tasks, Utc::now()));
            Ok(())
        }
    }
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

pub fn done(no: usize, store: &TaskStore) -> Result<(), AppError> {
    complete_task(no, store)
}

pub fn undone(no: usize, store: &TaskStore) -> Result<(), AppError> {
    incomplete_task(no, store)
}

pub fn delete(no: usize, store: &TaskStore) -> Result<(), AppError> {
    delete_task(no, store)
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
    println!("{}", list_table(&backup_tasks, Utc::now()));

    if !yes && !confirm(&mut io::stdin().lock()) {
        println!(">_< Undo cancelled.");
        return Ok(());
    }
    undo_task_apply(store, &backup_tasks)?;
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

pub fn status(store: &TaskStore) -> Result<(), AppError> {
    let task_status = TaskStatus::collect(store, Utc::now())?;
    println!("{}", status_table(&task_status));
    Ok(())
}

/// 单元测试
#[cfg(test)]
mod commands_test {
    use super::*;
    use crate::test_helpers::TempGuard;
    use crate::time::to_utc;
    use chrono::NaiveDateTime;

    fn set_test_task(store: &TaskStore) {
        add(
            "task1".into(),
            store,
            Some("desc1".into()),
            Some("2000-01-01T12:00:00".into()),
            Some(Priority::High),
        )
        .unwrap();
        add("task2".into(), store, None, None, None).unwrap();
        add(
            "task3".into(),
            store,
            None,
            Some("2000-01-02T08:00:00".into()),
            Some(Priority::Medium),
        )
        .unwrap();
    }

    #[test]
    fn test_add() {
        let guard = TempGuard::new("add");
        let store = TaskStore::new(Some(guard.main_path()));

        // 测试1：全子项写入
        add(
            "test_add1".to_string(),
            &store,
            Some("about_assert_add_test".to_string()),
            Some("2000-01-01T12:00:00".to_string()),
            Some(Priority::High),
        )
        .unwrap();

        let tasks = store.load().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id(), 1);
        assert_eq!(tasks[0].content(), "test_add1");
        assert_eq!(tasks[0].description(), Some("about_assert_add_test"));
        assert_eq!(tasks[0].priority(), Priority::High);
        assert!(!tasks[0].is_complete());

        let expected = to_utc(
            &NaiveDateTime::parse_from_str("2000-01-01T12:00:00", "%Y-%m-%dT%H:%M:%S").unwrap(),
        )
        .unwrap();
        assert_eq!(tasks[0].deadline(), Some(expected));
        // 测试2：默认项测试
        add("test_add2".to_string(), &store, None, None, None).unwrap();
        let tasks = store.load().unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[1].id(), 2);
        assert_eq!(tasks[1].content(), "test_add2".to_string());
        assert_eq!(tasks[1].priority(), Priority::Low);
        assert!(tasks[1].deadline().is_none());
        assert!(tasks[1].description().is_none());
        assert!(!tasks[1].is_complete());
        // 测试3：deadline自动补全测试
        add(
            "test_add3".to_string(),
            &store,
            None,
            Some("2000-01-02".to_string()),
            None,
        )
        .unwrap();
        let tasks = store.load().unwrap();
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[2].id(), 3);
        assert_eq!(tasks[2].content(), "test_add3".to_string());
        assert_eq!(tasks[2].priority(), Priority::Low);
        assert!(tasks[2].description().is_none());
        assert!(!tasks[2].is_complete());

        let expected = to_utc(
            &NaiveDateTime::parse_from_str("2000-01-02T23:59:59", "%Y-%m-%dT%H:%M:%S").unwrap(),
        )
        .unwrap();
        assert_eq!(tasks[2].deadline(), Some(expected));
        // 测试4：错误数据测试
        assert!(
            add(
                "test_add3".to_string(),
                &store,
                None,
                Some("not-a-date".to_string()),
                None
            )
            .is_err()
        );
        assert_eq!(store.load().unwrap().len(), 3);
    }

    #[test]
    fn test_list_show() {
        let guard = TempGuard::new("list_show");
        let store = TaskStore::new(Some(guard.main_path()));

        assert!(list(&store, None).is_ok());
        set_test_task(&store);
        assert!(list(&store, Some(SortBy::Priority)).is_ok());
        println!();
        assert!(list(&store, Some(SortBy::Deadline)).is_ok());
        println!();
        assert!(show(2, &store).is_ok());
        println!();

        assert!(show(0, &store).is_err());
        assert!(show(99, &store).is_err());
    }

    #[test]
    fn test_delete() {
        let guard = TempGuard::new("delete");
        let store = TaskStore::new(Some(guard.main_path()));

        assert!(delete(1, &store).is_err());

        set_test_task(&store);
        delete(2, &store).unwrap();
        let tasks = store.load().unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].content(), "task1".to_string());
        assert_eq!(tasks[1].content(), "task3".to_string());

        assert!(delete(0, &store).is_err());
        assert!(delete(99, &store).is_err());
        assert_eq!(store.load().unwrap().len(), 2);
    }

    #[test]
    fn test_change() {
        let guard = TempGuard::new("change");
        let store = TaskStore::new(Some(guard.main_path()));

        assert!(change(1, &store, None, None, None, None).is_err());
        assert!(
            change(
                1,
                &store,
                Some("change_task1".to_string()),
                None,
                None,
                None
            )
            .is_err()
        );

        set_test_task(&store);
        // 测试1：改content和desc
        change(
            1,
            &store,
            Some("change_task2".to_string()),
            Some(Some("new_desc2".to_string())),
            None,
            None,
        )
        .unwrap();
        let tasks = store.load().unwrap();
        assert_eq!(tasks[0].content(), "change_task2".to_string());
        assert_eq!(tasks[0].description(), Some("new_desc2"));

        // 测试2：清空desc
        change(1, &store, None, Some(None), None, None).unwrap();
        assert!(store.load().unwrap()[0].description().is_none());

        // 测试3：清空deadline
        change(1, &store, None, None, Some(None), None).unwrap();
        assert!(store.load().unwrap()[0].deadline().is_none());

        // 测试4：设置deadline
        change(
            1,
            &store,
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
        assert_eq!(store.load().unwrap()[0].deadline(), Some(expected));

        // 测试5：清空priority
        change(3, &store, None, None, None, Some(None)).unwrap();
        assert_eq!(store.load().unwrap()[2].priority(), Priority::Low);

        // 测试6：添加priority
        change(3, &store, None, None, None, Some(Some(Priority::Medium))).unwrap();
        assert_eq!(store.load().unwrap()[2].priority(), Priority::Medium);

        // 测试7：非法数据
        let tasks = store.load().unwrap();
        assert!(
            change(
                1,
                &store,
                None,
                None,
                Some(Some("not-a-date".to_string())),
                None
            )
            .is_err()
        );
        assert_eq!(store.load().unwrap(), tasks);
    }

    #[test]
    fn test_sort() {
        let guard = TempGuard::new("sort");
        let store = TaskStore::new(Some(guard.main_path()));

        set_test_task(&store);

        list(&store, Some(SortBy::Deadline)).unwrap();
        let tasks = store.load().unwrap();
        let expected = to_utc(
            &NaiveDateTime::parse_from_str("2000-01-01T12:00:00", "%Y-%m-%dT%H:%M:%S").unwrap(),
        )
        .unwrap();
        assert_eq!(tasks[0].deadline(), Some(expected));
        assert!(tasks[2].deadline().is_none());

        list(&store, Some(SortBy::Priority)).unwrap();
        let tasks = store.load().unwrap();
        assert_eq!(tasks[0].priority(), Priority::High);
        assert_eq!(tasks[1].priority(), Priority::Medium);
        assert_eq!(tasks[2].priority(), Priority::Low);
    }

    #[test]
    fn test_done_undone() {
        let guard = TempGuard::new("done_undone");
        let store = TaskStore::new(Some(guard.main_path()));

        set_test_task(&store);

        assert!(done(0, &store).is_err());
        assert!(done(99, &store).is_err());

        done(1, &store).unwrap();
        assert!(store.load().unwrap()[0].is_complete());
        undone(1, &store).unwrap();
        assert!(!store.load().unwrap()[0].is_complete());
    }

    #[cfg(test)]
    mod tests_undo {
        use super::*;

        #[test]
        fn test_undo() {
            let guard = TempGuard::new("test_undo");
            let store = TaskStore::new(Some(guard.main_path()));

            set_test_task(&store);
            add("undo_test_content1".to_string(), &store, None, None, None).unwrap();
            assert_eq!(store.load().unwrap()[3].content(), "undo_test_content1");
            assert_eq!(store.load_backup().unwrap()[1].content(), "task2");
            assert_eq!(store.load_backup().unwrap()[2].content(), "task3");
            assert_eq!(store.load_backup().unwrap().len(), 3);

            undo(&store, true).unwrap();
            assert_eq!(store.load().unwrap()[1].content(), "task2");
            assert_eq!(store.load().unwrap()[2].content(), "task3");
            assert_eq!(store.load().unwrap().len(), 3);
        }

        #[test]
        fn test_undo_after_sort() {
            let guard = TempGuard::new("test_undo_after_sort");
            let store = TaskStore::new(Some(guard.main_path()));

            set_test_task(&store);
            add("undo_test_content1".to_string(), &store, None, None, None).unwrap();
            assert_eq!(store.load().unwrap()[3].content(), "undo_test_content1");
            assert_eq!(store.load_backup().unwrap()[1].content(), "task2");
            assert_eq!(store.load_backup().unwrap()[2].content(), "task3");
            assert_eq!(store.load_backup().unwrap().len(), 3);

            list(&store, Some(SortBy::Deadline)).unwrap();
            list(&store, Some(SortBy::Priority)).unwrap();
            undo(&store, true).unwrap();
            assert_eq!(store.load().unwrap()[1].content(), "task2");
            assert_eq!(store.load().unwrap()[2].content(), "task3");
            assert_eq!(store.load().unwrap().len(), 3);
        }
    }

    #[cfg(test)]
    mod tests_status {
        use super::*;

        #[test]
        fn test_status_count() {
            let guard = TempGuard::new("test_status_count");
            let store = TaskStore::new(Some(guard.main_path()));
            set_test_task(&store);
            assert!(done(1, &store).is_ok());

            let count = TaskStatus::collect(&store, Utc::now()).unwrap();
            assert_eq!(
                count.rows(),
                vec![("Total", 3), ("Done", 1), ("Undone", 2), ("Overdue", 1)]
            );
        }

        #[test]
        fn test_status() {
            let guard = TempGuard::new("test_status");
            let store = TaskStore::new(Some(guard.main_path()));

            assert!(status(&store).is_ok());
            set_test_task(&store);
            assert!(status(&store).is_ok());
            assert!(delete(1, &store).is_ok());
            assert!(status(&store).is_ok());
            assert!(done(1, &store).is_ok());
            assert!(status(&store).is_ok());
        }
    }
}
