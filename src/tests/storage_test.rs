//! storage.rs单元测试
#[cfg(test)]
mod tests {
    use crate::{
        UserInterfaceTypes::Cli,
        error::AppError,
        io::storage::TaskStore,
        task::{Priority, Task},
        tests::test_helpers::*,
    };
    use chrono::{DateTime, Utc};
    use std::{fs, os::unix::fs::PermissionsExt, path::Path};

    fn write_file(path: &Path, contents: &str) {
        fs::write(path, contents).unwrap();
    }

    fn set_test_task() -> Task {
        let deadline: DateTime<Utc> = "2000-1-1T12:00:00+00:00".parse().unwrap();
        Task::new(
            1,
            "test_task1".to_string(),
            Some("desc".to_string()),
            Some(deadline),
            Priority::High,
        )
    }

    #[test]
    fn file_path() {
        let store = TaskStore::new(Some("test1.json".to_string()), Cli);
        assert_eq!(store.main_path(), Path::new("test1.json"));
        assert_eq!(store.backup_path(), Path::new("test1.json.bak"));

        let default_store = TaskStore::new(None, Cli);
        let main = default_store.main_path();
        assert!(main.ends_with(Path::new("rstodo").join("task.json")));
        assert_eq!(
            default_store.backup_path(),
            main.with_file_name("task.json.bak")
        );
    }

    #[cfg(test)]
    mod tests_load {

        use super::*;

        #[test]
        fn test_load_tasks() {
            let guard = TempGuard::new("load");
            let path = TaskStore::new(Some(guard.main_path()), Cli);

            assert_eq!(path.load().unwrap(), Vec::new());

            write_file(path.main_path(), "");
            assert_eq!(path.load().unwrap(), Vec::new());

            write_file(
                path.main_path(),
                &serde_json::to_string_pretty(&vec![set_test_task()]).unwrap(),
            );
            let load = path.load().unwrap();
            assert_eq!(load, vec![set_test_task()]);

            write_file(path.main_path(), "{Illegal data");
            match path.load() {
                Err(AppError::Corrupted { path, source: _ }) => {
                    assert_eq!(path, guard.main_path())
                }
                _ => unreachable!(),
            }
        }

        #[test]
        fn test_load_tasks_err() {
            let guard = TempGuard::new("load_err_not_utf8");
            let path = TaskStore::new(Some(guard.main_path()), Cli);

            fs::write(path.main_path(), [0xFF, 0xFF]).unwrap();
            let err = path.load().unwrap_err();

            match err {
                AppError::Io {
                    operation,
                    path: _,
                    source,
                } => {
                    assert_eq!(operation, "read to string");
                    assert!(!source.to_string().is_empty());
                }
                _ => unreachable!(),
            }
        }

        #[cfg(unix)]
        #[test]
        fn test_permission_denied() {
            let guard = TempGuard::new("permission_denied");
            let path = TaskStore::new(Some(guard.main_path()), Cli);

            fs::write(path.main_path(), "test_content").unwrap();
            assert_eq!(
                fs::read_to_string(path.main_path()).unwrap(),
                "test_content".to_string()
            );

            let no_permission = 0o000;
            let std_permission = 0o644;
            fs::set_permissions(path.main_path(), fs::Permissions::from_mode(no_permission))
                .unwrap();
            let err = path.load().unwrap_err();
            match err {
                AppError::Io {
                    operation,
                    path: _,
                    source,
                } => {
                    assert_eq!(operation, "open read");
                    assert!(!source.to_string().is_empty());
                }
                _ => unreachable!(),
            }
            fs::set_permissions(path.main_path(), fs::Permissions::from_mode(std_permission))
                .unwrap();
        }
    }

    #[cfg(test)]
    mod tests_update {
        use super::*;

        #[test]
        fn test_update() {
            let guard = TempGuard::new("update");
            let path = TaskStore::new(Some(guard.main_path()), Cli);
            for _ in 0..5 {
                path.update_with_backup(|t| {
                    t.push(set_test_task());
                    Ok(())
                })
                .unwrap();
            }
            let load = path.load().unwrap();
            assert_eq!(load.len(), 5);

            for task in load.iter().take(5) {
                assert_eq!(task.id(), 1);
                assert_eq!(task.content(), "test_task1".to_string());
                assert_eq!(task.description(), Some("desc"));
                assert_eq!(
                    task.deadline(),
                    Some("2000-1-1T12:00:00+00:00".parse::<DateTime<Utc>>().unwrap())
                );
                assert_eq!(task.priority(), Priority::High);
            }

            path.update_with_backup(|t| {
                t.clear();
                Ok(())
            })
            .unwrap();
            let load = path.load().unwrap();
            assert_eq!(load, Vec::new());
        }

        #[test]
        fn test_update_err() {
            let guard = TempGuard::new("update_err");
            let path = TaskStore::new(Some(guard.main_path()), Cli);

            path.update_with_backup(|t| {
                t.push(set_test_task());
                Ok(())
            })
            .unwrap();

            let before = fs::read_to_string(path.main_path()).unwrap();
            let err = path
                .update_with_backup(|_| Err(AppError::NothingToChange))
                .unwrap_err();
            match err {
                AppError::NothingToChange => {}
                _ => unreachable!(),
            }
            assert_eq!(fs::read_to_string(path.main_path()).unwrap(), before);
        }

        #[test]
        fn test_update_create_dir() {
            let guard = TempGuard::new("create_file");
            let file = format!("{}/sub/subsub/task.json", guard.main_path());
            let path = TaskStore::new(Some(file.clone()), Cli);

            path.update_with_backup(|t| {
                t.push(set_test_task());
                Ok(())
            })
            .unwrap();
            let load = path.load().unwrap();
            assert_eq!(load.len(), 1);
            assert_eq!(load[0].content(), "test_task1".to_string());
        }

        #[test]
        fn test_update_create_dir_err() {
            let path = TaskStore::new(Some("".to_string()), Cli);
            let err = path.update_with_backup(|_| Ok(())).unwrap_err();
            match err {
                AppError::Io {
                    operation,
                    path: _,
                    source,
                } => {
                    assert_eq!(operation, "open read write");
                    assert!(!source.to_string().is_empty());
                }
                _ => unreachable!(),
            }
        }
    }

    #[cfg(test)]
    mod tests_backup {
        use super::*;

        #[test] // 主文件损坏，备份为空
        fn test_backup1_fn_readonly() {
            let guard = TempGuard::new("test_backup1");
            let path = TaskStore::new(Some(guard.main_path()), Cli);

            write_file(path.main_path(), "{Illegal data");
            write_file(path.backup_path(), "");
            match path.load() {
                Err(AppError::Corrupted { path, source }) => {
                    assert_eq!(path, guard.main_path());
                    assert!(!source.to_string().is_empty());
                }
                _ => unreachable!(),
            }
            assert_eq!(
                fs::read_to_string(path.main_path()).unwrap(),
                "{Illegal data"
            );
        }

        #[test] // 主文件损坏，备份完好，从备份恢复
        fn test_backup2_fn_readonly() {
            let guard = TempGuard::new("test_backup2");
            let path = TaskStore::new(Some(guard.main_path()), Cli);

            write_file(path.main_path(), "{Illegal data");
            write_file(
                path.backup_path(),
                &serde_json::to_string_pretty(&vec![set_test_task()]).unwrap(),
            );
            let load = path.load().unwrap();
            assert_eq!(load, vec![set_test_task()]);
            let backup: Vec<Task> =
                serde_json::from_str(&fs::read_to_string(path.backup_path()).unwrap()).unwrap();
            assert_eq!(backup, load);
            assert_eq!(backup, vec![set_test_task()]);
        }

        #[test] // 主文件损坏，备份为空
        fn test_backup3_fn_readwrite() {
            let guard = TempGuard::new("test_backup3");
            let path = TaskStore::new(Some(guard.main_path()), Cli);

            write_file(path.main_path(), "{Illegal data");
            write_file(path.backup_path(), "");

            match path.update_with_backup(|t| {
                t.push(set_test_task());
                Ok(())
            }) {
                Err(AppError::Corrupted { path, source }) => {
                    assert_eq!(path, guard.main_path());
                    assert!(!source.to_string().is_empty());
                }
                _ => unreachable!(),
            }
            assert_eq!(
                fs::read_to_string(path.main_path()).unwrap(),
                "{Illegal data"
            );
        }

        #[test] // 主文件损坏，备份完好，从备份恢复
        fn test_backup4_fn_readwrite() {
            let guard = TempGuard::new("test_backup4");
            let path = TaskStore::new(Some(guard.main_path()), Cli);

            write_file(path.main_path(), "{Illegal data");
            write_file(
                path.backup_path(),
                &serde_json::to_string_pretty(&vec![set_test_task()]).unwrap(),
            );

            path.update_with_backup(|t| {
                t.push(set_test_task());
                Ok(())
            })
            .unwrap();
            let load = path.load().unwrap();
            assert_eq!(load.len(), 2);
            assert_eq!(load[0].content(), "test_task1".to_string());
            assert_eq!(load[1].content(), "test_task1".to_string());
            let backup: Vec<Task> =
                serde_json::from_str(&fs::read_to_string(path.backup_path()).unwrap()).unwrap();
            assert_eq!(backup, vec![set_test_task()]);
        }

        #[test] // 主文件确实，备份完好，从备份恢复
        fn test_backup5_fn_readwrite() {
            let guard = TempGuard::new("test_backup5");
            let path = TaskStore::new(Some(guard.main_path()), Cli);

            write_file(
                path.backup_path(),
                &serde_json::to_string_pretty(&vec![set_test_task()]).unwrap(),
            );

            path.update_with_backup(|t| {
                t.push(set_test_task());
                Ok(())
            })
            .unwrap();
            let load = path.load().unwrap();
            assert_eq!(load.len(), 2);
            assert_eq!(load[0].content(), "test_task1".to_string());
            assert_eq!(load[1].content(), "test_task1".to_string());
            let backup: Vec<Task> =
                serde_json::from_str(&fs::read_to_string(path.backup_path()).unwrap()).unwrap();
            assert_eq!(backup, vec![set_test_task()]);
        }

        /// 备份文件写入错误，直接抛出错误，而不是警告
        ///
        /// 使用linux的/dev/full模拟满盘无法写入的状态
        #[cfg(target_os = "linux")]
        #[test]
        fn test_backup_write_fail() {
            use std::os::unix::fs::symlink;
            let guard = TempGuard::new("backup_write_fail");
            let path = TaskStore::new(Some(guard.main_path()), Cli);

            write_file(
                path.main_path(),
                &serde_json::to_string_pretty(&vec![set_test_task()]).unwrap(),
            );
            symlink("/dev/full", path.backup_path()).unwrap();

            let before = fs::read_to_string(path.main_path()).unwrap();
            let err = path
                .update_with_backup(|t| {
                    t.push(set_test_task());
                    Ok(())
                })
                .unwrap_err();
            assert!(matches!(
                err,
                AppError::Io {
                    operation: _,
                    path: _,
                    source: _
                }
            ));
            assert_eq!(fs::read_to_string(path.main_path()).unwrap(), before);
        }
    }

    #[cfg(test)]
    mod tests_load_backup {
        use super::*;

        #[test]
        fn test_load_backup() {
            let guard = TempGuard::new("test_load_backup");
            let path = TaskStore::new(Some(guard.main_path()), Cli);

            write_file(
                path.backup_path(),
                &serde_json::to_string_pretty(&vec![set_test_task()]).unwrap(),
            );
            let tasks = path.load_backup().unwrap();
            assert_eq!(tasks, vec![set_test_task()]);
        }

        #[test]
        fn test_load_notexist_backup() {
            let guard = TempGuard::new("test_load_notexist_backup");
            let path = TaskStore::new(Some(guard.main_path()), Cli);

            let err = path.load_backup().unwrap_err();
            assert!(matches!(err, AppError::NothingToUndo));
        }

        #[test]
        fn test_load_empty_backup() {
            let guard = TempGuard::new("test_load_empty_backup");
            let path = TaskStore::new(Some(guard.main_path()), Cli);

            write_file(path.backup_path(), "");

            let err = path.load_backup().unwrap_err();
            assert!(matches!(err, AppError::NothingToUndo));
        }

        #[test]
        fn test_load_illegal_backup() {
            let guard = TempGuard::new("test_load_illegal_backup");
            let path = TaskStore::new(Some(guard.main_path()), Cli);

            write_file(path.backup_path(), "{Illegal data");
            let err = path.load_backup().unwrap_err();
            match err {
                AppError::Corrupted {
                    path: err_path,
                    source,
                } => {
                    assert_eq!(err_path, path.backup_path().to_string_lossy().to_string());
                    assert!(!source.to_string().is_empty());
                }
                _ => unreachable!(),
            }
        }
    }

    #[cfg(test)]
    mod tests_restore_backup {
        use super::*;

        #[test]
        fn test_restore_backup() {
            let guard = TempGuard::new("test_restore_backup");
            let path = TaskStore::new(Some(guard.main_path()), Cli);

            write_file(
                path.main_path(),
                &serde_json::to_string_pretty(&vec![set_test_task()]).unwrap(),
            );
            write_file(
                path.backup_path(),
                &serde_json::to_string_pretty(&vec![set_test_task(), set_test_task()]).unwrap(),
            );

            assert_eq!(path.load().unwrap(), vec![set_test_task()]);
            let backup: Vec<Task> =
                serde_json::from_str(&fs::read_to_string(path.backup_path()).unwrap()).unwrap();
            assert_eq!(backup, vec![set_test_task(), set_test_task()]);

            path.restore_backup(&backup).unwrap();
            assert_eq!(path.load().unwrap(), vec![set_test_task(), set_test_task()]);
            assert_eq!(
                path.load_backup().unwrap(),
                vec![set_test_task(), set_test_task()]
            );

            path.restore_backup(&backup).unwrap();
            assert_eq!(path.load().unwrap(), vec![set_test_task(), set_test_task()]);
            assert_eq!(
                path.load_backup().unwrap(),
                vec![set_test_task(), set_test_task()]
            );
        }

        #[test]
        fn test_restore_empty_backup() {
            let guard = TempGuard::new("test_restore_empty_backup");
            let path = TaskStore::new(Some(guard.main_path()), Cli);

            write_file(
                path.main_path(),
                &serde_json::to_string_pretty(&vec![set_test_task()]).unwrap(),
            );
            write_file(path.backup_path(), "");

            let err = path.restore_backup(&[]).unwrap_err();
            assert!(matches!(err, AppError::NothingToUndo));
        }

        #[test]
        fn test_restore_illegal_backup() {
            let guard = TempGuard::new("test_restore_illegal_backup");
            let path = TaskStore::new(Some(guard.main_path()), Cli);

            write_file(path.backup_path(), "{Illegal data");
            let err = path.restore_backup(&Vec::new()).unwrap_err();
            match err {
                AppError::Corrupted {
                    path: err_path,
                    source,
                } => {
                    assert_eq!(err_path, path.backup_path().to_string_lossy().to_string());
                    assert!(!source.to_string().is_empty());
                }
                _ => unreachable!(),
            }
        }

        #[test]
        fn test_restore_conflict_backup() {
            let guard = TempGuard::new("test_restore_conflict_backup");
            let path = TaskStore::new(Some(guard.main_path()), Cli);

            write_file(
                path.main_path(),
                &serde_json::to_string_pretty(&vec![set_test_task()]).unwrap(),
            );
            write_file(
                path.backup_path(),
                &serde_json::to_string_pretty(&vec![set_test_task(), set_test_task()]).unwrap(),
            );

            let conflict_snapshot = &[set_test_task()];
            let err = path.restore_backup(conflict_snapshot).unwrap_err();
            assert!(matches!(err, AppError::UndoConflict));
            assert_eq!(path.load().unwrap(), vec![set_test_task()]);
            assert_eq!(
                path.load_backup().unwrap(),
                vec![set_test_task(), set_test_task()]
            );
        }
        #[test]
        fn test_restore_empty_task() {
            let guard = TempGuard::new("test_restore_empty_task");
            let path = TaskStore::new(Some(guard.main_path()), Cli);

            write_file(
                path.main_path(),
                &serde_json::to_string_pretty(&vec![set_test_task()]).unwrap(),
            );
            write_file(path.backup_path(), "[]");

            path.restore_backup(&[]).unwrap();
            assert_eq!(path.load().unwrap(), vec![]);
            assert_eq!(path.load_backup().unwrap(), vec![]);
        }
    }
}
