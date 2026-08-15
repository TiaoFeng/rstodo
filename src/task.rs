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

    pub fn _content(&self) -> &str {
        &self.content
    }

    pub fn _completed(&self) -> bool {
        self.completed
    }

    pub fn complete(&mut self) {
        self.completed = true;
    }

    pub fn incomplete(&mut self) {
        self.completed = false;
    }

    pub fn print(&self) {
        let status: &str = if self.completed { "✓" } else { " " };
        println!("  [{}]   {:<3}  {}", status, self.id, self.content);
    }
}

#[test]
fn test_task() {
    let id = 1001;
    let content = String::from("test_content");
    let mut task = Task::new(id, content.clone());
    assert_eq!(task.id(), id);
    assert_eq!(task._content(), content);
    assert_eq!(task._completed(), false);

    task.complete();
    assert_eq!(task._completed(), true);

    task.incomplete();
    assert_eq!(task._completed(), false);
}
