use crate::task::{Task, TaskRow};
use comfy_table::{Cell, CellAlignment, Table, presets};

pub fn list_table(tasks: &[Task]) -> Table {
    let mut table = Table::new();
    table.load_style(presets::ASCII_MARKDOWN);
    table.set_header(vec![
        Cell::new("status").set_alignment(CellAlignment::Center),
        Cell::new("no").set_alignment(CellAlignment::Center),
        Cell::new("priority").set_alignment(CellAlignment::Center),
        Cell::new("deadline").set_alignment(CellAlignment::Center),
        Cell::new("task").set_alignment(CellAlignment::Center),
        Cell::new("more").set_alignment(CellAlignment::Center),
    ]);

    for (i, task) in tasks.iter().enumerate() {
        let row = TaskRow { task, no: i + 1 };
        table.add_row(row.to_table());
    }

    table
        .column_mut(0)
        .unwrap()
        .set_cell_alignment(CellAlignment::Center);
    table
        .column_mut(1)
        .unwrap()
        .set_cell_alignment(CellAlignment::Left);
    table
        .column_mut(2)
        .unwrap()
        .set_cell_alignment(CellAlignment::Center);
    table
        .column_mut(3)
        .unwrap()
        .set_cell_alignment(CellAlignment::Center);
    table
        .column_mut(4)
        .unwrap()
        .set_cell_alignment(CellAlignment::Left);
    table
        .column_mut(5)
        .unwrap()
        .set_cell_alignment(CellAlignment::Center);
    table
}

pub fn show_table(task: &Task, no: usize) -> Table {
    let mut table = Table::new();
    table.load_style(presets::ASCII_MARKDOWN);
    table.set_header(vec![
        Cell::new("status").set_alignment(CellAlignment::Center),
        Cell::new("no").set_alignment(CellAlignment::Center),
        Cell::new("priority").set_alignment(CellAlignment::Center),
        Cell::new("deadline").set_alignment(CellAlignment::Center),
        Cell::new("task").set_alignment(CellAlignment::Center),
        Cell::new("more").set_alignment(CellAlignment::Center),
    ]);
    let row = TaskRow { task, no };
    table.add_row(row.to_table());

    table
        .column_mut(0)
        .unwrap()
        .set_cell_alignment(CellAlignment::Center);
    table
        .column_mut(1)
        .unwrap()
        .set_cell_alignment(CellAlignment::Left);
    table
        .column_mut(2)
        .unwrap()
        .set_cell_alignment(CellAlignment::Center);
    table
        .column_mut(3)
        .unwrap()
        .set_cell_alignment(CellAlignment::Center);
    table
        .column_mut(4)
        .unwrap()
        .set_cell_alignment(CellAlignment::Left);
    table
        .column_mut(5)
        .unwrap()
        .set_cell_alignment(CellAlignment::Center);
    table
}
