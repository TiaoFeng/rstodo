use crate::task::Task;
use std::fs;

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
            FilePath::Default => String::from("task.json"),
        }
    }
}

pub fn load_tasks(path: &FilePath) -> Vec<Task> {
    let path = path.path();
    let content = fs::read_to_string(path);
    match content {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|_| Vec::new()),
        Err(_) => Vec::new(),
    }
}

pub fn save_tasks(tasks: &[Task], path: &FilePath) -> Result<(), Box<dyn std::error::Error>> {
    let path = path.path();
    let data = serde_json::to_string_pretty(tasks)?;
    fs::write(path, data)?;
    Ok(())
}

#[test]
fn test_save_and_load() {
    let task1: Task = Task::new(1001, String::from("test_content1"));
    let task2: Task = Task::new(1002, String::from("test_content2"));
    let tasks: Vec<Task> = vec![task1, task2];

    let test_path: FilePath = FilePath::new(Some(String::from("test_save_and_load.json")));
    let _ = fs::remove_file(test_path.path());
    save_tasks(&tasks, &test_path).unwrap();

    let loading_tasks: Vec<Task> = load_tasks(&test_path);
    assert_eq!(tasks, loading_tasks);
}
