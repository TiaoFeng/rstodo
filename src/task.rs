//! Task类型模块
//!
//! 定义Task类型结构体与相关的方法

use chrono::{DateTime, Utc};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;

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
        !self.completed && self.deadline.is_some_and(|d| d < now)
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
