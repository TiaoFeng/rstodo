//! commands.rs单元测试
#[cfg(test)]
mod tests {
    use crate::{
        UserInterfaceTypes::Cli,
        commands::*,
        io::storage::TaskStore,
        task::Priority,
        tests::test_helpers::*,
        time::to_utc,
        todo::{SortBy, TaskStatus, list_tasks},
    };
    use chrono::{NaiveDateTime, Utc};

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
            let store = TaskStore::new(Some(guard.main_path()), Cli);

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
            let store = TaskStore::new(Some(guard.main_path()), Cli);

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
            let store = TaskStore::new(Some(guard.main_path()), Cli);

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
            let store = TaskStore::new(Some(guard.main_path()), Cli);

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
            let store = TaskStore::new(Some(guard.main_path()), Cli);

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
            let store = TaskStore::new(Some(guard.main_path()), Cli);

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
            let store = TaskStore::new(Some(guard.main_path()), Cli);

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
            let store = TaskStore::new(Some(guard.main_path()), Cli);
            set_test_task(&store);
            delete(vec![3, 1, 1, 3, 1, 3], &store, false, false).unwrap();
            let tasks = store.load().unwrap();
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].content(), "task2".to_string());
        }

        #[test]
        fn test_delete_alldone() {
            let guard = TempGuard::new("test_delete_alldone");
            let store = TaskStore::new(Some(guard.main_path()), Cli);
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
            let store = TaskStore::new(Some(guard.main_path()), Cli);
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
            let store = TaskStore::new(Some(guard.main_path()), Cli);
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
            let store = TaskStore::new(Some(guard.main_path()), Cli);

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
            let store = TaskStore::new(Some(guard.main_path()), Cli);
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
            let store = TaskStore::new(Some(guard.main_path()), Cli);

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
            let store = TaskStore::new(Some(guard.main_path()), Cli);

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
            let store = TaskStore::new(Some(guard.main_path()), Cli);

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
            let store = TaskStore::new(Some(guard.main_path()), Cli);

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
            let store = TaskStore::new(Some(guard.main_path()), Cli);

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
            let store = TaskStore::new(Some(guard.main_path()), Cli);

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
            let store = TaskStore::new(Some(guard.main_path()), Cli);
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
            let store = TaskStore::new(Some(guard.main_path()), Cli);

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
