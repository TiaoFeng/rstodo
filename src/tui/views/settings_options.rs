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
    widgets::{Block, Clear, Paragraph},
};

use crate::tui::{
    app::{App, AppState},
    theme::THEME,
    ui::{centered_rect, input_window, menu_popup},
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
///
/// 统一使用ui::menu_popup渲染
fn draw_palette(frame: &mut Frame, app: &App) {
    let items: Vec<&str> = SettingMenu::ALL.iter().map(|cmd| cmd.label()).collect();
    menu_popup(frame, " Commands ", &items, app.menu_index, 44);
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
    let (visible_input, cursor_col) = input_window(
        app.search_line.value(),
        app.search_line.cursor(),
        value_width,
    );
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
