//! 主界面
//!
//! 四象限布局: 当地时间 / 任务status / task列表 / 选中task细节, 底部为按键提示栏

use chrono::{Local, Utc};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, Paragraph, Wrap};

use super::super::app::{App, AppState};
use super::super::theme::THEME;
use super::super::ui::priority_style;
use crate::time::to_local_time;

/// 绘制主界面
pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    frame.render_widget(Block::default().style(THEME.base_style()), area);

    let [top, bottom, footer] = Layout::vertical([
        Constraint::Length(7),
        Constraint::Min(8),
        Constraint::Length(1),
    ])
    .areas(area);
    let [clock_area, status_area] =
        Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)]).areas(top);
    let [tasks_area, details_area] =
        Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)]).areas(bottom);

    draw_clock(frame, clock_area);
    draw_status(frame, app, status_area);
    draw_tasks(frame, app, tasks_area);
    draw_details(frame, app, details_area);
    draw_footer(frame, app, footer);
}

/// 三行像素字体,用于显示时钟数字与冒号
fn glyph(ch: char) -> [&'static str; 3] {
    match ch {
        '0' => ["█▀█", "█ █", "█▄█"],
        '1' => ["▄█ ", " █ ", "▄█▄"],
        '2' => ["▀▀█", "▄█▀", "█▄▄"],
        '3' => ["▀▀█", " ▀▄", "▄▄█"],
        '4' => ["█ █", "█▄█", "  █"],
        '5' => ["█▀▀", "▀▀█", "▄▄█"],
        '6' => ["█▀▀", "█▀█", "█▄█"],
        '7' => ["▀▀█", " █ ", " █ "],
        '8' => ["█▀█", "█▀█", "█▄█"],
        '9' => ["█▀█", "▀▀█", "▄▄█"],
        ':' => [" ", "▀", "▄"],
        _ => ["   ", "   ", "   "],
    }
}

/// 左上角: 当地时间(大字时钟 + 日期)
fn draw_clock(frame: &mut Frame, area: Rect) {
    let now = Local::now();
    let time = now.format("%H:%M").to_string();

    let mut rows = [String::new(), String::new(), String::new()];
    for ch in time.chars() {
        let glyph = glyph(ch);
        for (row, part) in rows.iter_mut().zip(glyph) {
            row.push_str(part);
            row.push(' ');
        }
    }
    let digit_style = Style::default()
        .fg(THEME.peach)
        .add_modifier(Modifier::BOLD);
    let lines = vec![
        Line::from(""), // 往下移动一行更美观
        Line::from(Span::styled(rows[0].clone(), digit_style)),
        Line::from(Span::styled(rows[1].clone(), digit_style)),
        Line::from(Span::styled(rows[2].clone(), digit_style)),
        Line::from(Span::styled(
            format!("{} {}", now.format("%Y-%m-%d"), now.format("%A")),
            THEME.muted(),
        )),
    ];

    let block = Block::bordered()
        .title(Span::styled(" TIME ", THEME.title()))
        .border_style(Style::default().fg(THEME.border))
        .style(THEME.base_style());
    let clock = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Center);
    frame.render_widget(clock, area);
}

/// 右上角: 任务status统计
fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    let (total, done, undone, overdue) = app.status_counts(Utc::now());
    let overdue_style = if overdue > 0 {
        Style::default().fg(THEME.red)
    } else {
        Style::default().fg(THEME.text)
    };
    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(format!(" Total:   {:<5}", total), Style::default()),
            Span::styled(format!(" Undone:  {}", undone), Style::default()),
        ]),
        Line::from(vec![
            Span::styled(
                format!(" Done:    {:<5}", done),
                Style::default().fg(THEME.green),
            ),
            Span::styled(format!(" Overdue: {}", overdue), overdue_style),
        ]),
    ];
    let block = Block::bordered()
        .title(Span::styled(" STATUS ", THEME.title()))
        .border_style(Style::default().fg(THEME.border))
        .style(THEME.base_style());

    frame.render_widget(block.clone(), area); // 先渲染边框

    let inner_area = block.inner(area); // 边框内部的空间

    // 计算文本的垂直居中位置
    let content_height = lines.len() as u16;
    let vertical_padding = (inner_area.height.saturating_sub(content_height) / 2).saturating_sub(1); // 稍微偏上比偏下更好看

    let centered_area = Rect {
        x: inner_area.x,
        y: inner_area.y + vertical_padding,
        width: inner_area.width,
        height: content_height,
    };

    // 渲染不带边框的 Paragraph
    let paragraph = Paragraph::new(lines);

    frame.render_widget(paragraph, centered_area);
}

/// 左下角: task列表,多选模式下带勾选框
fn draw_tasks(frame: &mut Frame, app: &mut App, area: Rect) {
    let in_multi = matches!(app.state, AppState::MultiSelect);
    let title = match (in_multi, app.find.is_some()) {
        (true, _) => format!(" TASK ({}) [multi-select] ", app.tasks.len()),
        (false, true) => format!(" TASK ({}) [filtered] ", app.tasks.len()),
        (false, false) => format!(" TASK ({}) ", app.tasks.len()),
    };
    let block = Block::bordered()
        .title(Span::styled(title, THEME.title()))
        .border_style(Style::default().fg(THEME.border))
        .style(THEME.base_style());

    if app.tasks.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(Span::styled("+_+ No tasks.", THEME.muted())),
            Line::from(""),
            Line::from(Span::styled("Press ctrl+p to add one.", THEME.muted())),
        ])
        .block(block);
        frame.render_widget(empty, area);
        return;
    }

    let now = Utc::now();
    let items: Vec<ListItem> = app
        .tasks
        .iter()
        .map(|(no, task)| {
            let checkbox = if in_multi {
                if app.multi_selected.contains(&task.id()) {
                    "[x] "
                } else {
                    "[ ] "
                }
            } else {
                ""
            };
            let marker = if task.is_complete() {
                "✓"
            } else if task.is_overdue(now) {
                "!"
            } else {
                " "
            };
            let style = if task.is_complete() {
                Style::default()
                    .fg(THEME.muted)
                    .add_modifier(Modifier::CROSSED_OUT)
            } else {
                Style::default().fg(THEME.text)
            };
            let marker_style = if !task.is_complete() && task.is_overdue(now) {
                Style::default().fg(THEME.red).add_modifier(Modifier::BOLD)
            } else if task.is_complete() {
                Style::default().fg(THEME.green)
            } else {
                style
            };
            let line = Line::from(vec![
                Span::styled(format!(" {}{:>2} ", checkbox, no), style),
                Span::styled(format!("{} ", marker), marker_style),
                Span::styled(task.content().to_string(), style),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(THEME.highlight());
    frame.render_stateful_widget(list, area, &mut app.list_state);
}

/// 右下角: 选中task的细节
fn draw_details(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::bordered()
        .title(Span::styled(" DETAILS ", THEME.title()))
        .border_style(Style::default().fg(THEME.border))
        .style(THEME.base_style());

    let Some((no, task)) = app.selected_task() else {
        let empty = Paragraph::new(Line::from(Span::styled(
            "'_? No task selected",
            THEME.muted(),
        )))
        .block(block);
        frame.render_widget(empty, area);
        return;
    };

    let now = Utc::now();
    let (status_text, status_style) = if task.is_complete() {
        ("Done", Style::default().fg(THEME.green))
    } else if task.is_overdue(now) {
        (
            "Overdue",
            Style::default().fg(THEME.red).add_modifier(Modifier::BOLD),
        )
    } else {
        ("Undone", Style::default().fg(THEME.text))
    };
    let deadline = match task.deadline() {
        Some(d) => to_local_time(&d).format("%Y-%m-%d %H:%M:%S").to_string(),
        None => "'_? No deadline".to_string(),
    };
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                format!("#{} {}", no, task.content()),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(task.priority().to_string(), priority_style(task.priority())),
        ]),
        Line::from(Span::styled(deadline, Style::default().fg(THEME.text))),
        Line::from(vec![
            Span::styled("Status: ", THEME.muted()),
            Span::styled(status_text, status_style),
        ]),
        Line::from(""),
        Line::from(Span::styled("-Description-", THEME.muted())),
    ];
    match task.description() {
        None => lines.push(Line::from("'_? No description".to_string())),
        // 显式\n换行展示;超宽部分由Paragraph软换行,不算换行
        Some(desc) => {
            for line in desc.split('\n') {
                lines.push(Line::from(line.to_string()));
            }
        }
    }
    let details = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(details, area);
}

/// 底部: 按键提示 / 当前排序与搜索条件 / 操作消息
fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    // 表单弹窗内已展示校验错误信息,footer不再重复显示
    if let Some(message) = app.message.as_deref()
        && !matches!(app.state, AppState::Form(_))
    {
        let line = Line::from(Span::styled(
            format!(" {}", message),
            Style::default()
                .fg(THEME.yellow)
                .add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(Paragraph::new(line), area);
        return;
    }

    let hints = match app.state {
        AppState::Main => {
            " ↑/↓ move  space done  ^A add  ^D delete  ^E change  ^P cmds  ^F find  ^L sort"
        }
        AppState::TaskOptions | AppState::Settings => " ↑/↓ move  enter select  esc back",
        AppState::SearchInput => " type keyword  enter apply  esc back",
        AppState::MultiSelect => " ↑/↓ move  space select  enter confirm  esc cancel",
        AppState::SortMode => " sort by: [p]riority [d]eadline [n]one   esc cancel",
        AppState::Form(_) => {
            " tab next field  enter newline(desc)  ←/→ priority  ^S save  esc cancel"
        }
        AppState::Confirm(_) => " [y] confirm  [n/esc] cancel",
    };
    let mut spans = vec![Span::styled(hints, THEME.muted())];
    if let Some(sort) = &app.sort {
        let name = match sort {
            crate::todo::SortBy::Priority => "priority",
            crate::todo::SortBy::Deadline => "deadline",
        };
        spans.push(Span::styled(format!("  |  sort: {}", name), THEME.muted()));
    }
    if let Some(find) = &app.find {
        spans.push(Span::styled(
            format!("  |  find: \"{}\"", find),
            THEME.muted(),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}
