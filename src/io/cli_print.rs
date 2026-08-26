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
    use super::*;
    use crate::task::{Task, TaskRow};

    /// 将传入的整个`&[Task]`整理为表格`Table`返回，用于list
    pub fn list_table(tasks: &[Task]) -> Table {
        let mut table = new_task_table();

        for (i, task) in tasks.iter().enumerate() {
            let row = TaskRow { task, no: i + 1 };
            table.add_row(row.to_table());
        }
        table
    }

    /// 将传入的`&Task`中的第no项，整理为表格`Table`返回，用于Show detail
    pub fn show_table(task: &Task, no: usize) -> Table {
        let mut table = new_task_table();

        let row = TaskRow { task, no };
        table.add_row(row.to_table());
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
