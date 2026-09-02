//! 布局分割UI模块
//!
//! 按照功能分为四部分：
//! - 左上角显示当地时间
//! - 右上角显示任务status
//! - 左下角显示任务列表
//! - 右下角显示任务列表中选中项的细节

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph},
};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    task::Priority,
    tui::{
        app::{App, AppState, ConfirmAction},
        theme::THEME,
        views::{form, main_view, settings_options, task_options},
    },
};

const MINIMUM_WIDTH: u16 = 60;
const MINIMUM_HEIGHT: u16 = 16;

/// 绘制入口: 绘制主界面，再按状态叠加弹窗。
pub fn draw(frame: &mut Frame, app: &mut App) {
    if frame.area().width < MINIMUM_WIDTH || frame.area().height < MINIMUM_HEIGHT {
        frame.render_widget(Block::default().style(THEME.base_style()), frame.area());
        frame.render_widget(
            Paragraph::new(format!(
                "<'_> Terminal too small (minimum {} x {})",
                MINIMUM_WIDTH, MINIMUM_HEIGHT
            ))
            .style(THEME.muted())
            .alignment(Alignment::Center),
            centered_rect(frame.area(), frame.area().width, 1),
        );
        return;
    }
    main_view::draw(frame, app, frame.area());
    match app.state {
        AppState::TaskOptions => task_options::draw(frame, app),
        AppState::Settings | AppState::SearchInput => settings_options::draw(frame, app),
        AppState::MultiSelect if app.multi_menu_open => task_options::draw_multi_menu(frame, app),
        AppState::Form(_) => form::draw(frame, app),
        AppState::Confirm(ConfirmAction::DeleteAll(ref tasks)) => draw_confirm(
            frame,
            " Confirm ",
            &format!("Delete {} done task(s)?", tasks.len()),
        ),
        AppState::Confirm(ConfirmAction::Undo(ref tasks)) => draw_confirm(
            frame,
            " Undo ",
            &format!("Restore {} task(s) from backup?", tasks.len()),
        ),
        _ => {}
    }
}

/// 返回位于area正中的width×height小窗
pub fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}

/// 优先级对应的配色
pub fn priority_style(priority: Priority) -> Style {
    let color = match priority {
        Priority::High => THEME.red,
        Priority::Medium => THEME.yellow,
        Priority::Low => THEME.green,
    };
    Style::default().fg(color)
}

/// Ratatui 实际使用的终端显示宽度。
pub fn display_width(s: &str) -> u16 {
    Line::from(s).width().min(u16::MAX as usize) as u16
}

/// 截取光标附近的单行输入内容，并返回光标在截取结果中的显示列。
pub fn input_window(value: &str, cursor: usize, width: u16) -> (String, u16) {
    if width == 0 {
        return (String::new(), 0);
    }

    let prefix: String = value.chars().take(cursor).collect();
    let suffix: String = value.chars().skip(cursor).collect();
    let cursor_limit = width.saturating_sub(1) as usize;
    let mut visible_prefix = Vec::new();
    let mut prefix_width = 0;
    for grapheme in prefix.graphemes(true).rev() {
        let grapheme_width = Line::from(grapheme).width();
        if prefix_width + grapheme_width > cursor_limit {
            break;
        }
        visible_prefix.push(grapheme);
        prefix_width += grapheme_width;
    }
    visible_prefix.reverse();

    let mut visible = visible_prefix.concat();
    let mut total_width = prefix_width;
    for grapheme in suffix.graphemes(true) {
        let grapheme_width = Line::from(grapheme).width();
        if total_width + grapheme_width > width as usize {
            break;
        }
        visible.push_str(grapheme);
        total_width += grapheme_width;
    }
    (visible, prefix_width as u16)
}

/// 通用居中选项菜单弹窗
///
/// items为按显示顺序给出的菜单标签, selected为高亮项的下标
pub fn menu_popup(frame: &mut Frame, title: &str, items: &[&str], selected: usize, width: u16) {
    let area = centered_rect(frame.area(), width, items.len() as u16 + 4);
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(Span::styled(title.to_string(), THEME.title()))
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
