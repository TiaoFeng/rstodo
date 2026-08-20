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

    pub fn deadline(&self) -> Option<DateTime<Utc>> {
        self.deadline
    }

    pub fn priority(&self) -> Priority {
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
            Some(t) => to_local_time(&t).to_string(),
        };
        let more = if task.description.is_some() {
            String::from("Show desc")
        } else {
            String::new()
        };
        vec![
            status.to_string(),
            self.no.to_string(),
            task.priority.to_string(),
            deadline,
            task.content.clone(),
            more,
        ]
    }
}

#[cfg(test)]
mod task_test {
    use super::*;

    #[test]
    fn test_task() {
        let id: usize = 1001;
        let content: String = String::from("test_content1");
        let deadline1: DateTime<Utc> = "2000-01-01T12:00:00+00:00".parse().unwrap();
        let description = Some(String::from("test_description1"));
        let priority = Priority::default();
        let mut task: Task = Task::new(id, content, description, Some(deadline1), priority);

        assert_eq!(task.id(), 1001);
        assert_eq!(task._content(), "test_content1".to_string());
        assert_eq!(task.description(), Some("test_description1".to_string()));
        assert_eq!(task.priority(), Priority::Low);
        assert!(!task._completed());

        let expected = "2000-01-01T12:00:00+00:00".parse().unwrap();
        assert_eq!(task.deadline(), Some(expected));

        task.set_content("test_content2".to_string());
        assert_eq!(task._content(), "test_content2".to_string());
        task.set_description(Some("test_description2".to_string()));
        assert_eq!(task.description(), Some("test_description2".to_string()));
        task.set_description(None);
        assert!(task.description().is_none());
        task.complete();
        assert!(task._completed());
        task.incomplete();
        assert!(!task._completed());
        task.set_priority(Priority::High);
        assert_eq!(task.priority(), Priority::High);

        let deadline2: DateTime<Utc> = "2000-01-02T12:00:00+00:00".parse().unwrap();
        task.set_deadline(Some(deadline2));
        let expected = "2000-01-02T12:00:00+00:00".parse().unwrap();
        assert_eq!(task.deadline(), Some(expected));
        task.set_deadline(None);
        assert!(task.deadline().is_none());
    }
}
