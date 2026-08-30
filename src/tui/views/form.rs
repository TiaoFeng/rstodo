//! add / change 输入表单
//!
//! 使用文本框让用户输入 content/desc/deadline/priority
//! description为三行高的多行文本框: enter换行(保存为\n),方向键移动光标,
//! 超出输入宽度的部分自动软换行显示(不写入\n),内容超出三行时自动滚动
//! 使用ctrl+s保存

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

use crate::tui::app::FormField;

use super::super::app::{App, AppState, DESC_VISIBLE_LINES, FormData, FormMode};
use super::super::theme::THEME;
use super::super::ui::{centered_rect, display_width, input_window, priority_style};

/// 输入框标签的显示宽度(标签12 + ": " 2)
const LABEL_WIDTH: u16 = 14;

/// 绘制add / change表单弹窗
pub fn draw(frame: &mut Frame, app: &mut App) {
    let message = app.message.clone();
    let AppState::Form(form) = &mut app.state else {
        return;
    };
    let title = match form.mode {
        FormMode::Add => " Add Task ".to_string(),
        FormMode::Change { no, .. } => format!(" Change Task #{} ", no),
    };
    let area = centered_rect(frame.area(), 62, 12);
    frame.render_widget(Clear, area);

    let block = Block::bordered()
        .title(Span::styled(title, THEME.title()))
        .border_style(Style::default().fg(THEME.border))
        .style(THEME.surface_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [
        content_area,
        desc_area,
        deadline_area,
        priority_area,
        _,
        hint_area,
    ] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(DESC_VISIBLE_LINES as u16),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    draw_text_field(
        frame,
        "Content",
        &form.content,
        form.content_cursor,
        None,
        content_area,
        form.focus == FormField::Content,
    );
    draw_description_field(frame, form, desc_area);
    draw_text_field(
        frame,
        "Deadline",
        &form.deadline,
        form.deadline_cursor,
        Some("2000-01-01 or 2000-01-01T12:00:00"),
        deadline_area,
        form.focus == FormField::Deadline,
    );
    draw_priority_field(frame, form, priority_area);

    // 表单内的校验错误优先显示,否则显示按键提示
    let hint = match message {
        Some(message) => Line::from(Span::styled(
            message,
            Style::default()
                .fg(THEME.yellow)
                .add_modifier(Modifier::BOLD),
        )),
        None => Line::from(Span::styled(
            "tab next field · enter newline · ctrl+s save · esc cancel",
            THEME.muted(),
        )),
    };
    frame.render_widget(Paragraph::new(hint), hint_area);
}

/// 输入框标签样式,聚焦时高亮
fn label_style(focused: bool) -> Style {
    if focused {
        Style::default()
            .fg(THEME.peach)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(THEME.purple)
    }
}

/// 绘制一行文本输入框,聚焦时在光标处显示编辑光标
fn draw_text_field(
    frame: &mut Frame,
    label: &str,
    value: &str,
    cursor: usize,
    placeholder: Option<&str>,
    area: Rect,
    focused: bool,
) {
    let input_width = area.width.saturating_sub(LABEL_WIDTH);
    let (visible_value, cursor_col) = input_window(value, cursor, input_width);
    let value_span = if value.is_empty()
        && let Some(hint) = placeholder
    {
        Span::styled(hint.to_string(), THEME.muted())
    } else {
        Span::styled(visible_value, Style::default().fg(THEME.text))
    };
    let line = Line::from(vec![
        Span::styled(format!("{:<12}: ", label), label_style(focused)),
        value_span,
    ]);
    frame.render_widget(Paragraph::new(line), area);
    if focused && input_width > 0 {
        frame.set_cursor_position((area.x + LABEL_WIDTH + cursor_col, area.y));
    }
}

/// 绘制三行高的description多行文本框
///
/// 文本按显式\n分行,超宽部分软换行显示;内容超过三行时随光标滚动,保持可见窗口高度不变
fn draw_description_field(frame: &mut Frame, form: &mut FormData, area: Rect) {
    let focused = form.focus == FormField::Description;
    // 文本区宽度随布局变化,渲染时更新供编辑逻辑计算软换行
    let text_width = (area.width as usize).saturating_sub(LABEL_WIDTH as usize);
    form.set_desc_wrap_width(text_width);
    form.adjust_desc_scroll();

    let texts: Vec<String> = form
        .desc_rows()
        .iter()
        .map(|&(start, end)| {
            form.description
                .chars()
                .skip(start)
                .take(end - start)
                .collect()
        })
        .collect();
    let scroll = form.desc_scroll();
    for i in 0..DESC_VISIBLE_LINES {
        let row_area = Rect {
            y: area.y + i as u16,
            ..area
        };
        let mut spans = Vec::new();
        if i == 0 {
            spans.push(Span::styled(
                format!("{:<12}: ", "Description"),
                label_style(focused),
            ));
        } else {
            spans.push(Span::raw(" ".repeat(LABEL_WIDTH as usize)));
        }
        match texts.get(scroll + i) {
            // 空文本框在第一行显示placeholder
            None if form.description.is_empty() && i == 0 => {
                spans.push(Span::styled("(optional)", THEME.muted()));
            }
            Some(text) => {
                spans.push(Span::styled(text.clone(), Style::default().fg(THEME.text)));
            }
            None => {}
        }
        frame.render_widget(Paragraph::new(Line::from(spans)), row_area);
    }
    if focused && text_width > 0 && area.height > 0 {
        let (row, col) = form.desc_cursor_row_col();
        let prefix: String = texts
            .get(row)
            .map_or(String::new(), |text| text.chars().take(col).collect());
        let cursor_x = area.x + LABEL_WIDTH + display_width(&prefix);
        let cursor_y = area.y + row.saturating_sub(scroll) as u16;
        frame.set_cursor_position((
            cursor_x.min(area.x + area.width.saturating_sub(1)),
            cursor_y.min(area.y + area.height.saturating_sub(1)),
        ));
    }
}

/// 绘制优先级选择框,使用左右键切换
fn draw_priority_field(frame: &mut Frame, form: &FormData, area: Rect) {
    let focused = form.focus == FormField::Priority;
    let arrow_style = if focused {
        Style::default().fg(THEME.peach)
    } else {
        THEME.muted()
    };
    let line = Line::from(vec![
        Span::styled(format!("{:<12}: ", "Priority"), label_style(focused)),
        Span::styled("< ", arrow_style),
        Span::styled(
            form.priority.to_string(),
            priority_style(form.priority).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" >", arrow_style),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}
