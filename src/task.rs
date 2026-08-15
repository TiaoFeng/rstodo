use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Task {
    id: usize,
    content: String,
    completed: bool,
}

impl Task {
    pub fn new(id: usize, content: String) -> Task {
        Task {
            id: id,
            content: content,
            completed: false,
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn completed(&self) -> bool {
        self.completed
    }

    pub fn complete(&mut self) {
        self.completed = true;
    }

    pub fn print(&self) {
        if self.completed {
            println!("Task{}, {}, Done", self.id, self.content);
        } else {
            println!("Task{}, {}, Incomplete", self.id, self.content);
        }
    }
}

#[test]
fn test_task() {
    let id = 1001;
    let content = String::from("test_content");
    let mut task = Task::new(id, content.clone());
    assert_eq!(task.id(), id);
    assert_eq!(task.content(), content);
    assert_eq!(task.completed(), false);

    task.complete();
    assert_eq!(task.completed(), true);
}
