use crate::storage::{load_tasks, save_tasks};
use crate::task::Task;
use std::io::{Error, ErrorKind};

pub fn add_task(content: String) -> Result<(), Box<dyn std::error::Error>> {
    let mut tasks: Vec<Task> = load_tasks();
    let new_id: usize = tasks.iter().map(|t| t.id()).max().unwrap_or(0) + 1;
    let new_task: Task = Task::new(new_id, content);
    tasks.push(new_task);
    save_tasks(&tasks)
}

pub fn list_task() {
    let tasks: Vec<Task> = load_tasks();
    if tasks.is_empty() {
        println!("No tasks");
        return;
    }
    println!("Task List");
    for task in tasks {
        task.print();
    }
}

pub fn complete_task(id: usize) -> Result<(), Box<dyn std::error::Error>> {
    let mut tasks: Vec<Task> = load_tasks();
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
    save_tasks(&tasks)
}

pub fn delete_task(id: usize) -> Result<(), Box<dyn std::error::Error>> {
    let mut tasks: Vec<Task> = load_tasks();
    let orgin_len = tasks.len();
    tasks.retain(|task: &Task| task.id() != id);
    if tasks.len() == orgin_len {
        return Err(Box::new(Error::new(ErrorKind::NotFound, "Not Found")));
    }
    save_tasks(&tasks)
}

#[test]
fn test_add() {
    let content = String::from("test_content3");
    add_task(content).expect("add_task failed");
}

#[test]
fn test_list() {
    list_task();
}

#[test]
fn test_complete() {
    let content = String::from("test_content1");
    add_task(content).expect("add_task failed");
    complete_task(1).unwrap();
    list_task();
}

#[test]
fn test_delete() {
    // let content = String::from("test_content1");
    // add_task(content).expect("add_task failed");
    delete_task(1).unwrap();
    list_task();
}
