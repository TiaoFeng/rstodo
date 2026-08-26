//! 业务核心模块
//!
//! 实现项目所需的add, change, list, show, complete,
//! incomplete, delete, undo等接口
use std::cmp::Ordering;

use chrono::{DateTime, Utc};
use clap::ValueEnum;

use crate::error::AppError;
use crate::io::storage::TaskStore;
use crate::task::{Priority, Task};

/// 定义了用户可选择的两种排序方式，按照时间或优先级
#[derive(ValueEnum, Clone)]
pub enum SortBy {
    #[value(alias = "d")]
    Deadline,
    #[value(alias = "p")]
    Priority,
}

/// 把change所需参数打包进入结构体
pub struct TaskUpdate {
    content: Option<String>,
    description: Option<Option<String>>,
    deadline: Option<Option<DateTime<Utc>>>, //传入的数值应当是处理过的
    priority: Option<Option<Priority>>,
}

impl TaskUpdate {
    pub fn new(
        content: Option<String>,
        description: Option<Option<String>>,
        deadline: Option<Option<DateTime<Utc>>>,
        priority: Option<Option<Priority>>,
    ) -> Self {
        TaskUpdate {
            content,
            description,
            deadline,
            priority,
        }
    }

    /// 判断传入的change参数是否全为空，用于特殊判定`AppError::NothingToChange`
    fn is_empty(&self) -> bool {
        self.content.is_none()
            && self.description.is_none()
            && self.deadline.is_none()
            && self.priority.is_none()
    }

    /// 公开的change业务接口
    pub fn change_task(self, no: usize, store: &TaskStore) -> Result<(), AppError> {
        if self.is_empty() {
            return Err(AppError::NothingToChange);
        }
        store.update_with_backup(|tasks| {
            if no == 0 || no > tasks.len() {
                return Err(AppError::TaskNotFound { no });
            }
            let task = tasks.get_mut(no - 1).ok_or(AppError::TaskNotFound { no })?;

            if let Some(c) = self.content {
                task.set_content(c);
            }
            match self.deadline {
                Some(Some(d)) => {
                    task.set_deadline(Some(d));
                }
                Some(None) => task.set_deadline(None),
                None => {}
            }
            match self.description {
                Some(Some(s)) => {
                    task.set_description(Some(s));
                }
                Some(None) => task.set_description(None),
                None => {}
            }
            match self.priority {
                Some(Some(p)) => task.set_priority(p),
                Some(None) => task.set_priority(Priority::default()),
                None => {}
            }
            Ok(())
        })
    }
}

pub fn add_task(
    store: &TaskStore,
    content: String,
    description: Option<String>,
    deadline: Option<DateTime<Utc>>,
    priority: Option<Priority>,
) -> Result<(), AppError> {
    store.update_with_backup(|tasks: &mut Vec<Task>| {
        let new_id: usize = tasks.iter().map(|t: &Task| t.id()).max().unwrap_or(0) + 1;
        let new_task: Task = Task::new(
            new_id,
            content,
            description,
            deadline,
            priority.unwrap_or_default(),
        );
        tasks.push(new_task);
        Ok(())
    })
}

pub fn list_tasks(store: &TaskStore, sort: Option<SortBy>) -> Result<Option<Vec<Task>>, AppError> {
    match sort {
        None => {
            let tasks: Vec<Task> = store.load()?;
            if tasks.is_empty() {
                return Ok(None);
            }
            Ok(Some(tasks))
        }
        Some(order) => {
            store.update_without_backup(|tasks: &mut Vec<Task>| {
                sort_tasks(tasks, order);
                Ok(())
            })?;
            let tasks = store.load()?;
            if tasks.is_empty() {
                return Ok(None);
            }
            Ok(Some(tasks))
        }
    }
}

fn sort_tasks(tasks: &mut [Task], order: SortBy) {
    match order {
        SortBy::Deadline => {
            tasks.sort_by(|a: &Task, b: &Task| match (a.deadline(), b.deadline()) {
                (Some(d1), Some(d2)) => d1.cmp(&d2),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            })
        }
        SortBy::Priority => tasks.sort_by_key(|t: &Task| t.priority()),
    }
}

pub fn show_details(no: usize, store: &TaskStore) -> Result<Option<Task>, AppError> {
    let tasks: Vec<Task> = store.load()?;
    if tasks.is_empty() {
        Ok(None)
    } else if no == 0 || no > tasks.len() {
        Err(AppError::TaskNotFound { no })
    } else {
        let task: Task = tasks
            .get(no - 1)
            .cloned()
            .ok_or(AppError::TaskNotFound { no })?;
        Ok(Some(task))
    }
}

pub fn complete_task(no: usize, store: &TaskStore) -> Result<(), AppError> {
    store.update_with_backup(|tasks| {
        if no == 0 || no > tasks.len() {
            Err(AppError::TaskNotFound { no })
        } else {
            let task = tasks.get_mut(no - 1).ok_or(AppError::TaskNotFound { no })?;
            task.complete();
            Ok(())
        }
    })
}

pub fn incomplete_task(no: usize, store: &TaskStore) -> Result<(), AppError> {
    store.update_with_backup(|tasks| {
        if no == 0 || no > tasks.len() {
            Err(AppError::TaskNotFound { no })
        } else {
            let task = tasks.get_mut(no - 1).ok_or(AppError::TaskNotFound { no })?;
            task.incomplete();
            Ok(())
        }
    })
}

pub fn delete_task(no: usize, store: &TaskStore) -> Result<(), AppError> {
    store.update_with_backup(|tasks| {
        if no == 0 || no > tasks.len() {
            return Err(AppError::TaskNotFound { no });
        }
        tasks.remove(no - 1);
        Ok(())
    })
}

/// 返回备份文件中的任务列表，用于预览恢复的内容，当备份的文件与主文件一致，返回`AppError::NothingToUndo`
pub fn undo_task_preview(store: &TaskStore) -> Result<Vec<Task>, AppError> {
    let backup_tasks = store.load_backup()?;

    let current = store.load()?;
    if backup_tasks == current {
        return Err(AppError::NothingToUndo);
    }
    Ok(backup_tasks)
}

/// 执行恢复任务，将备份写入主文件
pub fn undo_task_apply(store: &TaskStore, snapshot: &[Task]) -> Result<(), AppError> {
    store.restore_backup(snapshot)
}

/// 任务状态结构体，用于显示任务状态清单
pub struct TaskStatus {
    total: usize,
    done: usize,
    undone: usize,
}

impl TaskStatus {
    /// 收集传入TaskStore的任务状态
    ///
    /// 包括：
    /// - total：总计任务数量
    /// - done： 完成的任务数量
    /// - undone： 未完成的任务数量
    pub fn collect(store: &TaskStore) -> Result<Self, AppError> {
        let tasks = store.load()?;
        let total = tasks.len();
        let done = tasks.iter().filter(|task| task.completed()).count();
        Ok(TaskStatus {
            total,
            done,
            undone: total - done,
        })
    }

    /// 输出符合cli_print.rs中转换为表格所需要每一行的数据
    ///
    /// 返回一个Vec列表元组，第一位&Str为元素的名称，第二位usize表示对应的数量
    pub fn rows(&self) -> Vec<(&str, usize)> {
        vec![
            ("Total", self.total),
            ("Done", self.done),
            ("Undone", self.undone),
        ]
    }
}
