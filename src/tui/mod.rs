//! TUI模块
//!
//! 模块声明和tui界面入口

use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::DefaultTerminal;

use crate::{error::AppError, io::storage::TaskStore};
mod app;
mod handler;
mod theme;
mod tui_test;
mod ui;
mod views;

use app::App;

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
    let result = main_loop(&mut terminal, store);

    // 两个清理动作都必须执行；主循环错误比清理错误更有诊断价值。
    let cursor_result = terminal.show_cursor();
    let restore_result = ratatui::try_restore();
    result?;
    cursor_result?;
    restore_result?;
    Ok(())
}

/// 主循环: 绘制界面并分发键盘事件,定时刷新时钟
fn main_loop(terminal: &mut DefaultTerminal, store: &TaskStore) -> Result<(), AppError> {
    let mut app = App::new(store)?;
    while !app.should_quit {
        app.expire_message();
        terminal.draw(|frame| ui::draw(frame, &mut app))?;
        if event::poll(Duration::from_millis(250))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            handler::handle_key(&mut app, key);
            app.consume_notice();
        }
    }
    Ok(())
}
