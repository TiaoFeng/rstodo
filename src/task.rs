use crate::time::to_local_time;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Task {
    id: usize,
    content: String,
    completed: bool,
    #[serde(default)]
    deadline: Option<DateTime<Utc>>,
}

impl Task {
    pub fn new(id: usize, content: String, deadline: Option<DateTime<Utc>>) -> Task {
        Task {
            id: id,
            content: content,
            completed: false,
            deadline: match deadline {
                Some(t) => Some(t),
                None => None,
            },
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

    pub fn _deadline(&self) -> Option<DateTime<Utc>> {
        self.deadline
    }

    pub fn complete(&mut self) {
        self.completed = true;
    }

    pub fn incomplete(&mut self) {
        self.completed = false;
    }

    pub fn set_content(&mut self, content: String) {
        self.content = content;
    }

    pub fn set_deadline(&mut self, deadline: Option<DateTime<Utc>>) {
        self.deadline = deadline;
    }

    pub fn print(&self) {
        let status: &str = if self.completed { "✓" } else { " " };
        let deadline: &str = match self.deadline {
            None => "       No          ",
            Some(t) => &to_local_time(&t).unwrap().to_string(),
        };
        println!(
            "  [{}]   {:<3}    {}      {}",
            status, self.id, deadline, self.content
        );
    }
}

#[test]
fn test_task() {
    let id: usize = 1001;
    let content: String = String::from("test_content");
    let deadline: DateTime<Utc> = "2000-01-01T12:00:00+00:00".parse().unwrap();
    let mut task: Task = Task::new(id, content.clone(), None);
    task.set_deadline(Some(deadline));

    assert_eq!(task.id(), id);
    assert_eq!(task._content(), content);
    assert_eq!(task._completed(), false);

    task.complete();
    assert_eq!(task._completed(), true);

    task.incomplete();
    assert_eq!(task._completed(), false);

    // let deadline_wrong: DateTime<Utc> = "2000-01-10T12:00:00+00:00".parse().unwrap();
    assert_eq!(task._deadline(), Some(deadline));
}
