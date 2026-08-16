use crate::storage::{FilePath, load_tasks, save_tasks};
use crate::task::Task;
use crate::time::parse_deadline_input;
use std::io::{Error, ErrorKind};

pub fn add_task(
    content: String,
    path: &FilePath,
    deadline: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let parsed_deadline = match deadline {
        Some(s) => Some(parse_deadline_input(&s)?),
        None => None,
    };
    let mut tasks: Vec<Task> = load_tasks(path);
    let new_id: usize = tasks.iter().map(|t| t.id()).max().unwrap_or(0) + 1;
    let new_task: Task = Task::new(new_id, content, parsed_deadline);
    tasks.push(new_task);
    save_tasks(&tasks, path)
}

pub fn list_task(path: &FilePath) {
    let tasks: Vec<Task> = load_tasks(path);
    if tasks.is_empty() {
        println!("No tasks");
        return;
    }
    println!("status| id |         deadline         | task");
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

pub fn change_task(
    id: usize,
    path: &FilePath,
    content: Option<String>,
    deadline: Option<Option<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    if content.is_none() && deadline.is_none() {
        return Err(Box::new(Error::new(
            ErrorKind::InvalidInput,
            "Nothing to change",
        )));
    }
    let mut tasks = load_tasks(path);
    let mut find = false;
    for task in tasks.iter_mut() {
        if task.id() == id {
            if let Some(c) = content {
                task.set_content(c);
            }
            match deadline {
                Some(Some(s)) => {
                    task.set_deadline(Some(parse_deadline_input(&s)?));
                }
                Some(None) => task.set_deadline(None),
                None => {}
            }
            find = true;
            break;
        }
    }
    if !find {
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
    add_task(content1, &path, None).unwrap();
    list_task(&path);
    complete_task(1, &path).unwrap();
    list_task(&path);
    incomplete_task(1, &path).unwrap();
    list_task(&path);
    delete_task(1, &path).unwrap();
    list_task(&path);

    let content2: String = String::from("test_content2");
    let deadline1 = String::from("2000-01-01T12:00:00");
    add_task(content2, &path, None).unwrap();
    change_task(1, &path, None, Some(Some(deadline1))).unwrap();
    list_task(&path);

    let deadline2 = String::from("2000-01-01T20:00:00");
    change_task(1, &path, None, Some(Some(deadline2))).unwrap();
    list_task(&path);

    change_task(1, &path, Some(String::from("change_content")), None).unwrap();
    list_task(&path);

    change_task(1, &path, None, Some(None)).unwrap();
    list_task(&path);
}
