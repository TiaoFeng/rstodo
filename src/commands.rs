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
        tasks_table::{list_table, show_table, with_display_no},
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

    #[cfg(test)]
    mod tests_add {
        use super::*;

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
        }

        #[test]
        fn test_add_err() {
            let guard = TempGuard::new("test_add_err");
            let store = TaskStore::new(Some(guard.main_path()));

            set_test_task(&store);
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
            // 测试5：空content测试
            assert!(add(" ".to_string(), &store, None, None, None).is_err());
            assert_eq!(store.load().unwrap().len(), 3);
            // 测试6： 空description测试
            assert!(
                add(
                    "test_add4".to_string(),
                    &store,
                    Some("     ".to_string()),
                    None,
                    None
                )
                .is_err()
            );
            assert_eq!(store.load().unwrap().len(), 3);
        }

        /// 备份文件不可写，直接抛出错误
        ///
        /// 利用权限系统，无法创建可以读写的句柄
        #[cfg(unix)]
        #[test]
        fn test_backup_not_writable() {
            use std::{fs, os::unix::fs::PermissionsExt};
            let guard = TempGuard::new("test_backup_not_writable");
            let store = TaskStore::new(Some(guard.main_path()));

            assert!(add("test1".to_string(), &store, None, None, None).is_ok());
            assert!(add("test2".to_string(), &store, None, None, None).is_ok());

            let no_permission = 0o200;
            let std_permission = 0o644;
            fs::set_permissions(
                store.backup_path(),
                fs::Permissions::from_mode(no_permission),
            )
            .unwrap();
            assert!(add("test3".to_string(), &store, None, None, None).is_err());
            assert_eq!(store.load().unwrap().len(), 2);

            fs::set_permissions(
                store.backup_path(),
                fs::Permissions::from_mode(std_permission),
            )
            .unwrap();
            assert!(add("test3".to_string(), &store, None, None, None).is_ok());
            assert_eq!(store.load().unwrap().len(), 3);
        }
    }

    #[cfg(test)]
    mod tests_list_show {
        use super::*;

        #[test]
        fn test_list_show() {
            let guard = TempGuard::new("test_list_show");
            let store = TaskStore::new(Some(guard.main_path()));

            assert!(list(&store, None, None).is_ok());
            set_test_task(&store);
            assert!(list(&store, Some(SortBy::Priority), None).is_ok());
            println!();
            assert!(list(&store, Some(SortBy::Deadline), None).is_ok());
            println!();
            assert!(show(2, &store).is_ok());
            println!();

            assert!(show(0, &store).is_err());
            assert!(show(99, &store).is_err());
        }

        #[test]
        fn test_list_find() {
            let guard = TempGuard::new("test_list_find");
            let store = TaskStore::new(Some(guard.main_path()));

            // 空列表搜索返回None，不会报错
            assert!(list(&store, None, Some("empty".to_string())).is_ok());

            set_test_task(&store);

            // 搜索content
            let out = list_tasks(&store, None, Some("task1".to_string())).unwrap();
            assert!(out.is_some());
            let tasks = out.unwrap();
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].1.content(), "task1");
            assert_eq!(tasks[0].0, 1); // 序号为1

            // 搜索description
            let out = list_tasks(&store, None, Some("desc1".to_string())).unwrap();
            assert!(out.is_some());
            let tasks = out.unwrap();
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].1.content(), "task1");

            // 搜索priority
            let out = list_tasks(&store, None, Some("medium".to_string())).unwrap();
            assert!(out.is_some());
            let tasks = out.unwrap();
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].0, 3); // 保留在原列表中的序号，应该是3
            assert_eq!(tasks[0].1.priority(), Priority::Medium);

            // 搜索不存在的关键词
            let out = list_tasks(&store, None, Some("nonexistent".to_string())).unwrap();
            assert!(out.is_none());

            // done
            done(vec![1, 3], &store).unwrap();
            let out = list_tasks(&store, None, Some("done".to_string())).unwrap();
            assert!(out.is_some());
            let tasks = out.unwrap();
            assert_eq!(tasks.len(), 2); // task1, task3
            assert_eq!(tasks[0].0, 1);
            assert_eq!(tasks[1].0, 3);

            // todo
            let result = list_tasks(&store, None, Some("todo".to_string())).unwrap();
            assert!(result.is_some());
            let tasks = result.unwrap();
            assert_eq!(tasks.len(), 1); // task2
            assert_eq!(tasks[0].0, 2); // task2

            // 测试搜索+排序组合
            let result =
                list_tasks(&store, Some(SortBy::Priority), Some("done".to_string())).unwrap();
            assert!(result.is_some());
            let tasks = result.unwrap();
            assert_eq!(tasks.len(), 2); // task1(high), task3(medium)
            // 验证排序正确性
            assert!(tasks[0].1.priority() <= tasks[1].1.priority());
        }
    }

    #[cfg(test)]
    mod tests_delete {
        use super::*;

        #[test]
        fn test_delete_sigle() {
            let guard = TempGuard::new("test_delete_sigle");
            let store = TaskStore::new(Some(guard.main_path()));

            assert!(delete(vec![1], &store, false, false).is_err());

            set_test_task(&store);
            delete(vec![2], &store, false, false).unwrap();
            let tasks = store.load().unwrap();
            assert_eq!(tasks.len(), 2);
            assert_eq!(tasks[0].content(), "task1".to_string());
            assert_eq!(tasks[1].content(), "task3".to_string());

            assert!(delete(vec![0], &store, false, false).is_err());
            assert!(delete(vec![99], &store, false, false).is_err());
            assert_eq!(store.load().unwrap().len(), 2);
        }

        #[test]
        fn test_delete_multiple() {
            let guard = TempGuard::new("test_delete_multiple");
            let store = TaskStore::new(Some(guard.main_path()));

            set_test_task(&store);
            delete(vec![1, 2], &store, false, false).unwrap();
            let tasks = store.load().unwrap();
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].content(), "task3".to_string());

            assert!(delete(vec![1, 99, 999], &store, false, false).is_err());
        }

        #[test]
        fn test_delete_multiple_duplicate() {
            let guard = TempGuard::new("test_delete_multiple_duplicate");
            let store = TaskStore::new(Some(guard.main_path()));
            set_test_task(&store);
            delete(vec![3, 1, 1, 3, 1, 3], &store, false, false).unwrap();
            let tasks = store.load().unwrap();
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].content(), "task2".to_string());
        }

        #[test]
        fn test_delete_alldone() {
            let guard = TempGuard::new("test_delete_alldone");
            let store = TaskStore::new(Some(guard.main_path()));
            set_test_task(&store);
            assert!(done(vec![1, 2], &store).is_ok());
            assert!(delete(vec![], &store, true, true).is_ok());
            let tasks = store.load().unwrap();
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].content(), "task3".to_string());
        }

        #[test]
        fn test_delete_alldone_no_tasks_done() {
            let guard = TempGuard::new("test_delete_alldone_no_tasks_done");
            let store = TaskStore::new(Some(guard.main_path()));
            set_test_task(&store);
            assert!(delete(vec![], &store, true, true).is_ok());
            let tasks = store.load().unwrap();
            assert_eq!(tasks.len(), 3);
            assert_eq!(tasks[0].content(), "task1".to_string());
            assert_eq!(tasks[1].content(), "task2".to_string());
            assert_eq!(tasks[2].content(), "task3".to_string());
        }

        #[test]
        fn test_delete_alldone_err() {
            let guard = TempGuard::new("test_delete_alldone_err");
            let store = TaskStore::new(Some(guard.main_path()));
            set_test_task(&store);
            assert!(delete(vec![1, 2], &store, true, true).is_err());
            let tasks = store.load().unwrap();
            assert_eq!(tasks.len(), 3);
            assert_eq!(tasks[0].content(), "task1".to_string());
            assert_eq!(tasks[1].content(), "task2".to_string());
            assert_eq!(tasks[2].content(), "task3".to_string());

            assert!(delete(vec![], &store, false, true).is_err());
            let tasks = store.load().unwrap();
            assert_eq!(tasks.len(), 3);
            assert_eq!(tasks[0].content(), "task1".to_string());
            assert_eq!(tasks[1].content(), "task2".to_string());
            assert_eq!(tasks[2].content(), "task3".to_string());
        }
    }

    #[cfg(test)]
    mod tests_change {
        use super::*;

        #[test]
        fn test_change() {
            let guard = TempGuard::new("test_change");
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
        }

        #[test]
        fn test_change_err() {
            let guard = TempGuard::new("test_change_err");
            let store = TaskStore::new(Some(guard.main_path()));
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
            // 测试8： content为空
            assert!(change(1, &store, Some(" ".to_string()), None, None, None).is_err());
            assert_eq!(store.load().unwrap(), tasks);
            // 测试9： description为空
            assert!(change(1, &store, None, Some(Some("   ".to_string())), None, None).is_err());
            assert_eq!(store.load().unwrap(), tasks);
        }
    }

    #[cfg(test)]
    mod tests_sort {
        use super::*;

        #[test]
        fn test_sort() {
            let guard = TempGuard::new("test_sort");
            let store = TaskStore::new(Some(guard.main_path()));

            set_test_task(&store);

            list(&store, Some(SortBy::Deadline), None).unwrap();
            let tasks = store.load().unwrap();
            let expected = to_utc(
                &NaiveDateTime::parse_from_str("2000-01-01T12:00:00", "%Y-%m-%dT%H:%M:%S").unwrap(),
            )
            .unwrap();
            assert_eq!(tasks[0].deadline(), Some(expected));
            assert!(tasks[2].deadline().is_none());

            list(&store, Some(SortBy::Priority), None).unwrap();
            let tasks = store.load().unwrap();
            assert_eq!(tasks[0].priority(), Priority::High);
            assert_eq!(tasks[1].priority(), Priority::Medium);
            assert_eq!(tasks[2].priority(), Priority::Low);
        }
    }

    #[cfg(test)]
    mod tests_done_undone {
        use super::*;

        #[test]
        fn test_done_undone_sigle() {
            let guard = TempGuard::new("test_done_undone_sigle");
            let store = TaskStore::new(Some(guard.main_path()));

            set_test_task(&store);

            assert!(done(vec![0], &store).is_err());
            assert!(undone(vec![99], &store).is_err());

            done(vec![1], &store).unwrap();
            assert!(store.load().unwrap()[0].is_complete());
            undone(vec![1], &store).unwrap();
            assert!(!store.load().unwrap()[0].is_complete());
        }

        #[test]
        fn test_done_undone_multiple() {
            let guard = TempGuard::new("test_done_undone_multiple");
            let store = TaskStore::new(Some(guard.main_path()));

            set_test_task(&store);

            assert!(done(vec![1, 3, 5, 7, 9], &store).is_err());
            assert!(undone(vec![1, 99], &store).is_err());

            done(vec![1, 3], &store).unwrap();
            assert!(store.load().unwrap()[0].is_complete());
            assert!(store.load().unwrap()[2].is_complete());
            undone(vec![1, 3], &store).unwrap();
            assert!(!store.load().unwrap()[0].is_complete());
            assert!(!store.load().unwrap()[2].is_complete());
        }
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

            list(&store, Some(SortBy::Deadline), None).unwrap();
            list(&store, Some(SortBy::Priority), None).unwrap();
            undo(&store, true).unwrap();
            assert_eq!(store.load().unwrap()[1].content(), "task2");
            assert_eq!(store.load().unwrap()[2].content(), "task3");
            assert_eq!(store.load().unwrap().len(), 3);
        }

        #[test]
        fn test_undo_to_empty() {
            let guard = TempGuard::new("test_undo_to_empty");
            let store = TaskStore::new(Some(guard.main_path()));

            add("undo_test_content1".to_string(), &store, None, None, None).unwrap();
            undo(&store, true).unwrap();
            assert_eq!(store.load().unwrap().len(), 0);
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
            assert!(done(vec![1], &store).is_ok());

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
            assert!(delete(vec![1], &store, false, false).is_ok());
            assert!(status(&store).is_ok());
            assert!(done(vec![1], &store).is_ok());
            assert!(status(&store).is_ok());
        }
    }
}
