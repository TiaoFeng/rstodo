use crate::task::Task;
use std::fs;

const FILE_PATH: &str = "task.json";

pub fn load_tasks() -> Vec<Task> {
    let content = fs::read_to_string(FILE_PATH);
    match content {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|_| Vec::new()),
        Err(_) => Vec::new(),
    }
}

pub fn save_tasks(tasks: &[Task]) -> Result<(), Box<dyn std::error::Error>> {
    let data = serde_json::to_string_pretty(tasks)?;
    fs::write(FILE_PATH, data)?;
    Ok(())
}

#[test]
fn test_save_and_load() {
    let task1: Task = Task::new(1001, String::from("test_content1"));
    let task2: Task = Task::new(1002, String::from("test_content2"));
    let tasks: Vec<Task> = vec![task1, task2];

    save_tasks(&tasks).unwrap();

    let loading_tasks: Vec<Task> = load_tasks();
    assert_eq!(tasks, loading_tasks);
}
