//! 使用Enter进入选中task菜单
//!
//! 为task实现change,done-undone,delete操作

use ratatui::{
    Frame,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, ListState},
};

use crate::tui::{app::App, theme::THEME, ui::centered_rect};

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

    pub fn label(self, app: &App) -> &'static str {
        match self {
            TaskOpMenu::StatusChange => {
                if app.selected_task().is_some_and(|(_, t)| t.is_complete()) {
                    "Undone"
                } else {
                    "Done"
                }
            }
            TaskOpMenu::Change => "Change",
            TaskOpMenu::Delete => "Delete",
        }
    }
}

pub fn items(app: &App) -> Vec<String> {
    TaskOpMenu::ALL
        .iter()
        .map(|m| m.label(app).to_string())
        .collect()
}

/// enter进入的选中task选项弹窗
pub fn draw(frame: &mut Frame, app: &App) {
    menu_popup(frame, &items(app), app.menu_index);
}

/// 多选确认后的done/undone/delete选项弹窗
pub fn draw_multi_menu(frame: &mut Frame, app: &App) {
    let items: Vec<String> = MultiOpMenu::ALL
        .iter()
        .map(|op| op.label().to_string())
        .collect();
    menu_popup(frame, &items, app.menu_index);
}

/// 通用的居中选项菜单弹窗
fn menu_popup(frame: &mut Frame, items: &[String], selected: usize) {
    let area = centered_rect(frame.area(), 26, items.len() as u16 + 4);
    frame.render_widget(Clear, area);

    let block = Block::bordered()
        .title(Span::styled(" options ", THEME.title()))
        .title_top(Line::from(Span::styled(" esc ", THEME.muted())).right_aligned())
        .border_style(Style::default().fg(THEME.border))
        .style(THEME.surface_style());
    let rows: Vec<ListItem> = items
        .iter()
        .map(|item| ListItem::new(Line::from(format!("  {}", item))))
        .collect();
    let mut state = ListState::default();
    state.select(Some(selected));
    let list = List::new(rows)
        .block(block)
        .highlight_style(THEME.highlight());
    frame.render_stateful_widget(list, area, &mut state);
}
