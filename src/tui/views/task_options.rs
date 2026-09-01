//! 使用Enter进入选中task菜单
//!
//! 为task实现change,done-undone,delete操作

use ratatui::Frame;

use crate::tui::{
    app::{App, TaskAction},
    ui::menu_popup,
};

/// 多选确认后的选项菜单
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiOpMenu {
    Done,
    Undone,
    Delete,
}

impl MultiOpMenu {
    pub const ALL: [Self; 3] = [MultiOpMenu::Done, MultiOpMenu::Undone, MultiOpMenu::Delete];

    pub fn label(self) -> &'static str {
        match self {
            MultiOpMenu::Done => "Done",
            MultiOpMenu::Undone => "Undone",
            MultiOpMenu::Delete => "Delete",
        }
    }

    /// 回车后执行的动作
    ///
    /// 与 label 由同一枚举变体派生, 新增菜单项时不会漏配
    pub fn action(self) -> TaskAction {
        match self {
            MultiOpMenu::Done => TaskAction::Complete,
            MultiOpMenu::Undone => TaskAction::Incomplete,
            MultiOpMenu::Delete => TaskAction::Delete,
        }
    }
}

/// 选中task的选项菜单项,done/undone随任务完成状态切换
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskOpMenu {
    StatusChange,
    Change,
    Delete,
}

impl TaskOpMenu {
    pub const ALL: [Self; 3] = [
        TaskOpMenu::StatusChange,
        TaskOpMenu::Change,
        TaskOpMenu::Delete,
    ];

    pub fn label(self, done: bool) -> &'static str {
        match self {
            TaskOpMenu::StatusChange if done => "Undone",
            TaskOpMenu::StatusChange => "Done",
            TaskOpMenu::Change => "Change",
            TaskOpMenu::Delete => "Delete",
        }
    }
}

/// enter进入的选中task选项弹窗
pub fn draw(frame: &mut Frame, app: &App) {
    let done = app.selected_done();
    let items: Vec<&str> = TaskOpMenu::ALL.iter().map(|op| op.label(done)).collect();
    menu_popup(frame, " options ", &items, app.menu_index, 26);
}

/// 多选确认后的done/undone/delete选项弹窗
pub fn draw_multi_menu(frame: &mut Frame, app: &App) {
    let items: Vec<&str> = MultiOpMenu::ALL.iter().map(|op| op.label()).collect();
    menu_popup(frame, " options ", &items, app.menu_index, 26);
}
