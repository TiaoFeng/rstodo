//! 使用ctrl+p进入的控制面板
//!
//! 包含：
//! - 内联搜索输入框
//! - add
//! - multiple choices
//! - delete all done
//! - undo
//! - exit

use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph},
};

use crate::tui::{
    app::{App, AppState},
    theme::THEME,
    ui::{centered_rect, input_window},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingMenu {
    Search,
    Add,
    MultipleChoices,
    DeleteAllDone,
    Undo,
    Exit,
}

impl SettingMenu {
    pub const ALL: [Self; 6] = [
        SettingMenu::Search,
        SettingMenu::Add,
        SettingMenu::MultipleChoices,
        SettingMenu::DeleteAllDone,
        SettingMenu::Undo,
        SettingMenu::Exit,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Search => "Search",
            Self::Add => "Add",
            Self::MultipleChoices => "Multiple Choices",
            Self::DeleteAllDone => "Delete All Done",
            Self::Undo => "Undo",
            Self::Exit => "Exit",
        }
    }
}

/// 绘制ctrl+p命令面板(或内嵌搜索输入框)
pub fn draw(frame: &mut Frame, app: &App) {
    match app.state {
        AppState::SearchInput => draw_search(frame, app),
        _ => draw_palette(frame, app),
    }
}

/// 命令面板
fn draw_palette(frame: &mut Frame, app: &App) {
    let area = centered_rect(frame.area(), 44, SettingMenu::ALL.len() as u16 + 4);
    frame.render_widget(Clear, area);

    let block = Block::bordered()
        .title(Span::styled(" Commands ", THEME.title()))
        .title_top(Line::from(Span::styled(" esc ", THEME.muted())).right_aligned())
        .border_style(Style::default().fg(THEME.border))
        .style(THEME.surface_style());
    let rows: Vec<ListItem> = SettingMenu::ALL
        .iter()
        .map(|cmd| ListItem::new(Line::from(format!("  {}", cmd.label()))))
        .collect();
    let mut state = ListState::default();
    state.select(Some(app.menu_index));
    let list = List::new(rows)
        .block(block)
        .highlight_style(THEME.highlight());
    frame.render_stateful_widget(list, area, &mut state);
}

/// 面板内嵌的搜索输入框
fn draw_search(frame: &mut Frame, app: &App) {
    let area = centered_rect(frame.area(), 50, 7);
    frame.render_widget(Clear, area);

    let block = Block::bordered()
        .title(Span::styled(" Search ", THEME.title()))
        .border_style(Style::default().fg(THEME.border))
        .style(THEME.surface_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [input_area, _, hint_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    let value_width = input_area.width.saturating_sub(3);
    let (visible_input, cursor_col) =
        input_window(&app.search_input, app.search_cursor, value_width);
    let input = Line::from(vec![
        Span::styled(
            ">_ ", // 开头引导标记，3字符宽度
            Style::default()
                .fg(THEME.peach)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(visible_input),
    ]);
    frame.render_widget(Paragraph::new(input), input_area);
    // 光标置于编辑位置(默认在内容末尾)
    if value_width > 0 {
        frame.set_cursor_position((input_area.x + 3 + cursor_col, input_area.y));
    }

    let hint = Line::from(Span::styled("enter apply · esc back", THEME.muted()));
    frame.render_widget(Paragraph::new(hint), hint_area);
}

/// 通用二次确认弹窗(y确认 / n、esc取消)，用于delete all done和undo
pub fn draw_confirm(frame: &mut Frame, title: &str, text: &str) {
    let area = centered_rect(frame.area(), 40, 6);
    frame.render_widget(Clear, area);

    let block = Block::bordered()
        .title(Span::styled(title, THEME.title()))
        .border_style(Style::default().fg(THEME.red))
        .style(THEME.surface_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [text_area, hint_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Length(2)]).areas(inner);
    let text = Line::from(Span::styled(text, Style::default().fg(THEME.text)));
    frame.render_widget(Paragraph::new(text), text_area);
    let hint = Line::from(Span::styled("[y] confirm  [N/esc] cancel", THEME.muted()));
    frame.render_widget(Paragraph::new(hint), hint_area);
}
