use std::fmt;

use crate::time::to_local_time;
use chrono::{DateTime, Utc};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, ValueEnum, Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
)]
pub enum Priority {
    #[value(alias = "1")]
    High,
    #[value(alias = "2")]
    Medium,
    #[value(alias = "3")]
    #[default]
    Low,
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Priority::High => write!(f, "High"),
            Priority::Medium => write!(f, "Medium"),
            Priority::Low => write!(f, "Low"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Task {
    id: usize,
    content: String,
    #[serde(default)]
    description: Option<String>,
    completed: bool,
    #[serde(default)]
    deadline: Option<DateTime<Utc>>,
    #[serde(default)]
    priority: Priority,
}

impl Task {
    pub fn new(
        id: usize,
        content: String,
        description: Option<String>,
        deadline: Option<DateTime<Utc>>,
        priority: Priority,
    ) -> Task {
        Task {
            id,
            content,
            description,
            completed: false,
            deadline,
            priority,
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn _content(&self) -> &str {
        &self.content
    }

    pub fn description(&self) -> Option<String> {
        self.description.clone()
    }

    pub fn _completed(&self) -> bool {
        self.completed
    }

    pub fn _deadline(&self) -> Option<DateTime<Utc>> {
        self.deadline
    }

    pub fn _priority(&self) -> Priority {
        self.priority
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

    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
    }

    pub fn set_deadline(&mut self, deadline: Option<DateTime<Utc>>) {
        self.deadline = deadline;
    }

    pub fn set_priority(&mut self, priority: Priority) {
        self.priority = priority;
    }
}

pub struct TaskRow<'a> {
    pub task: &'a Task,
    pub no: usize,
}

impl TaskRow<'_> {
    pub fn to_table(&self) -> Vec<String> {
        let task = self.task;
        let status: &str = if task.completed { "✓" } else { " " };
        let deadline = match task.deadline {
            None => String::from("No"),
            Some(t) => to_local_time(&t).unwrap().to_string(),
        };
        let description = task.description.as_ref().map(|_| String::from("Show desc"));
        if let Some(more) = description {
            return vec![
                status.to_string(),
                self.no.to_string(),
                task.priority.to_string(),
                deadline,
                task.content.clone(),
                more,
            ];
        }
        vec![
            status.to_string(),
            self.no.to_string(),
            task.priority.to_string(),
            deadline,
            task.content.clone(),
        ]
    }
}

#[test]
fn test_task() {
    let id: usize = 1001;
    let content: String = String::from("test_content");
    let deadline: DateTime<Utc> = "2000-01-01T12:00:00+00:00".parse().unwrap();
    let description = Some(String::from("test_description"));
    let priority = Priority::High;
    let mut task: Task = Task::new(id, content.clone(), None, None, priority);
    task.set_deadline(Some(deadline));
    task.set_description(description.clone());

    assert_eq!(task.id(), id);
    assert_eq!(task._content(), content);
    assert!(!task._completed());
    task.complete();
    assert!(task._completed());
    task.incomplete();
    assert!(!task._completed());
    // let deadline_wrong: DateTime<Utc> = "2000-01-10T12:00:00+00:00".parse().unwrap();
    assert_eq!(task._deadline(), Some(deadline));
    assert_eq!(task.description(), description);
}
