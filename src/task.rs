//! Task类型模块
//!
//! 定义Task类型结构体与相关的方法
use std::fmt;

use chrono::{DateTime, Utc};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// 优先级枚举
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
    Low, // 优先级默认为Low
}

/// 为优先级实现`fmt::Display`的trait
impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Priority::High => write!(f, "High"),
            Priority::Medium => write!(f, "Medium"),
            Priority::Low => write!(f, "Low"),
        }
    }
}

/// Task结构体，定义了Task的数据类型与结构
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
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
    /// new方法，用于创建新Task实例
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

    /// 返回Task的id
    pub fn id(&self) -> usize {
        self.id
    }

    /// 返回Task的content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// 返回Task的description，可以为None
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// 返回Task是否完成
    pub fn is_complete(&self) -> bool {
        self.completed
    }

    /// 返回Task是否已经逾期
    pub fn is_overdue(&self, now: DateTime<Utc>) -> bool {
        if !self.completed {
            match self.deadline {
                Some(d) if d < now => {
                    return true;
                }
                _ => (),
            }
        }
        false
    }

    /// 返回Task的deadline，可以为None
    pub fn deadline(&self) -> Option<DateTime<Utc>> {
        self.deadline
    }

    /// 返回Task的priority
    pub fn priority(&self) -> Priority {
        self.priority
    }

    /// 用于将Task标记为完成
    pub fn complete(&mut self) {
        self.completed = true;
    }

    /// 用于将Task标记为未完成
    pub fn incomplete(&mut self) {
        self.completed = false;
    }

    /// 用于设置Task的content字段
    pub fn set_content(&mut self, content: String) {
        self.content = content;
    }

    /// 用于设置Task的description字段
    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
    }

    /// 用于设置Task的deadline字段
    pub fn set_deadline(&mut self, deadline: Option<DateTime<Utc>>) {
        self.deadline = deadline;
    }

    /// 用于设置Task的priority字段
    pub fn set_priority(&mut self, priority: Priority) {
        self.priority = priority;
    }
}

/// 单元测试
#[cfg(test)]
mod task_test {
    use std::ops::Add;

    use chrono::Days;

    use super::*;

    #[test]
    fn test_task_new() {
        let id: usize = 1001;
        let content: String = String::from("test_content1");
        let deadline1: DateTime<Utc> = "2000-01-01T12:00:00+00:00".parse().unwrap();
        let description = Some(String::from("test_description1"));
        let priority = Priority::default();
        let mut task: Task = Task::new(id, content, description, Some(deadline1), priority);

        assert_eq!(task.id(), 1001);
        assert_eq!(task.content(), "test_content1".to_string());
        assert_eq!(task.description(), Some("test_description1"));
        assert_eq!(task.priority(), Priority::Low);
        assert!(!task.is_complete());

        let expected = "2000-01-01T12:00:00+00:00".parse().unwrap();
        assert_eq!(task.deadline(), Some(expected));

        task.set_content("test_content2".to_string());
        assert_eq!(task.content(), "test_content2".to_string());
        task.set_description(Some("test_description2".to_string()));
        assert_eq!(task.description(), Some("test_description2"));
        task.set_description(None);
        assert!(task.description().is_none());
        task.complete();
        assert!(task.is_complete());
        task.incomplete();
        assert!(!task.is_complete());
        task.set_priority(Priority::High);
        assert_eq!(task.priority(), Priority::High);

        let deadline2: DateTime<Utc> = "2000-01-02T12:00:00+00:00".parse().unwrap();
        task.set_deadline(Some(deadline2));
        let expected = "2000-01-02T12:00:00+00:00".parse().unwrap();
        assert_eq!(task.deadline(), Some(expected));
        task.set_deadline(None);
        assert!(task.deadline().is_none());
    }

    #[test]
    fn test_task_is_overdue() {
        let id: usize = 1001;
        let content: String = String::from("test_content1");
        let deadline1: DateTime<Utc> = "2000-01-01T12:00:00+00:00".parse().unwrap();
        let description = Some(String::from("test_description1"));
        let priority = Priority::default();
        let mut task: Task = Task::new(id, content, description, Some(deadline1), priority);
        assert!(task.is_overdue(Utc::now()));

        task.complete();
        assert!(!task.is_overdue(Utc::now()));

        task.incomplete();
        assert!(task.is_overdue(Utc::now()));
        task.set_deadline(None);
        assert!(!task.is_overdue(Utc::now()));

        let deadline2: DateTime<Utc> = Utc::now().add(Days::new(1));
        task.set_deadline(Some(deadline2));
        assert!(!task.is_overdue(Utc::now()));
    }
}
