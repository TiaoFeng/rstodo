//! TUI模块
//!
//! 模块声明和tui界面入口

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
};
use ratatui::DefaultTerminal;
use std::{io::stdout, time::Duration};

use crate::{error::AppError, io::storage::TaskStore, tui::app::App};

mod app;
mod form_state;
mod handler;
mod text;
mod theme;
mod ui;
mod views;

#[cfg(test)]
mod tests;

/// TUI入口: 初始化终端,进入主循环,退出前恢复终端
///
/// 使用ratatui::try_init/try_restore: 内部会安装panic hook,
/// 即使主循环panic也会先恢复终端再展开panic
pub fn run(store: &TaskStore) -> Result<(), AppError> {
    let mut terminal = match ratatui::try_init() {
        Ok(terminal) => terminal,
        Err(err) => {
            // try_init 可能在开启 raw mode 后失败，仍需尝试恢复。
            ratatui::restore();
            return Err(err.into());
        }
    };
    // 所有后续io动作的错误都收集为变量而不用`?`提前返回:
    // 任何一步失败都不得跳过后续清理, 否则终端会停留在raw mode
    let mouse_enable = execute!(stdout(), EnableMouseCapture);
    let result = main_loop(&mut terminal, store);
    let mouse_disable = execute!(stdout(), DisableMouseCapture);
    let cursor_result = terminal.show_cursor();
    let restore_result = ratatui::try_restore();
    // 按诊断价值排序上报: 主循环错误优先，清理错误次之
    result?;
    mouse_enable?;
    mouse_disable?;
    cursor_result?;
    restore_result?;
    Ok(())
}

/// 主循环: 绘制界面并分发键盘事件,定时刷新时钟
fn main_loop(terminal: &mut DefaultTerminal, store: &TaskStore) -> Result<(), AppError> {
    let mut app = App::new(store)?;
    while !app.should_quit() {
        app.expire_message();
        terminal.draw(|frame| ui::draw(frame, &mut app))?;
        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    handler::handle_key(&mut app, key);
                    app.consume_notice();
                }
                Event::Mouse(_) => {}
                _ => {}
            }
        }
    }
    Ok(())
}
