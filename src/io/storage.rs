use crate::error::{AppError, io_err};
use crate::task::Task;
use std::fs::File;
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
enum Origin {
    Main,
    Backup,
}

pub struct TaskStore {
    main: PathBuf,
    backup: PathBuf,
}

impl TaskStore {
    pub fn new(custom: Option<String>) -> Self {
        let main = custom.map(PathBuf::from).unwrap_or_else(Self::default_main);
        let mut backup = main.clone().into_os_string();
        backup.push(".bak");
        Self {
            main,
            backup: PathBuf::from(backup),
        }
    }

    fn default_main() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rstodo")
            .join("task.json")
    }

    pub fn main_path(&self) -> &Path {
        &self.main
    }

    pub fn backup_path(&self) -> &Path {
        &self.backup
    }

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

    pub fn update(
        &self,
        f: impl FnOnce(&mut Vec<Task>) -> Result<(), AppError>,
    ) -> Result<(), AppError> {
        create_dir(self.main_path())?;

        let main = open_read_write(self.main_path())?;
        let backup = open_read_write(self.backup_path())?;

        lock_private(&main, self.main_path())?;
        lock_private(&backup, self.backup_path())?;

        let (origin, mut tasks) =
            load_for_update(&main, self.main_path(), &backup, self.backup_path())?;
        f(&mut tasks)?;
        if origin == Origin::Main
            && let Err(err) =
                copy_main_to_backup(&main, self.main_path(), &backup, self.backup_path())
        {
            eprintln!("Warning: backup failed: {}", err);
        };
        overwrite(
            &main,
            serialize_tasks(&tasks, self.main_path())?.as_bytes(),
            self.main_path(),
        )?;
        Ok(())
    }

    pub fn load_backup(&self) -> Result<Vec<Task>, AppError> {
        load_backup_strict(self.backup_path())
    }

    pub fn restore_backup(&self) -> Result<(), AppError> {
        create_dir(self.main_path())?;
        let main = open_read_write(self.main_path())?;
        let backup = open_read_write(self.backup_path())?;
        lock_private(&main, self.main_path())?;
        lock_private(&backup, self.backup_path())?;
        restore_main_from_backup(&main, self.main_path(), &backup, self.backup_path())
    }
}

// tier1
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

fn serialize_tasks(tasks: &[Task], path: &Path) -> Result<String, AppError> {
    let data =
        serde_json::to_string_pretty(tasks).map_err(|serde_json_err| AppError::Corrupted {
            path: path.to_string_lossy().to_string(),
            source: serde_json_err,
        })?;
    Ok(data)
}

// tier2
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

fn create_dir(path: &Path) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| io_err("create dir", path, err))?;
    };
    Ok(())
}

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

fn lock_private(file: &File, path: &Path) -> Result<(), AppError> {
    file.lock()
        .map_err(|err| io_err("lock private", path, err))?;
    Ok(())
}

fn lock_share(file: &File, path: &Path) -> Result<(), AppError> {
    file.lock_shared()
        .map_err(|err| io_err("lock shared", path, err))?;
    Ok(())
}

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
fn read_task_from(file: &File, path: &Path) -> Result<Option<Vec<Task>>, AppError> {
    let text = read_string(file, path)?;
    let tasks = parse_tasks(&text, path)?;
    Ok(tasks)
}

fn copy_main_to_backup(
    main: &File,
    main_path: &Path,
    backup: &File,
    backup_path: &Path,
) -> Result<(), AppError> {
    let data = read_bytes(main, main_path)?;
    overwrite(backup, &data, backup_path)
}

fn warn_recovered_from_backup(backup_path: &Path) {
    eprintln!(
        "Warning: main task file is missing or corrupted, loaded data from backup file '{}'. Changes since the last successful save may be lost.",
        backup_path.display()
    )
}

// tier4
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

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use crate::task::Priority;

    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt, process};

    fn temp_path(name: &str) -> String {
        std::env::temp_dir()
            .join(format!("{}_rstodo_test_{}.json", process::id(), name))
            .to_string_lossy()
            .to_string()
    }

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

    #[test]
    fn test_load_tasks() {
        let file = temp_path("load");
        let path = TaskStore::new(Some(file.clone()));
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());

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
                assert_eq!(path, file)
            }
            _ => unreachable!(),
        }

        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());
    }

    #[test]
    fn test_update() {
        let file = temp_path("update");
        let path = TaskStore::new(Some(file.clone()));
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());
        for _ in 0..5 {
            path.update(|t| {
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
            assert_eq!(task.description(), Some("desc".to_string()));
            assert_eq!(
                task.deadline(),
                Some("2000-1-1T12:00:00+00:00".parse::<DateTime<Utc>>().unwrap())
            );
            assert_eq!(task.priority(), Priority::High);
        }

        path.update(|t| {
            t.clear();
            Ok(())
        })
        .unwrap();
        let load = path.load().unwrap();
        assert_eq!(load, Vec::new());
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());
    }

    #[test]
    fn test_update_err() {
        let file = temp_path("update_err");
        let path = TaskStore::new(Some(file.clone()));
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());

        path.update(|t| {
            t.push(set_test_task());
            Ok(())
        })
        .unwrap();

        let before = fs::read_to_string(path.main_path()).unwrap();
        let err = path.update(|_| Err(AppError::NothingToChange)).unwrap_err();
        match err {
            AppError::NothingToChange => {}
            _ => unreachable!(),
        }
        assert_eq!(fs::read_to_string(path.main_path()).unwrap(), before);
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());
    }

    #[test]
    fn test_load_tasks_err() {
        let file = temp_path("load_err_not_utf8");
        let path = TaskStore::new(Some(file.clone()));
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());

        fs::write(path.main_path(), [0xFF, 0xFF]).unwrap();
        let err = path.load().unwrap_err();
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());

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
        let file = temp_path("permission_denied");
        let path = TaskStore::new(Some(file.clone()));
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());

        fs::write(path.main_path(), "test_content").unwrap();
        assert_eq!(
            fs::read_to_string(path.main_path()).unwrap(),
            "test_content".to_string()
        );

        let no_permission = 0o000;
        let std_permission = 0o644;
        fs::set_permissions(path.main_path(), fs::Permissions::from_mode(no_permission)).unwrap();
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
        fs::set_permissions(path.main_path(), fs::Permissions::from_mode(std_permission)).unwrap();
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());
    }

    #[test]
    fn test_update_create_dir() {
        let dir = temp_path("create_file");
        let file = format!("{}/sub/subsub/task.json", dir);
        let path = TaskStore::new(Some(file.clone()));
        let _ = fs::remove_dir_all(&dir);

        path.update(|t| {
            t.push(set_test_task());
            Ok(())
        })
        .unwrap();
        let load = path.load().unwrap();
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(load.len(), 1);
        assert_eq!(load[0]._content(), "test_task1".to_string());
    }

    #[test]
    fn test_update_create_dir_err() {
        let path = TaskStore::new(Some("".to_string()));
        let err = path.update(|_| Ok(())).unwrap_err();
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

    #[test] // 主文件损坏，备份为空
    fn test_backup1_fn_readonly() {
        let file = temp_path("test_backup1");
        let path = TaskStore::new(Some(file.clone()));
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());

        write_file(path.main_path(), "{Illegal data");
        write_file(path.backup_path(), "");
        match path.load() {
            Err(AppError::Corrupted { path, source }) => {
                assert_eq!(path, file);
                assert!(!source.to_string().is_empty());
            }
            _ => unreachable!(),
        }
        assert_eq!(
            fs::read_to_string(path.main_path()).unwrap(),
            "{Illegal data"
        );

        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());
    }

    #[test] // 主文件损坏，备份完好，从备份恢复
    fn test_backup2_fn_readonly() {
        let file = temp_path("test_backup2");
        let path = TaskStore::new(Some(file.clone()));
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());

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

        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());
    }

    #[test] // 主文件损坏，备份为空
    fn test_backup3_fn_readwrite() {
        let file = temp_path("test_backup3");
        let path = TaskStore::new(Some(file.clone()));
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());

        write_file(path.main_path(), "{Illegal data");
        write_file(path.backup_path(), "");

        match path.update(|t| {
            t.push(set_test_task());
            Ok(())
        }) {
            Err(AppError::Corrupted { path, source }) => {
                assert_eq!(path, file);
                assert!(!source.to_string().is_empty());
            }
            _ => unreachable!(),
        }
        assert_eq!(
            fs::read_to_string(path.main_path()).unwrap(),
            "{Illegal data"
        );

        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());
    }

    #[test] // 主文件损坏，备份完好，从备份恢复
    fn test_backup4_fn_readwrite() {
        let file = temp_path("test_backup4");
        let path = TaskStore::new(Some(file.clone()));
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());

        write_file(path.main_path(), "{Illegal data");
        write_file(
            path.backup_path(),
            &serde_json::to_string_pretty(&vec![set_test_task()]).unwrap(),
        );

        path.update(|t| {
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

        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());
    }

    #[test] // 主文件确实，备份完好，从备份恢复
    fn test_backup5_fn_readwrite() {
        let file = temp_path("test_backup5");
        let path = TaskStore::new(Some(file.clone()));
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());

        write_file(
            path.backup_path(),
            &serde_json::to_string_pretty(&vec![set_test_task()]).unwrap(),
        );

        path.update(|t| {
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

        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());
    }

    #[test]
    fn test_load_backup() {
        let file = temp_path("test_load_backup");
        let path = TaskStore::new(Some(file.clone()));
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());

        let err = path.load_backup().unwrap_err();
        assert!(matches!(err, AppError::NothingToUndo));

        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());

        write_file(
            path.backup_path(),
            &serde_json::to_string_pretty(&vec![set_test_task()]).unwrap(),
        );

        let tasks = path.load_backup().unwrap();
        assert_eq!(tasks, vec![set_test_task()]);

        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());

        write_file(path.backup_path(), "");

        let err = path.load_backup().unwrap_err();
        assert!(matches!(err, AppError::NothingToUndo));

        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());

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
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());
    }

    #[test]
    fn test_restore_backup() {
        let file = temp_path("test_restore_backup");
        let path = TaskStore::new(Some(file.clone()));
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());
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

        path.restore_backup().unwrap();
        assert_eq!(path.load().unwrap(), vec![set_test_task(), set_test_task()]);
        assert_eq!(
            path.load_backup().unwrap(),
            vec![set_test_task(), set_test_task()]
        );

        path.restore_backup().unwrap();
        assert_eq!(path.load().unwrap(), vec![set_test_task(), set_test_task()]);
        assert_eq!(
            path.load_backup().unwrap(),
            vec![set_test_task(), set_test_task()]
        );

        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());
        write_file(
            path.main_path(),
            &serde_json::to_string_pretty(&vec![set_test_task()]).unwrap(),
        );
        write_file(path.backup_path(), "");

        let err = path.restore_backup().unwrap_err();
        assert!(matches!(err, AppError::NothingToUndo));

        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());

        write_file(path.backup_path(), "{Illegal data");
        let err = path.restore_backup().unwrap_err();
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
        let _ = fs::remove_file(path.main_path());
        let _ = fs::remove_file(path.backup_path());
    }
}
