use crate::storage::{FilePath, load_tasks, update_tasks};
use crate::task::Task;
use crate::time::parse_deadline_input;
use std::io::{Error, ErrorKind};

pub fn add_task(
    content: String,
    path: &FilePath,
    description: Option<String>,
    deadline: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let parsed_deadline = match deadline {
        Some(s) => Some(parse_deadline_input(&s)?),
        None => None,
    };
    update_tasks(path, |tasks| {
        let new_id: usize = tasks.iter().map(|t| t.id()).max().unwrap_or(0) + 1;
        let new_task: Task = Task::new(new_id, content, description, parsed_deadline);
        tasks.push(new_task);
        Ok(())
    })
}

pub fn list_task(path: &FilePath) -> Result<(), Box<dyn std::error::Error>> {
    let tasks: Vec<Task> = load_tasks(path)?;
    if tasks.is_empty() {
        println!("No tasks");
        return Ok(());
    }
    println!("status| id |         deadline         | task");
    for task in tasks {
        print!("{}", task);
        match task.description() {
            None => println!(),
            Some(_) => println!("    --Show desc"),
        }
    }
    Ok(())
}

pub fn show_details(id: usize, path: &FilePath) -> Result<(), Box<dyn std::error::Error>> {
    let tasks = load_tasks(path)?;
    if tasks.is_empty() {
        println!("No tasks");
        return Ok(());
    }
    let mut find = false;
    for task in tasks {
        if task.id() == id {
            find = true;
            println!("status| id |         deadline         | task");
            println!("{}", task);
            println!("description:");
            if task.description().is_none() {
                println!("No description");
                break;
            } else {
                println!("{}", task.description().unwrap());
                break;
            }
        }
    }
    if !find {
        return Err(Box::new(Error::new(ErrorKind::NotFound, "Not Found")));
    }
    Ok(())
}

pub fn complete_task(id: usize, path: &FilePath) -> Result<(), Box<dyn std::error::Error>> {
    update_tasks(path, |tasks| {
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
        Ok(())
    })
}

pub fn incomplete_task(id: usize, path: &FilePath) -> Result<(), Box<dyn std::error::Error>> {
    update_tasks(path, |tasks| {
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
        Ok(())
    })
}

pub fn delete_task(id: usize, path: &FilePath) -> Result<(), Box<dyn std::error::Error>> {
    update_tasks(path, |tasks| {
        let orgin_len = tasks.len();
        tasks.retain(|task: &Task| task.id() != id);
        if tasks.len() == orgin_len {
            return Err(Box::new(Error::new(ErrorKind::NotFound, "Not Found")));
        }
        Ok(())
    })
}

pub fn change_task(
    id: usize,
    path: &FilePath,
    content: Option<String>,
    description: Option<Option<String>>,
    deadline: Option<Option<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    if content.is_none() && deadline.is_none() && description.is_none() {
        return Err(Box::new(Error::new(
            ErrorKind::InvalidInput,
            "Nothing to change",
        )));
    }
    update_tasks(path, |tasks| {
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
                match description {
                    Some(Some(s)) => {
                        task.set_description(Some(s));
                    }
                    Some(None) => task.set_description(None),
                    None => {}
                }
                find = true;
                break;
            }
        }
        if !find {
            return Err(Box::new(Error::new(ErrorKind::NotFound, "Not Found")));
        }
        Ok(())
    })
}

#[test]
fn test_commands() {
    use std::fs;
    let path: FilePath = FilePath::new(Some(String::from("test_commands.json")));
    let _ = fs::remove_file(path.path());
    let content1 = String::from("test_content1");
    list_task(&path).unwrap();
    add_task(content1, &path, None, None).unwrap();
    list_task(&path).unwrap();
    complete_task(1, &path).unwrap();
    list_task(&path).unwrap();
    incomplete_task(1, &path).unwrap();
    list_task(&path).unwrap();
    delete_task(1, &path).unwrap();
    list_task(&path).unwrap();

    let content2: String = String::from("test_content2");
    let deadline1 = String::from("2000-01-01T12:00:00");
    add_task(content2, &path, None, None).unwrap();
    change_task(1, &path, None, None, Some(Some(deadline1))).unwrap();
    list_task(&path).unwrap();

    let deadline2 = String::from("2000-01-01T20:00:00");
    change_task(1, &path, None, None, Some(Some(deadline2))).unwrap();
    list_task(&path).unwrap();

    change_task(1, &path, Some(String::from("change_content")), None, None).unwrap();
    list_task(&path).unwrap();

    change_task(1, &path, None, None, Some(None)).unwrap();
    list_task(&path).unwrap();

    let description = Some(Some(String::from("test_description")));
    change_task(1, &path, None, description, Some(None)).unwrap();
    list_task(&path).unwrap();
    show_details(1, &path).unwrap();
    change_task(1, &path, None, Some(None), Some(None)).unwrap();
    list_task(&path).unwrap();
    show_details(1, &path).unwrap();
}
