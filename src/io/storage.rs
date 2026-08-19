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
    file.lock()
        .map_err(|err| io_err("lock", path.path(), err))?;
    let mut tasks = load_from(&file, &path.path())?;
    f(&mut tasks)?;
    save_to(&file, &tasks, &path.path())?;
    file.unlock()
        .map_err(|err| io_err("unlock", path.path(), err))?;
    Ok(())
}

fn load_from(file: &File, path: &str) -> Result<Vec<Task>, AppError> {
    let mut f = file
        .try_clone()
        .map_err(|err| io_err("clone file for reading", path.to_string(), err))?;
    f.seek(SeekFrom::Start(0))
        .map_err(|err| io_err("seek to the beginning", path.to_string(), err))?;
    let mut text = String::new();
    f.read_to_string(&mut text)
        .map_err(|err| io_err("read tasks", path.to_string(), err))?;
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }

    serde_json::from_str(&text).map_err(|serde_json_err| AppError::Corrupted {
        path: path.to_string(),
        source: serde_json_err,
    })
}

fn save_to(file: &File, tasks: &[Task], path: &str) -> Result<(), AppError> {
    let mut f = file
        .try_clone()
        .map_err(|err| io_err("clone file for saving", path.to_string(), err))?;
    let data =
        serde_json::to_string_pretty(tasks).map_err(|serde_json_err| AppError::Corrupted {
            path: path.to_string(),
            source: serde_json_err,
        })?;
    f.seek(SeekFrom::Start(0))
        .map_err(|err| io_err("seek to the beginning", path.to_string(), err))?;
    f.set_len(0)
        .map_err(|err| io_err("reset text for saving", path.to_string(), err))?;
    f.write_all(data.as_bytes())
        .map_err(|err| io_err("write to file", path.to_string(), err))?;
    f.flush()
        .map_err(|err| io_err("flush", path.to_string(), err))?;
    Ok(())
}

pub fn load_tasks(path: &FilePath) -> Result<Vec<Task>, AppError> {
    let file = match File::options()
        .read(true)
        .write(false)
        .create(false)
        .truncate(false)
        .open(path.path())
    {
        Ok(f) => f,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(io_err("create a read-only handle", path.path(), err)),
    };
    file.lock_shared()
        .map_err(|err| io_err("try to get a shared lock", path.path(), err))?;
    load_from(&file, &path.path())
}
