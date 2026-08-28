//! CLI打印模块
//!
//! 利用comfy_table依赖，实现对于Tasks的表格打印

use comfy_table::{Cell, CellAlignment, Table, presets};

/// 根据传入的对齐参数列表，设置每一列的对齐
fn set_alignment(table: &mut Table, alignments: &[CellAlignment]) {
    for (i, align) in alignments.iter().enumerate() {
        if let Some(col) = table.column_mut(i) {
            col.set_cell_alignment(*align);
        }
    }
}

/// 根据传入的表头列表，设置每一列的表头
fn set_header(table: &mut Table, headers: &[&str]) {
    let header_vec: Vec<Cell> = headers
        .iter()
        .map(|h| Cell::new(h).set_alignment(CellAlignment::Center))
        .collect();
    table.set_header(header_vec);
}

/// 用于输出任务列表的表格
pub mod tasks_table {
    use chrono::{DateTime, Utc};
    use comfy_table::Color;

    use super::*;
    use crate::task::Task;
    use crate::time::to_local_time;

    const DEADLINE_COL: usize = 3;

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

    /// 将传入的整个`&[(usize, Task)]`(调整为自带编号，而不是在输出的时候编号)
    /// 整理为表格`Table`返回，用于list
    pub fn list_table(tasks: &[(usize, Task)], now: DateTime<Utc>) -> Table {
        let mut table = new_task_table();

        for (no, task) in tasks {
            let row = TaskRow { task, no: *no };
            table.add_row(row.to_table(now));
        }
        table
    }

    /// 将传入的`&Task`中的第no项，整理为表格`Table`返回，用于Show detail
    pub fn show_table(task: &Task, no: usize, now: DateTime<Utc>) -> Table {
        let mut table = new_task_table();

        let row = TaskRow { task, no };
        table.add_row(row.to_table(now));
        table
    }

    /// 新建一个表格，并确定表格的格式
    fn new_task_table() -> Table {
        let mut table = Table::new();
        table.load_style(presets::ASCII_MARKDOWN); // 表格风格：Markdown
        let headers = ["status", "no", "priority", "deadline", "task", "more"]; // 表头列表
        set_header(&mut table, &headers); // 设置表头

        // 对齐列表
        let alignments = [
            CellAlignment::Center, // 列1完成状态居中
            CellAlignment::Left,   // 列2序号居左
            CellAlignment::Center, // 列3优先级居中
            CellAlignment::Center, // 列4时间居中
            CellAlignment::Left,   // 列5任务标题居左
            CellAlignment::Center, // 列6更多信息居中
        ];

        set_alignment(&mut table, &alignments); // 设置对齐
        table
    }

    /// 定义了TaskRow结构体，标记Task的序号，用于输出
    ///
    /// 为何不对Task定义Display trait，主要考虑到输出序号的完整性，
    /// 使用id输出，在用户删改使用后，序号不连续，比较丑陋
    struct TaskRow<'a> {
        pub task: &'a Task, // 需要保证TaskRow的生命周期与Task相同
        pub no: usize,
    }

    impl TaskRow<'_> {
        /// 输出符合cli_print.rs中转换为表格所需要的数据格式
        ///
        /// 逻辑：
        /// 1. 使用✓符号标记是否完成
        /// 2. 标记是否有deadline，description
        /// 3. 检查是否已经过了截止日期，若过期且未完成，添加感叹号标记，并标黄
        /// 4. 整理需要打印的行，转换为`Vec<Cell>`供排版打印
        pub fn to_table(&self, now: DateTime<Utc>) -> Vec<Cell> {
            let task = self.task;
            let status: &str = if task.is_complete() { "✓" } else { " " };
            let overdue = task.is_overdue(now);
            let deadline = match task.deadline() {
                None => String::from("No"),
                Some(t) if overdue => format!("{} !", to_local_time(&t)),
                Some(t) => to_local_time(&t).to_string(),
            };
            let more = if task.description().is_some() {
                String::from("Show desc")
            } else {
                String::new()
            };

            [
                status.to_string(),
                self.no.to_string(),
                task.priority().to_string(),
                deadline,
                task.content().to_string(),
                more,
            ]
            .into_iter()
            .enumerate()
            .map(|(col, text)| {
                if overdue && col == DEADLINE_COL {
                    Cell::new(text).fg(Color::Rgb {
                        r: 245,
                        g: 210,
                        b: 45,
                    })
                } else {
                    Cell::new(text)
                }
            })
            .collect()
        }
    }
}

/// 用于输出任务状态的表格
pub mod tasks_status_table {
    use super::*;
    use crate::todo::TaskStatus;

    /// 将传入的任务状态结构体整理为表格`Table`返回，用于展示
    pub fn status_table(status: &TaskStatus) -> Table {
        let mut table = new_status_table();
        for (item, count) in status.rows() {
            table.add_row([item, &count.to_string()]);
        }
        table
    }

    /// 新建一个表格，并确定表格的格式
    fn new_status_table() -> Table {
        let mut table = Table::new();
        table.load_style(presets::UTF8_HORIZONTAL_ONLY); // 表格风格：UTF8 仅横线
        let headers = ["Items", "Count"]; // 表头列表
        set_header(&mut table, &headers); // 设置表头

        // 对齐列表
        let alignments = [
            CellAlignment::Center, // 列1元素名称居中
            CellAlignment::Center, // 列2元素统计数据居中
        ];

        set_alignment(&mut table, &alignments); // 设置对齐
        table
    }
}
