use crate::error::{invalid_input, not_found};
use crate::storage::{FilePath, load_tasks, update_tasks};
use crate::task::{Task, TaskRow};
use crate::time::parse_deadline_input;

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
    println!("status| no |         deadline         | task");
    for (i, task) in tasks.iter().enumerate() {
        print!("{}", TaskRow { task, no: i + 1 });
        match task.description() {
            None => println!(),
            Some(_) => println!("    --Show desc"),
        }
    }
    Ok(())
}

pub fn show_details(no: usize, path: &FilePath) -> Result<(), Box<dyn std::error::Error>> {
    let tasks = load_tasks(path)?;
    if tasks.is_empty() {
        println!("No tasks");
        return Ok(());
    }
    let task = tasks
        .get(no.checked_sub(1).ok_or_else(not_found)?)
        .ok_or_else(not_found)?;
    println!("status| no |         deadline         | task");
    println!("{}", TaskRow { task, no });
    println!("-Description-");
    if task.description().is_none() {
        println!("No description");
    } else {
        println!("{}", task.description().unwrap());
    }
    Ok(())
}

pub fn complete_task(no: usize, path: &FilePath) -> Result<(), Box<dyn std::error::Error>> {
    update_tasks(path, |tasks| {
        let idx = no.checked_sub(1).ok_or_else(not_found)?;
        let task = tasks.get_mut(idx).ok_or_else(not_found)?;
        task.complete();
        Ok(())
    })
}

pub fn incomplete_task(no: usize, path: &FilePath) -> Result<(), Box<dyn std::error::Error>> {
    update_tasks(path, |tasks| {
        let idx = no.checked_sub(1).ok_or_else(not_found)?;
        let task = tasks.get_mut(idx).ok_or_else(not_found)?;
        task.incomplete();
        Ok(())
    })
}

pub fn delete_task(no: usize, path: &FilePath) -> Result<(), Box<dyn std::error::Error>> {
    update_tasks(path, |tasks| {
        let idx = no.checked_sub(1).ok_or_else(not_found)?;
        if idx >= tasks.len() {
            return Err(not_found());
        }
        tasks.remove(idx);
        Ok(())
    })
}

pub fn change_task(
    no: usize,
    path: &FilePath,
    content: Option<String>,
    description: Option<Option<String>>,
    deadline: Option<Option<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    if content.is_none() && deadline.is_none() && description.is_none() {
        return Err(invalid_input());
    }
    update_tasks(path, |tasks| {
        let idx = no.checked_sub(1).ok_or_else(not_found)?;
        let task = tasks.get_mut(idx).ok_or_else(not_found)?;
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
        Ok(())
    })
}

#[cfg(test)]
mod commands_test {
    use crate::{
        commands::{add_task, change_task, delete_task, list_task, show_details},
        storage::*,
    };
    use std::fs;
    #[test]
    fn test_add_list_show() {
        let path: FilePath = FilePath::new(Some(String::from("test1_commands.json")));
        let _ = fs::remove_file(path.path());
        let content1 = String::from("test_content1");
        let deadline1 = String::from("2000-01-01T12:00:00");
        let description1 = Some(String::from("test_description1"));
        add_task(content1, &path, description1, Some(deadline1)).unwrap();

        let content2 = String::from("test_content2");
        let deadline2 = String::from("2000-01-01T18:00:00");
        let description2 = Some(String::from("test_description2"));
        add_task(content2, &path, description2, Some(deadline2)).unwrap();

        let content3 = String::from("test_content3");
        let deadline3 = None;
        let description3 = None;
        add_task(content3, &path, description3, deadline3).unwrap();

        println!("----add打印测试----");
        list_task(&path).unwrap();
        println!("----show打印细节测试----");
        show_details(2, &path).unwrap();
    }

    #[test]
    fn test_delete() {
        let path: FilePath = FilePath::new(Some(String::from("test2_commands.json")));
        let _ = fs::remove_file(path.path());
        let content1 = String::from("test_content1");
        let deadline1 = String::from("2000-01-01T12:00:00");
        let description1 = Some(String::from("test_description1"));
        add_task(content1, &path, description1, Some(deadline1)).unwrap();

        let content2 = String::from("test_content2");
        let deadline2 = String::from("2000-01-01T18:00:00");
        let description2 = Some(String::from("test_description2"));
        add_task(content2, &path, description2, Some(deadline2)).unwrap();

        let content3 = String::from("test_content3");
        let deadline3 = None;
        let description3 = None;
        add_task(content3, &path, description3, deadline3).unwrap();
        println!("----delete打印测试----");
        list_task(&path).unwrap();
        delete_task(2, &path).unwrap();
        list_task(&path).unwrap();
    }

    #[test]
    fn test_change() {
        let path: FilePath = FilePath::new(Some(String::from("test3_commands.json")));
        let _ = fs::remove_file(path.path());
        let content1 = String::from("test_content1");
        let deadline1 = String::from("2000-01-01T12:00:00");
        let description1 = Some(String::from("test_description1"));
        add_task(content1, &path, description1, Some(deadline1)).unwrap();

        let content2 = String::from("test_content2");
        let deadline2 = String::from("2000-01-01T18:00:00");
        let description2 = Some(String::from("test_description2"));
        add_task(content2, &path, description2, Some(deadline2)).unwrap();

        let content3 = String::from("test_content3");
        let deadline3 = None;
        let description3 = None;
        add_task(content3, &path, description3, deadline3).unwrap();

        println!("----删除1description----");
        list_task(&path).unwrap();
        change_task(1, &path, None, Some(None), None).unwrap();
        list_task(&path).unwrap();
        println!("----修改1content,description,删除1deadline----");
        list_task(&path).unwrap();
        change_task(
            1,
            &path,
            Some(String::from("change_test")),
            Some(Some(String::from("change_test_desc"))),
            Some(None),
        )
        .unwrap();
        list_task(&path).unwrap();
        println!("----修改1deadline----");
        list_task(&path).unwrap();
        change_task(1, &path, None, None, Some(Some("2000-2-1".to_string()))).unwrap();
        list_task(&path).unwrap();
    }
}
