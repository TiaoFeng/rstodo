//! Task类型模块
//!
//! 定义Task类型结构体与相关的方法
use std::fmt;

use crate::time::to_local_time;
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

    /// 返回Task的content,暂时还未使用
    pub fn _content(&self) -> &str {
        &self.content
    }

    /// 返回Task的description，可以为None
    pub fn description(&self) -> Option<String> {
        self.description.clone()
    }

    /// 返回Task是否完成，暂时还未使用
    pub fn _completed(&self) -> bool {
        self.completed
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

/// 定义了TaskRow结构体，标记Task的序号，用于输出
///
/// 为何不对Task定义Display trait，主要考虑到输出序号的完整性，
/// 使用id输出，在用户删改使用后，序号不连续，比较丑陋
pub struct TaskRow<'a> {
    pub task: &'a Task, // 需要保证TaskRow的生命周期与Task相同
    pub no: usize,
}

impl TaskRow<'_> {
    /// 输出符合cli_print.rs中转换为表格所需要的数据格式
    ///
    /// 逻辑：
    /// 1. 使用✓符号标记是否完成
    /// 2. 标记是否有deadline，description
    /// 3. 整理需要打印的列表，转换为`Vec<String>`供排版打印
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

/// 单元测试
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
