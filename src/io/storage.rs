use crate::task::Task;
use std::fs::File;
use std::io::{Error, ErrorKind, Read, Seek, SeekFrom, Write};
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
    f: impl FnOnce(&mut Vec<Task>) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = std::path::Path::new(&path.path()).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path.path())?;
    file.lock()?;
    let mut tasks = load_from(&file)?;
    f(&mut tasks)?;
    save_to(&file, &tasks)?;
    file.unlock()?;
    Ok(())
}

fn load_from(file: &File) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
    let mut f = file.try_clone()?;
    f.seek(SeekFrom::Start(0))?;
    let mut text = String::new();
    f.read_to_string(&mut text)?;
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    Ok(serde_json::from_str(&text).map_err(|e| {
        Error::new(
            ErrorKind::InvalidData,
            format!("The file contents are corrupted: {}", e),
        )
    })?)
}

fn save_to(file: &File, tasks: &[Task]) -> Result<(), Box<dyn std::error::Error>> {
    let mut f = file.try_clone()?;
    let data = serde_json::to_string_pretty(tasks)?;
    f.seek(SeekFrom::Start(0))?;
    f.set_len(0)?;
    f.write_all(data.as_bytes())?;
    f.flush()?;
    Ok(())
}

pub fn load_tasks(path: &FilePath) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
    let file = match File::options()
        .read(true)
        .write(false)
        .create(false)
        .truncate(false)
        .open(path.path())
    {
        Ok(f) => f,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    file.lock_shared()?;
    load_from(&file)
}
