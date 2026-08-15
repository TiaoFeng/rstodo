use crate::storage::{FilePath, load_tasks, save_tasks};
use crate::task::Task;
use std::io::{Error, ErrorKind};

pub fn add_task(content: String, path: &FilePath) -> Result<(), Box<dyn std::error::Error>> {
    let mut tasks: Vec<Task> = load_tasks(path);
    let new_id: usize = tasks.iter().map(|t| t.id()).max().unwrap_or(0) + 1;
    let new_task: Task = Task::new(new_id, content);
    tasks.push(new_task);
    save_tasks(&tasks, path)
}

pub fn list_task(path: &FilePath) {
    let tasks: Vec<Task> = load_tasks(path);
    if tasks.is_empty() {
        println!("No tasks");
        return;
    }
    println!("status| id | task");
    for task in tasks {
        task.print();
    }
}

pub fn complete_task(id: usize, path: &FilePath) -> Result<(), Box<dyn std::error::Error>> {
    let mut tasks: Vec<Task> = load_tasks(path);
    let mut find: bool = false;
    for task in tasks.iter_mut() {
        if task.id() == id {
            task.complete();
            find = true;
            break;
        }
    }
    if !find {
        return Err(Box::new(Error::new(ErrorKind::NotFound, "Not Found")));
    }
    save_tasks(&tasks, path)
}

pub fn incomplete_task(id: usize, path: &FilePath) -> Result<(), Box<dyn std::error::Error>> {
    let mut tasks: Vec<Task> = load_tasks(path);
    let mut find: bool = false;
    for task in tasks.iter_mut() {
        if task.id() == id {
            task.incomplete();
            find = true;
            break;
        }
    }
    if !find {
        return Err(Box::new(Error::new(ErrorKind::NotFound, "Not Found")));
    }
    save_tasks(&tasks, path)
}

pub fn delete_task(id: usize, path: &FilePath) -> Result<(), Box<dyn std::error::Error>> {
    let mut tasks: Vec<Task> = load_tasks(path);
    let orgin_len = tasks.len();
    tasks.retain(|task: &Task| task.id() != id);
    if tasks.len() == orgin_len {
        return Err(Box::new(Error::new(ErrorKind::NotFound, "Not Found")));
    }
    save_tasks(&tasks, path)
}

#[test]
fn test_commands() {
    use std::fs;
    let path: FilePath = FilePath::new(Some(String::from("test_commands.json")));
    let _ = fs::remove_file(path.path());
    let content1 = String::from("test_content1");
    list_task(&path);
    add_task(content1, &path).unwrap();
    list_task(&path);
    complete_task(1, &path).unwrap();
    list_task(&path);
    incomplete_task(1, &path).unwrap();
    list_task(&path);
    delete_task(1, &path).unwrap();
    list_task(&path);
}
