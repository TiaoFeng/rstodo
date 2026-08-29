//! 使用Enter进入选中task菜单
//!
//! 为task实现change,done-undone,delete操作

use ratatui::Frame;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, List, ListItem, ListState};

use super::super::app::App;
use super::super::theme::THEME;
use super::super::ui::centered_rect;

/// 多选确认后的选项菜单
pub const MULTI_ITEMS: [&str; 3] = ["done", "undone", "delete"];

/// 选中task的选项菜单项,done/undone随任务完成状态切换
pub fn items(app: &App) -> Vec<String> {
    let change_status = if app.selected_task().is_some_and(|(_, t)| t.is_complete()) {
        "undone"
    } else {
        "done"
    };
    vec![
        change_status.to_string(),
        "change".to_string(),
        "delete".to_string(),
    ]
}

/// enter进入的选中task选项弹窗
pub fn draw(frame: &mut Frame, app: &App) {
    menu_popup(frame, &items(app), app.menu_index);
}

/// 多选确认后的done/undone/delete选项弹窗
pub fn draw_multi_menu(frame: &mut Frame, app: &App) {
    let items: Vec<String> = MULTI_ITEMS.iter().map(|s| s.to_string()).collect();
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
