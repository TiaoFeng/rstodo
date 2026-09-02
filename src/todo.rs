//! 业务核心模块
//!
//! 实现项目所需的add, change, list, show, complete,
//! incomplete, delete, undo等接口

use chrono::{DateTime, Utc};
use clap::ValueEnum;
use std::cmp::Ordering;

use crate::{
    UserInterfaceTypes,
    error::AppError,
    io::storage::TaskStore,
    task::{Priority, Task},
    time::to_local_time,
};

/// 定义了用户可选择的两种排序方式，按照时间或优先级
#[derive(ValueEnum, Clone, Copy)]
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
        // 判断希望修改的content是否为空
        if let Some(content) = &self.content {
            validate_content(content)?;
        }
        // 判断希望修改的description是否为空
        if let Some(description) = &self.description {
            validate_desc(description.as_deref())?;
        }

        store.update_with_backup(|tasks| {
            if no == 0 || no > tasks.len() {
                return Err(AppError::TaskNotFound { no });
            }
            let task = &mut tasks[no - 1];

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

/// 判断content是否为空
fn validate_content(content: &str) -> Result<(), AppError> {
    if content.trim().is_empty() {
        return Err(AppError::InvalidContent {
            input: content.to_string(),
        });
    }
    Ok(())
}

/// 判断传入的description是否为空
fn validate_desc(description: Option<&str>) -> Result<(), AppError> {
    if let Some(desc) = description
        && desc.trim().is_empty()
    {
        return Err(AppError::InvalidDescription {
            input: desc.to_string(),
        });
    }
    Ok(())
}

pub fn add_task(
    store: &TaskStore,
    content: String,
    description: Option<String>,
    deadline: Option<DateTime<Utc>>,
    priority: Option<Priority>,
) -> Result<(), AppError> {
    validate_content(&content)?;
    validate_desc(description.as_deref())?;

    store.update_with_backup(|tasks| {
        let new_id: usize = tasks.iter().map(|t| t.id()).max().unwrap_or(0) + 1;
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

/// 将tasks列表中的task与task所在的序号，合并成一个元组，整理成列表返回
///
/// 用于下面list_table传入打印，不再由list_table按顺序生成序号，这样也可以支持更多操作
pub fn with_display_no(tasks: &[Task]) -> Vec<(usize, Task)> {
    tasks
        .iter()
        .enumerate()
        .map(|(i, t)| (i + 1, t.clone()))
        .collect()
}

/// 对已加载的任务做内存内的查找与排序，返回带原文件序号的列表（不落盘）
///
/// 查找与排序规则的唯一实现：list_tasks的不落盘与TUI快照渲染共用；
/// 序号取任务在快照中的原文件位置，排序不重编序号
pub fn view_tasks(
    tasks: &[Task],
    sort: Option<SortBy>,
    find: Option<String>,
) -> Vec<(usize, Task)> {
    let keyword_lower = find.map(|kw| kw.to_lowercase());
    let now = Utc::now();
    let mut listed: Vec<(usize, Task)> = match &keyword_lower {
        Some(kw) => tasks
            .iter()
            .enumerate()
            .filter(|(_, t)| find_tasks(t, kw, now))
            .map(|(i, t)| (i + 1, t.clone()))
            .collect(),
        None => with_display_no(tasks),
    };
    if let Some(order) = sort {
        sort_tasks(&mut listed, order, |(_, t)| t);
    }
    listed
}

/// list业务函数
///
/// 传入参数sort,排序方式
/// 传入参数find,查找的内容
/// 传出带任务编号的Vec<(usize, Task)>
///
/// 逻辑：
/// - 只有在cli交互下，进行排序操作，且不查找，才落盘，修改tasks.json文件
/// - 其他条件下，都在内存中进行排序和查找
pub fn list_tasks(
    store: &TaskStore,
    sort: Option<SortBy>,
    find: Option<String>,
) -> Result<Option<Vec<(usize, Task)>>, AppError> {
    // 载入列表
    let tasks = store.load()?;
    // 先判断是否为空，避免对空列表查找，浪费时间
    if tasks.is_empty() {
        return Ok(None);
    }

    // 落盘操作，输出的序号连续且有序，更加美观：
    // - 只有cli交互、排序且不查找才落盘；改写文件后按新快照重新编号返回
    // - 落盘与重载之间外部进程可能清空列表，保留与内存路径一致的判空返回None
    if let Some(order) = &sort
        && store.interface_type() == UserInterfaceTypes::Cli
        && find.is_none()
    {
        store.update_without_backup(move |tasks| {
            sort_tasks(tasks, *order, |t| t);
            Ok(())
        })?;
        let listed = with_display_no(&store.load()?);
        return Ok((!listed.is_empty()).then_some(listed));
    }

    // 内存操作，不落盘：
    // - 用户使用tui交互 || 既排序又查找
    let result_tasks = view_tasks(&tasks, sort, find); // 复用新增的view_tasks函数

    if result_tasks.is_empty() {
        return Ok(None);
    }
    Ok(Some(result_tasks))
}

/// 排序函数
///
/// 实现了基于任务deadline和priority的两种排序模式
/// 使用泛型同时兼容带序号的tasks列表和不带序号的tasks列表
fn sort_tasks<T, F>(items: &mut [T], order: SortBy, get_task: F)
where
    F: Fn(&T) -> &Task,
{
    match order {
        SortBy::Deadline => {
            items.sort_by(|a, b| {
                let ta = get_task(a);
                let tb = get_task(b);
                match (ta.deadline(), tb.deadline()) {
                    (Some(d1), Some(d2)) => d1.cmp(&d2),
                    (Some(_), None) => Ordering::Less,
                    (None, Some(_)) => Ordering::Greater,
                    (None, None) => Ordering::Equal,
                }
            });
        }
        SortBy::Priority => items.sort_by_key(|t| get_task(t).priority()),
    }
}

/// 搜索函数
///
/// 检查任务中是否匹配关键词
fn find_tasks(task: &Task, keyword: &str, now: DateTime<Utc>) -> bool {
    match keyword {
        "done" => task.is_complete(),
        "undone" | "todo" => !task.is_complete(),
        "overdue" => task.is_overdue(now),
        _ => {
            task.content().to_lowercase().contains(keyword)
                || task
                    .description()
                    .is_some_and(|desc| desc.to_lowercase().contains(keyword))
                || task.priority().to_string().to_lowercase().contains(keyword)
                || task.deadline().is_some_and(|deadline| {
                    to_local_time(&deadline)
                        .to_string()
                        .to_lowercase()
                        .contains(keyword)
                })
        }
    }
}

pub fn show_details(no: usize, store: &TaskStore) -> Result<Option<Task>, AppError> {
    let tasks: Vec<Task> = store.load()?;
    if tasks.is_empty() {
        Ok(None)
    } else if no == 0 || no > tasks.len() {
        Err(AppError::TaskNotFound { no })
    } else {
        let task: Task = tasks[no - 1].clone(); // 上方判定过 no 一定大于0且小于列表长度,不会下溢或越界
        Ok(Some(task))
    }
}

/// 为complete， incomplete和delete实现统一的函数
///
/// 这三者的代码高度相似，利用闭包减少代码重复
fn update_task<F>(mut nos: Vec<usize>, store: &TaskStore, mut action: F) -> Result<(), AppError>
where
    F: FnMut(usize, &mut Vec<Task>),
{
    store.update_with_backup(|tasks| {
        nos.sort_unstable();
        nos.dedup();
        for &no in &nos {
            if no == 0 || no > tasks.len() {
                return Err(AppError::TaskNotFound { no });
            }
        }
        // 删除应当从反向遍历，防止顺序改变，全部从反向也没有后果
        for no in nos.into_iter().rev() {
            action(no, tasks);
        }
        Ok(())
    })
}

pub fn complete_task(nos: Vec<usize>, store: &TaskStore) -> Result<(), AppError> {
    update_task(nos, store, |no, tasks| tasks[no - 1].complete())
}

pub fn incomplete_task(nos: Vec<usize>, store: &TaskStore) -> Result<(), AppError> {
    update_task(nos, store, |no, tasks| tasks[no - 1].incomplete())
}

pub fn delete_task(nos: Vec<usize>, store: &TaskStore) -> Result<(), AppError> {
    update_task(nos, store, |no, tasks| {
        tasks.remove(no - 1);
    })
}

/// 从主文件中返回所有完成项目的预览，用于预览delete alldone要删除哪些内容
pub fn delete_alldone_preview(store: &TaskStore) -> Result<Vec<Task>, AppError> {
    let tasks = store.load()?;
    // 整理所有完成的tasks，就是要删除的
    let done_tasks: Vec<Task> = tasks.into_iter().filter(|t| t.is_complete()).collect();
    if done_tasks.is_empty() {
        return Err(AppError::NothingToDelete);
    }
    Ok(done_tasks)
}

/// 执行删除所有完成项目，需要与snapshot对比，检查文件是否在预览后被篡改
pub fn delete_alldone_apply(store: &TaskStore, snapshot: &[Task]) -> Result<(), AppError> {
    store.update_with_backup(|tasks| {
        let current: Vec<Task> = tasks.iter().filter(|t| t.is_complete()).cloned().collect();
        if current != snapshot {
            return Err(AppError::DeleteConflict);
        }
        tasks.retain(|t| !t.is_complete());
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
    overdue: usize,
}

impl TaskStatus {
    /// 收集传入TaskStore的任务状态
    ///
    /// 包括：
    /// - total：总计任务数量
    /// - done： 完成的任务数量
    /// - undone： 未完成的任务数量
    /// - overdue: 逾期的任务数量
    pub fn collect(store: &TaskStore, now: DateTime<Utc>) -> Result<Self, AppError> {
        let tasks = store.load()?;
        let total = tasks.len();
        let done = tasks.iter().filter(|task| task.is_complete()).count();
        let overdue = tasks.iter().filter(|task| task.is_overdue(now)).count();
        Ok(TaskStatus {
            total,
            done,
            undone: total - done,
            overdue,
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
            ("Overdue", self.overdue),
        ]
    }
}
