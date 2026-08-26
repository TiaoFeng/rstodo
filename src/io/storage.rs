//! # storage模块
//!
//! 负责读取和保存数据：
//! - 包括主文件和备份文件
//! - 若主文件损坏会尝试读取备份文件
//! - 通过文件锁防止发生并发的读写冲突
use crate::error::{AppError, io_err};
use crate::task::Task;
use std::fs::File;
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// 文件来源枚举，用于标记读取任务列表的来源
/// - Main 主文件
/// - Backup 备份文件
#[derive(Debug, PartialEq)]
enum Origin {
    Main,
    Backup,
}

/// 核心tasks结构体，负责保存文件地址，实现公开的读写方法
///
/// 包括字段：
/// - main 主文件地址
/// - backup 备份文件地址
pub struct TaskStore {
    main: PathBuf,
    backup: PathBuf,
}

impl TaskStore {
    /// 用于初始化创建TaskStore实例
    /// ### Args:
    /// custom: `Option<String>` 用户自定义文件保存地址
    pub fn new(custom: Option<String>) -> Self {
        let main = custom.map(PathBuf::from).unwrap_or_else(Self::default_main);
        let mut backup = main.clone().into_os_string();
        backup.push(".bak");
        Self {
            main,
            backup: PathBuf::from(backup),
        }
    }

    /// 在未传入自定义地址时，利用data_dir生成默认地址，供new使用
    fn default_main() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rstodo")
            .join("task.json")
    }

    /// 返回主文件地址
    pub fn main_path(&self) -> &Path {
        &self.main
    }

    /// 返回备份文件地址
    pub fn backup_path(&self) -> &Path {
        &self.backup
    }

    /// Tasks只读方法
    /// 逻辑：
    /// 1.若主文件正常，只读主文件
    /// 2.若主文件不存在，走load_backup_fallback函数尝试读取备份文件
    /// 3.若主文件为空文件，走load_backup_fallback函数尝试读取备份文件
    /// 4.若主文件损坏，走load_backup_fallback函数尝试读取备份文件，若备份文件为空或损坏，报错而不覆盖
    pub fn load(&self) -> Result<Vec<Task>, AppError> {
        let file = match open_read(self.main_path()) {
            Ok(file) => file,
            Err(err) if is_not_found(&err) => {
                return load_backup_fallback(self.backup_path());
            }
            Err(e) => return Err(e),
        };
        lock_share(&file, self.main_path())?;
        match read_task_from(&file, self.main_path()) {
            Ok(Some(tasks)) => Ok(tasks),
            Ok(None) => load_backup_fallback(self.backup_path()),
            Err(AppError::Corrupted { path, source }) => {
                match load_backup_fallback(self.backup_path()) {
                    Ok(tasks) if !tasks.is_empty() => Ok(tasks),
                    _ => Err(AppError::Corrupted { path, source }),
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Tasks更新方法，同步更新备份
    ///
    /// 逻辑：
    /// 1.使用load_for_update函数读取(来源，任务列表)
    /// 2.若读取的是主文件，将主文件备份到副文件，之后再将操作覆盖到主文件
    pub fn update_with_backup(
        &self,
        f: impl FnOnce(&mut Vec<Task>) -> Result<(), AppError>,
    ) -> Result<(), AppError> {
        self.update(f, true)
    }

    /// Tasks更新方法，不更新备份
    ///
    /// 适用于排序，不改变备份文件，这样undo可以回到排序前的操作
    pub fn update_without_backup(
        &self,
        f: impl FnOnce(&mut Vec<Task>) -> Result<(), AppError>,
    ) -> Result<(), AppError> {
        self.update(f, false)
    }

    /// Task更新方法的实现
    ///
    /// 通过refresh_backup判断是否执行刷新动作
    fn update(
        &self,
        f: impl FnOnce(&mut Vec<Task>) -> Result<(), AppError>,
        refresh_backup: bool,
    ) -> Result<(), AppError> {
        create_dir(self.main_path())?;

        let main = open_read_write(self.main_path())?;
        let backup = open_read_write(self.backup_path())?;

        lock_private(&main, self.main_path())?;
        lock_private(&backup, self.backup_path())?;

        let (origin, mut tasks) =
            load_for_update(&main, self.main_path(), &backup, self.backup_path())?;
        f(&mut tasks)?;
        if refresh_backup
            && origin == Origin::Main
            && let Err(err) =
                copy_main_to_backup(&main, self.main_path(), &backup, self.backup_path())
        {
            eprintln!(">_< Warning: backup failed: {}", err);
        };
        overwrite(
            &main,
            serialize_tasks(&tasks, self.main_path())?.as_bytes(),
            self.main_path(),
        )?;
        Ok(())
    }

    /// 只读备份文件方法
    /// 调用load_backup_strict只读备份文件
    pub fn load_backup(&self) -> Result<Vec<Task>, AppError> {
        load_backup_strict(self.backup_path())
    }

    /// 提取备份文件并覆盖主文件
    /// 将备份文件提取，并通过restore_main_from_backup覆盖到主文件，以实现undo
    ///
    /// snapshot，用于对比undo预览的快照与目前进行操作的backup文件，以免被篡改
    ///
    /// 若snapshot与目前的备份文件不匹配，返回错误UndoConflict
    pub fn restore_backup(&self, snapshot: &[Task]) -> Result<(), AppError> {
        create_dir(self.main_path())?;
        let main = open_read_write(self.main_path())?;
        let backup = open_read_write(self.backup_path())?;
        lock_private(&main, self.main_path())?;
        lock_private(&backup, self.backup_path())?;

        let now = read_task_from(&backup, self.backup_path())?.unwrap_or_default();
        if snapshot == now {
            restore_main_from_backup(&main, self.main_path(), &backup, self.backup_path())
        } else {
            Err(AppError::UndoConflict)
        }
    }
}

// tier1
/// 将读取的字符串切片使用serde_json序列化为`Vec<Task>`
fn parse_tasks(text: &str, path: &Path) -> Result<Option<Vec<Task>>, AppError> {
    if text.trim().is_empty() {
        return Ok(None);
    }
    let tasks = serde_json::from_str(text).map_err(|err| AppError::Corrupted {
        path: path.to_string_lossy().to_string(),
        source: err,
    })?;
    Ok(Some(tasks))
}

/// 将`&[Task]`的数组使用serde_json格式化为用于保存的`String`
fn serialize_tasks(tasks: &[Task], path: &Path) -> Result<String, AppError> {
    let data =
        serde_json::to_string_pretty(tasks).map_err(|serde_json_err| AppError::Corrupted {
            path: path.to_string_lossy().to_string(),
            source: serde_json_err,
        })?;
    Ok(data)
}

// tier2
/// 捕获NotFound(未找到需要打开的文件)错误，用于后续特殊处理
fn is_not_found(err: &AppError) -> bool {
    matches!(
        err,
        AppError::Io {
            operation: _,
            path: _,
            source
        } if source.kind() == ErrorKind::NotFound
    )
}

/// 从备份中恢复数据的警告信息
fn warn_recovered_from_backup(backup_path: &Path) {
    eprintln!(
        ">_< Warning: main task file is missing or corrupted, loaded data from backup file '{}'. Changes since the last successful save may be lost.",
        backup_path.display()
    )
}

/// 根据地址，创建需要的文件夹
fn create_dir(path: &Path) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| io_err("create dir", path, err))?;
    };
    Ok(())
}

/// 传入文件`&Path`，打开并返回只读的文件句柄
fn open_read(path: &Path) -> Result<File, AppError> {
    let file = File::options()
        .read(true)
        .write(false)
        .create(false)
        .truncate(false)
        .open(path)
        .map_err(|err| io_err("open read", path, err))?;
    Ok(file)
}

/// 传入文件`&Path`，打开并返回可读写的文件句柄
fn open_read_write(path: &Path) -> Result<File, AppError> {
    let file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|err| io_err("open read write", path, err))?;
    Ok(file)
}

/// 将传入的`&File`句柄使用lock锁定，用于读写修改
fn lock_private(file: &File, path: &Path) -> Result<(), AppError> {
    file.lock()
        .map_err(|err| io_err("lock private", path, err))?;
    Ok(())
}

/// 将传入的`&File`句柄使用lock_shared锁定，用于只读项目
fn lock_share(file: &File, path: &Path) -> Result<(), AppError> {
    file.lock_shared()
        .map_err(|err| io_err("lock shared", path, err))?;
    Ok(())
}

/// 读取传入的`&File`句柄中的文件内容，返回文件全文的`String`字符串，用于序列化读取
fn read_string(file: &File, path: &Path) -> Result<String, AppError> {
    let mut f = file
        .try_clone()
        .map_err(|err| io_err("try clone", path, err))?;
    f.seek(SeekFrom::Start(0))
        .map_err(|err| io_err("seek to start", path, err))?;
    let mut text = String::new();
    f.read_to_string(&mut text)
        .map_err(|err| io_err("read to string", path, err))?;
    Ok(text)
}

/// 读取传入的`&File`句柄中的文件内容，返回文件全文的`Vec<u8>`字符比特，用于覆写其他文件
fn read_bytes(file: &File, path: &Path) -> Result<Vec<u8>, AppError> {
    let mut f = file
        .try_clone()
        .map_err(|err| io_err("try clone", path, err))?;
    f.seek(SeekFrom::Start(0))
        .map_err(|err| io_err("seek to start", path, err))?;
    let mut data: Vec<u8> = Vec::new();
    f.read_to_end(&mut data)
        .map_err(|err| io_err("read to end", path, err))?;
    Ok(data)
}

/// 使用传入的`&[u8]`字符比特，覆写到`&File`传入的文件中
fn overwrite(file: &File, data: &[u8], path: &Path) -> Result<(), AppError> {
    let mut f = file
        .try_clone()
        .map_err(|err| io_err("try clone", path, err))?;
    f.seek(SeekFrom::Start(0))
        .map_err(|err| io_err("seek to start", path, err))?;
    f.set_len(0).map_err(|err| io_err("set len 0", path, err))?;
    f.write_all(data)
        .map_err(|err| io_err("write all", path, err))?;
    f.flush().map_err(|err| io_err("flush", path, err))?;
    Ok(())
}

// tier3
/// 使用read_string读取文件的内容，并通过parse_tasks序列化为`Option<Vec<Task>>`返回
fn read_task_from(file: &File, path: &Path) -> Result<Option<Vec<Task>>, AppError> {
    let text = read_string(file, path)?;
    let tasks = parse_tasks(&text, path)?;
    Ok(tasks)
}

/// 将主文件通过read_bytes读取为字节比特，利用overwrite覆写入备份文件
fn copy_main_to_backup(
    main: &File,
    main_path: &Path,
    backup: &File,
    backup_path: &Path,
) -> Result<(), AppError> {
    let data = read_bytes(main, main_path)?;
    overwrite(backup, &data, backup_path)
}

// tier4
/// 从主文件或备份中读取任务列表，返回文件来源和任务列表
///
/// 逻辑：
/// - 若主文件正常，返回(Main, 主文件任务列表)
/// - 若主文件为空文件或损坏，尝试读取备份文件，若备份文件存在内容，发布警告，恢复数据，返回(Backup, 备份文件任务列表)
/// - 若主文件为空文件，尝试读取备份文件，若备份文件也为空，返回空列表重新开始
/// - 若主文件损坏，尝试读取备份文件，若备份文件为空，直接返回错误
fn load_for_update(
    main: &File,
    main_path: &Path,
    backup: &File,
    backup_path: &Path,
) -> Result<(Origin, Vec<Task>), AppError> {
    match read_task_from(main, main_path) {
        Ok(Some(tasks)) => Ok((Origin::Main, tasks)),
        Ok(None) => match read_task_from(backup, backup_path)? {
            Some(tasks) => {
                warn_recovered_from_backup(backup_path);
                Ok((Origin::Backup, tasks))
            }
            None => Ok((Origin::Main, Vec::new())),
        },
        Err(err) => match read_task_from(backup, backup_path)? {
            Some(tasks) => {
                warn_recovered_from_backup(backup_path);
                Ok((Origin::Backup, tasks))
            }
            None => Err(err),
        },
    }
}

/// 为TaskStore中只读的方法提供主文件为空或损坏后的回落
///
/// 逻辑：
/// - 若备份文件存在内容，抛出警告并返回备份文件中的内容
/// - 若备份文件为空，返回空列表
/// - 若备份文件损坏，返回错误
fn load_backup_fallback(backup_path: &Path) -> Result<Vec<Task>, AppError> {
    let backup_file = match open_read(backup_path) {
        Ok(file) => file,
        Err(err) if is_not_found(&err) => return Ok(Vec::new()),
        Err(err) => return Err(err),
    };
    lock_share(&backup_file, backup_path)?;
    match read_task_from(&backup_file, backup_path) {
        Ok(Some(tasks)) => {
            warn_recovered_from_backup(backup_path);
            Ok(tasks)
        }
        Ok(None) => Ok(Vec::new()),
        Err(err) => Err(err),
    }
}

/// 为TaskStore中只读的读取备份文件提供实现函数
///
/// 逻辑：
/// - 若备份文件不存在，返回`NothingToUndo`错误
/// - 若备份文件存在且不为空，返回备份文件中的任务列表
/// - 其他情况，返回`NothingToUndo`错误
fn load_backup_strict(backup_path: &Path) -> Result<Vec<Task>, AppError> {
    let backup_file = match open_read(backup_path) {
        Ok(file) => file,
        Err(err) if is_not_found(&err) => return Err(AppError::NothingToUndo),
        Err(e) => return Err(e),
    };
    lock_share(&backup_file, backup_path)?;
    match read_task_from(&backup_file, backup_path)? {
        Some(tasks) if !tasks.is_empty() => Ok(tasks),
        _ => Err(AppError::NothingToUndo),
    }
}

/// 为TaskStore中使用备份覆盖主文件的undo操作提供实现函数
///
/// 逻辑：
/// - 读取备份文件中的任务列表，若没有任务，返回`NothingToUndo`错误
/// - 读取备份文件的字符比特，覆写主文件，完成undo
fn restore_main_from_backup(
    main: &File,
    main_path: &Path,
    backup: &File,
    backup_path: &Path,
) -> Result<(), AppError> {
    if read_task_from(backup, backup_path)?.is_none() {
        return Err(AppError::NothingToUndo);
    }
    let data = read_bytes(backup, backup_path)?;
    overwrite(main, &data, main_path)
}

/// 单元测试
#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use crate::task::Priority;

    use super::*;
    use crate::test_helpers::TempGuard;
    use std::{fs, os::unix::fs::PermissionsExt};

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
        let store = TaskStore::new(Some("test1.json".to_string()));
        assert_eq!(store.main_path(), Path::new("test1.json"));
        assert_eq!(store.backup_path(), Path::new("test1.json.bak"));

        let default_store = TaskStore::new(None);
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
            let path = TaskStore::new(Some(guard.main_path()));

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
            let path = TaskStore::new(Some(guard.main_path()));

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
            let path = TaskStore::new(Some(guard.main_path()));

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
            let path = TaskStore::new(Some(guard.main_path()));
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
                assert_eq!(task._content(), "test_task1".to_string());
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
            let path = TaskStore::new(Some(guard.main_path()));

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
            let path = TaskStore::new(Some(file.clone()));

            path.update_with_backup(|t| {
                t.push(set_test_task());
                Ok(())
            })
            .unwrap();
            let load = path.load().unwrap();
            assert_eq!(load.len(), 1);
            assert_eq!(load[0]._content(), "test_task1".to_string());
        }

        #[test]
        fn test_update_create_dir_err() {
            let path = TaskStore::new(Some("".to_string()));
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
            let path = TaskStore::new(Some(guard.main_path()));

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
            let path = TaskStore::new(Some(guard.main_path()));

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
            let path = TaskStore::new(Some(guard.main_path()));

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
            let path = TaskStore::new(Some(guard.main_path()));

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
            assert_eq!(load[0]._content(), "test_task1".to_string());
            assert_eq!(load[1]._content(), "test_task1".to_string());
            let backup: Vec<Task> =
                serde_json::from_str(&fs::read_to_string(path.backup_path()).unwrap()).unwrap();
            assert_eq!(backup, vec![set_test_task()]);
        }

        #[test] // 主文件确实，备份完好，从备份恢复
        fn test_backup5_fn_readwrite() {
            let guard = TempGuard::new("test_backup5");
            let path = TaskStore::new(Some(guard.main_path()));

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
            assert_eq!(load[0]._content(), "test_task1".to_string());
            assert_eq!(load[1]._content(), "test_task1".to_string());
            let backup: Vec<Task> =
                serde_json::from_str(&fs::read_to_string(path.backup_path()).unwrap()).unwrap();
            assert_eq!(backup, vec![set_test_task()]);
        }
    }

    #[cfg(test)]
    mod tests_load_backup {
        use super::*;

        #[test]
        fn test_load_backup() {
            let guard = TempGuard::new("test_load_backup");
            let path = TaskStore::new(Some(guard.main_path()));

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
            let path = TaskStore::new(Some(guard.main_path()));

            let err = path.load_backup().unwrap_err();
            assert!(matches!(err, AppError::NothingToUndo));
        }

        #[test]
        fn test_load_empty_backup() {
            let guard = TempGuard::new("test_load_empty_backup");
            let path = TaskStore::new(Some(guard.main_path()));

            write_file(path.backup_path(), "");

            let err = path.load_backup().unwrap_err();
            assert!(matches!(err, AppError::NothingToUndo));
        }

        #[test]
        fn test_load_illegal_backup() {
            let guard = TempGuard::new("test_load_illegal_backup");
            let path = TaskStore::new(Some(guard.main_path()));

            write_file(path.backup_path(), "{Illegal data");
            let err = path.load_backup().unwrap_err();
            match err {
                AppError::Corrupted {
                    path: err_path,
                    source,
                } => {
                    assert_eq!(err_path, path.backup);
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
            let path = TaskStore::new(Some(guard.main_path()));

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
            let path = TaskStore::new(Some(guard.main_path()));

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
            let path = TaskStore::new(Some(guard.main_path()));

            write_file(path.backup_path(), "{Illegal data");
            let err = path.restore_backup(&Vec::new()).unwrap_err();
            match err {
                AppError::Corrupted {
                    path: err_path,
                    source,
                } => {
                    assert_eq!(err_path, path.backup);
                    assert!(!source.to_string().is_empty());
                }
                _ => unreachable!(),
            }
        }

        #[test]
        fn test_restore_conflict_backup() {
            let guard = TempGuard::new("test_restore_conflict_backup");
            let path = TaskStore::new(Some(guard.main_path()));

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
    }
}
