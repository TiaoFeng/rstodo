//! CLI打印模块
//!
//! 利用comfy_table依赖，实现对于Tasks的表格打印
use crate::task::{Task, TaskRow};
use comfy_table::{Cell, CellAlignment, Table, presets};

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
    set_header(&mut table);

    // 设置对齐
    let alignments = [
        CellAlignment::Center, // 列1完成状态居中
        CellAlignment::Left,   // 列2序号居左
        CellAlignment::Center, // 列3优先级居中
        CellAlignment::Center, // 列4时间居中
        CellAlignment::Left,   // 列5任务标题居左
        CellAlignment::Center, // 列6更多信息居中
    ];

    for (i, align) in alignments.iter().enumerate() {
        if let Some(col) = table.column_mut(i) {
            col.set_cell_alignment(*align);
        }
    }
    table
}

/// 设置表头
fn set_header(table: &mut Table) {
    table.set_header(vec![
        Cell::new("status").set_alignment(CellAlignment::Center),
        Cell::new("no").set_alignment(CellAlignment::Center),
        Cell::new("priority").set_alignment(CellAlignment::Center),
        Cell::new("deadline").set_alignment(CellAlignment::Center),
        Cell::new("task").set_alignment(CellAlignment::Center),
        Cell::new("more").set_alignment(CellAlignment::Center),
    ]);
}
