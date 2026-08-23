use crate::error::{AppError, io_err};
use crate::task::Task;
use std::fs::File;
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

pub enum FilePath {
    Custom(String),
    Default,
}

impl FilePath {
    pub fn new(path: Option<String>) -> Self {
        match path {
            Some(p) => FilePath::Custom(p),
            None => FilePath::Default,
        }
    }

    pub fn path(&self) -> String {
        match self {
            FilePath::Custom(p) => p.clone(),
            FilePath::Default => {
                let mut dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
                dir.push("rstodo");
                dir.push("task.json");
                dir.to_string_lossy().to_string()
            }
        }
    }

    pub fn backup_path(&self) -> String {
        let main_path = self.path();
        format!("{}.bak", main_path)
    }
}

enum LoadState {
    Normal,
    Recovered,
}

pub fn update_tasks(
    path: &FilePath,
    f: impl FnOnce(&mut Vec<Task>) -> Result<(), AppError>,
) -> Result<(), AppError> {
    if let Some(parent) = std::path::Path::new(&path.path()).parent() {
        std::fs::create_dir_all(parent).map_err(|err| io_err("create dir", path.path(), err))?;
    }
    let file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path.path())
        .map_err(|err| io_err("create a read-write handle", path.path(), err))?;
    let backup_file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path.backup_path())
        .map_err(|err| io_err("create a read-write handle", path.path(), err))?;

    file.lock()
        .map_err(|err| io_err("lock main file", path.path(), err))?;
    backup_file
        .lock()
        .map_err(|err| io_err("lock backup file", path.backup_path(), err))?;
    let (mut tasks, state) =
        load_tasks_write_read(&file, &backup_file, &path.path(), &path.backup_path())?;
    f(&mut tasks)?;
    save_to(
        &file,
        &backup_file,
        &tasks,
        &path.path(),
        &path.backup_path(),
        state,
    )?;
    file.unlock()
        .map_err(|err| io_err("unlock main file", path.path(), err))?;
    backup_file
        .unlock()
        .map_err(|err| io_err("unlock backup file", path.path(), err))?;
    Ok(())
}

pub fn load_tasks_read_only(path: &FilePath) -> Result<Vec<Task>, AppError> {
    let file = match File::options()
        .read(true)
        .write(false)
        .create(false)
        .truncate(false)
        .open(path.path())
    {
        Ok(f) => f,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            return try_load_from_backup_read_only(path);
        }
        Err(err) => {
            return Err(io_err("open main file", path.path(), err));
        }
    };
    file.lock_shared()
        .map_err(|err| io_err("shared lock main file", path.path(), err))?;

    match load_from(&file, &path.path()) {
        Ok(Some(tasks)) => Ok(tasks),
        Ok(None) => try_load_from_backup_read_only(path),
        Err(AppError::Corrupted { path: _, source }) => {
            eprintln!(
                "Warning: file: {} corrupted: {}, trying backup...",
                path.path(),
                source
            );
            match try_load_from_backup_read_only(path) {
                Ok(tasks) if tasks.is_empty() => Err(AppError::Corrupted {
                    path: path.path(),
                    source,
                }),
                Ok(tasks) => Ok(tasks),
                Err(_) => Err(AppError::Corrupted {
                    path: path.path(),
                    source,
                }),
            }
        }
        Err(err) => Err(err),
    }
}

fn load_from(file: &File, path: &str) -> Result<Option<Vec<Task>>, AppError> {
    let mut f = file
        .try_clone()
        .map_err(|err| io_err("clone file", path.to_string(), err))?;
    f.seek(SeekFrom::Start(0))
        .map_err(|err| io_err("seek to the beginning", path.to_string(), err))?;
    let mut text = String::new();
    f.read_to_string(&mut text)
        .map_err(|err| io_err("read tasks", path.to_string(), err))?;
    if text.trim().is_empty() {
        return Ok(None);
    }

    let tasks: Vec<Task> =
        serde_json::from_str(&text).map_err(|serde_json_err| AppError::Corrupted {
            path: path.to_string(),
            source: serde_json_err,
        })?;

    Ok(Some(tasks))
}

fn save_to(
    file: &File,
    backup_file: &File,
    tasks: &[Task],
    path: &str,
    backup_path: &str,
    state: LoadState,
) -> Result<(), AppError> {
    if let LoadState::Normal = state
        && let Err(err) = backup(file, backup_file, path, backup_path)
    {
        eprintln!("Warning: backup failed: {}", err);
    }

    let mut f = file
        .try_clone()
        .map_err(|err| io_err("clone main file", path.to_string(), err))?;
    let data =
        serde_json::to_string_pretty(tasks).map_err(|serde_json_err| AppError::Corrupted {
            path: path.to_string(),
            source: serde_json_err,
        })?;
    f.seek(SeekFrom::Start(0))
        .map_err(|err| io_err("seek to the beginning", path.to_string(), err))?;
    f.set_len(0)
        .map_err(|err| io_err("reset file for saving", path.to_string(), err))?;
    f.write_all(data.as_bytes())
        .map_err(|err| io_err("write main file", path.to_string(), err))?;
    f.flush()
        .map_err(|err| io_err("flush", path.to_string(), err))?;
    Ok(())
}

fn load_tasks_write_read(
    file: &File,
    backup_file: &File,
    path: &str,
    backup_path: &str,
) -> Result<(Vec<Task>, LoadState), AppError> {
    match load_from(file, path) {
        Ok(Some(tasks)) => Ok((tasks, LoadState::Normal)),
        Ok(None) => match load_from(backup_file, backup_path)? {
            Some(tasks) => {
                warn_recovered_from_backup(backup_path);
                Ok((tasks, LoadState::Recovered))
            }
            None => Ok((Vec::new(), LoadState::Normal)),
        },
        Err(AppError::Corrupted { path, source }) => {
            eprintln!(
                "Warning: file: {} corrupted: {}, trying backup...",
                path, source
            );
            match load_from(backup_file, backup_path)? {
                Some(tasks) => {
                    warn_recovered_from_backup(backup_path);
                    Ok((tasks, LoadState::Recovered))
                }
                None => Err(AppError::Corrupted { path, source }),
            }
        }
        Err(err) => Err(err),
    }
}

fn try_load_from_backup_read_only(path: &FilePath) -> Result<Vec<Task>, AppError> {
    let backup_file = match File::options()
        .read(true)
        .write(false)
        .create(false)
        .truncate(false)
        .open(path.backup_path())
    {
        Ok(f) => f,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(io_err("open backup file", path.backup_path(), err)),
    };

    backup_file
        .lock_shared()
        .map_err(|err| io_err("lock backup file", path.backup_path(), err))?;
    let result = match load_from(&backup_file, &path.backup_path()) {
        Ok(None) => Ok(Vec::new()),
        Ok(Some(tasks)) => {
            warn_recovered_from_backup(&path.backup_path());
            Ok(tasks)
        }
        Err(err) => Err(err),
    };
    backup_file
        .unlock()
        .map_err(|err| io_err("unlock backup file", path.backup_path(), err))?;
    result
}

fn backup(file: &File, backup_file: &File, path: &str, backup_path: &str) -> Result<(), AppError> {
    let mut f = file
        .try_clone()
        .map_err(|err| io_err("clone main file", path.to_string(), err))?;
    f.seek(SeekFrom::Start(0))
        .map_err(|err| io_err("seek to the beginning", path.to_string(), err))?;
    let mut content = Vec::new();
    f.read_to_end(&mut content)
        .map_err(|err| io_err("read main file", path.to_string(), err))?;

    let mut backf = backup_file
        .try_clone()
        .map_err(|err| io_err("clone backup file", backup_path.to_string(), err))?;
    backf
        .seek(SeekFrom::Start(0))
        .map_err(|err| io_err("seek to the beginning", backup_path.to_string(), err))?;
    backf
        .set_len(0)
        .map_err(|err| io_err("reset file for backup", backup_path.to_string(), err))?;
    backf
        .write_all(&content)
        .map_err(|err| io_err("write backup file", backup_path.to_string(), err))?;
    backf
        .flush()
        .map_err(|err| io_err("flush", backup_path.to_string(), err))?;
    Ok(())
}

fn warn_recovered_from_backup(backup_path: &str) {
    eprintln!(
        "Warning: main task file is missing or corrupted,\
        loaded data from backup file '{}'.\
        Changes since the last successful save may be lost.",
        backup_path
    )
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

    fn write_file(path: &str, contents: &str) {
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
        assert_eq!(
            FilePath::new(Some("test1.json".to_string())).path(),
            "test1.json"
        );
        let path_default = FilePath::new(None).path();
        assert!(path_default.contains("task.json"));
        assert!(path_default.contains("rstodo"));
    }

    #[test]
    fn test_load_tasks() {
        let file = temp_path("load");
        let path = FilePath::new(Some(file.clone()));
        let _ = fs::remove_file(&file);
        let _ = fs::remove_file(path.backup_path());

        assert_eq!(load_tasks_read_only(&path).unwrap(), Vec::new());

        write_file(&file, "");
        assert_eq!(load_tasks_read_only(&path).unwrap(), Vec::new());

        write_file(
            &file,
            &serde_json::to_string_pretty(&vec![set_test_task()]).unwrap(),
        );
        let load = load_tasks_read_only(&path).unwrap();
        assert_eq!(load, vec![set_test_task()]);

        write_file(&file, "{Illegal data");
        match load_tasks_read_only(&path) {
            Err(AppError::Corrupted { path, source: _ }) => {
                assert_eq!(path, file)
            }
            _ => unreachable!(),
        }

        let _ = fs::remove_file(&file);
        let _ = fs::remove_file(path.backup_path());
    }

    #[test]
    fn test_update() {
        let file = temp_path("update");
        let path = FilePath::new(Some(file.clone()));
        let _ = fs::remove_file(&file);
        let _ = fs::remove_file(path.backup_path());
        for _ in 0..5 {
            update_tasks(&path, |t| {
                t.push(set_test_task());
                Ok(())
            })
            .unwrap();
        }
        let load = load_tasks_read_only(&path).unwrap();
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

        update_tasks(&path, |t| {
            t.clear();
            Ok(())
        })
        .unwrap();
        let load = load_tasks_read_only(&path).unwrap();
        assert_eq!(load, Vec::new());
        let _ = fs::remove_file(&file);
        let _ = fs::remove_file(path.backup_path());
    }

    #[test]
    fn test_update_err() {
        let file = temp_path("update_err");
        let path = FilePath::new(Some(file.clone()));
        let _ = fs::remove_file(&file);
        let _ = fs::remove_file(path.backup_path());

        update_tasks(&path, |t| {
            t.push(set_test_task());
            Ok(())
        })
        .unwrap();

        let before = fs::read_to_string(&file).unwrap();
        let err = update_tasks(&path, |_| Err(AppError::NothingToChange)).unwrap_err();
        match err {
            AppError::NothingToChange => {}
            _ => unreachable!(),
        }
        assert_eq!(fs::read_to_string(&file).unwrap(), before);
        let _ = fs::remove_file(&file);
        let _ = fs::remove_file(path.backup_path());
    }

    #[test]
    fn test_load_tasks_err() {
        let file = temp_path("load_err_not_utf8");
        let path = FilePath::new(Some(file.clone()));
        let _ = fs::remove_file(&file);
        let _ = fs::remove_file(path.backup_path());

        fs::write(&file, [0xFF, 0xFF]).unwrap();
        let err = load_tasks_read_only(&path).unwrap_err();
        let _ = fs::remove_file(&file);
        let _ = fs::remove_file(path.backup_path());

        match err {
            AppError::Io {
                operation,
                path: _,
                source,
            } => {
                assert_eq!(operation, "read tasks");
                assert!(!source.to_string().is_empty());
            }
            _ => unreachable!(),
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_permission_denied() {
        let file = temp_path("permission_denied");
        let path = FilePath::new(Some(file.clone()));
        let _ = fs::remove_file(&file);
        let _ = fs::remove_file(path.backup_path());

        fs::write(&file, "test_content").unwrap();
        assert_eq!(
            fs::read_to_string(&file).unwrap(),
            "test_content".to_string()
        );

        let no_permission = 0o000;
        let std_permission = 0o644;
        fs::set_permissions(&file, fs::Permissions::from_mode(no_permission)).unwrap();
        let err = load_tasks_read_only(&path).unwrap_err();
        match err {
            AppError::Io {
                operation,
                path: _,
                source,
            } => {
                assert_eq!(operation, "open main file");
                assert!(!source.to_string().is_empty());
            }
            _ => unreachable!(),
        }
        fs::set_permissions(&file, fs::Permissions::from_mode(std_permission)).unwrap();
        let _ = fs::remove_file(&file);
        let _ = fs::remove_file(path.backup_path());
    }

    #[test]
    fn test_update_create_dir() {
        let dir = temp_path("create_file");
        let file = format!("{}/sub/subsub/task.json", dir);
        let path = FilePath::new(Some(file.clone()));
        let _ = fs::remove_dir_all(&dir);

        update_tasks(&path, |t| {
            t.push(set_test_task());
            Ok(())
        })
        .unwrap();
        let load = load_tasks_read_only(&path).unwrap();
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(load.len(), 1);
        assert_eq!(load[0]._content(), "test_task1".to_string());
    }

    #[test]
    fn test_update_create_dir_err() {
        let path = FilePath::new(Some("".to_string()));
        let err = update_tasks(&path, |_| Ok(())).unwrap_err();
        match err {
            AppError::Io {
                operation,
                path: _,
                source,
            } => {
                assert_eq!(operation, "create a read-write handle");
                assert!(!source.to_string().is_empty());
            }
            _ => unreachable!(),
        }
    }

    #[test] // 主文件损坏，备份为空
    fn test_backup1_fn_readonly() {
        let file = temp_path("test_backup1");
        let path = FilePath::new(Some(file.clone()));
        let _ = fs::remove_file(&file);
        let _ = fs::remove_file(path.backup_path());

        write_file(&file, "{Illegal data");
        write_file(&path.backup_path(), "");
        match load_tasks_read_only(&path) {
            Err(AppError::Corrupted { path, source }) => {
                assert_eq!(path, file);
                assert!(!source.to_string().is_empty());
            }
            _ => unreachable!(),
        }
        assert_eq!(fs::read_to_string(&file).unwrap(), "{Illegal data");

        let _ = fs::remove_file(&file);
        let _ = fs::remove_file(path.backup_path());
    }

    #[test] // 主文件损坏，备份完好，从备份恢复
    fn test_backup2_fn_readonly() {
        let file = temp_path("test_backup2");
        let path = FilePath::new(Some(file.clone()));
        let _ = fs::remove_file(&file);
        let _ = fs::remove_file(path.backup_path());

        write_file(&file, "{Illegal data");
        write_file(
            &path.backup_path(),
            &serde_json::to_string_pretty(&vec![set_test_task()]).unwrap(),
        );
        let load = load_tasks_read_only(&path).unwrap();
        assert_eq!(load, vec![set_test_task()]);
        let backup: Vec<Task> =
            serde_json::from_str(&fs::read_to_string(path.backup_path()).unwrap()).unwrap();
        assert_eq!(backup, load);
        assert_eq!(backup, vec![set_test_task()]);

        let _ = fs::remove_file(&file);
        let _ = fs::remove_file(path.backup_path());
    }

    #[test] // 主文件损坏，备份为空
    fn test_backup3_fn_readwrite() {
        let file = temp_path("test_backup3");
        let path = FilePath::new(Some(file.clone()));
        let _ = fs::remove_file(&file);
        let _ = fs::remove_file(path.backup_path());

        write_file(&file, "{Illegal data");
        write_file(&path.backup_path(), "");

        match update_tasks(&path, |t| {
            t.push(set_test_task());
            Ok(())
        }) {
            Err(AppError::Corrupted { path, source }) => {
                assert_eq!(path, file);
                assert!(!source.to_string().is_empty());
            }
            _ => unreachable!(),
        }
        assert_eq!(fs::read_to_string(&file).unwrap(), "{Illegal data");

        let _ = fs::remove_file(&file);
        let _ = fs::remove_file(path.backup_path());
    }

    #[test] // 主文件损坏，备份完好，从备份恢复
    fn test_backup4_fn_readwrite() {
        let file = temp_path("test_backup4");
        let path = FilePath::new(Some(file.clone()));
        let _ = fs::remove_file(&file);
        let _ = fs::remove_file(path.backup_path());

        write_file(&file, "{Illegal data");
        write_file(
            &path.backup_path(),
            &serde_json::to_string_pretty(&vec![set_test_task()]).unwrap(),
        );

        update_tasks(&path, |t| {
            t.push(set_test_task());
            Ok(())
        })
        .unwrap();
        let load = load_tasks_read_only(&path).unwrap();
        assert_eq!(load.len(), 2);
        assert_eq!(load[0]._content(), "test_task1".to_string());
        assert_eq!(load[1]._content(), "test_task1".to_string());
        let backup: Vec<Task> =
            serde_json::from_str(&fs::read_to_string(path.backup_path()).unwrap()).unwrap();
        assert_eq!(backup, vec![set_test_task()]);

        let _ = fs::remove_file(&file);
        let _ = fs::remove_file(path.backup_path());
    }

    #[test] // 主文件确实，备份完好，从备份恢复
    fn test_backup5_fn_readwrite() {
        let file = temp_path("test_backup5");
        let path = FilePath::new(Some(file.clone()));
        let _ = fs::remove_file(&file);
        let _ = fs::remove_file(path.backup_path());

        write_file(
            &path.backup_path(),
            &serde_json::to_string_pretty(&vec![set_test_task()]).unwrap(),
        );

        update_tasks(&path, |t| {
            t.push(set_test_task());
            Ok(())
        })
        .unwrap();
        let load = load_tasks_read_only(&path).unwrap();
        assert_eq!(load.len(), 2);
        assert_eq!(load[0]._content(), "test_task1".to_string());
        assert_eq!(load[1]._content(), "test_task1".to_string());
        let backup: Vec<Task> =
            serde_json::from_str(&fs::read_to_string(path.backup_path()).unwrap()).unwrap();
        assert_eq!(backup, vec![set_test_task()]);

        let _ = fs::remove_file(&file);
        let _ = fs::remove_file(path.backup_path());
    }
}
