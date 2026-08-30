//! 键盘事件分发模块
//!
//! 键盘事件包括：
//! 上下键上下移动task列表的光标
//! space修改任务是否完成
//! enter键进入选中task的task_options界面
//! ctrl+a添加
//! ctrl+e编辑
//! ctrl+f进入搜索
//! ctrl+p进入settings_options可以搜索和多选
//! 在多选界面上下键移动task列表的光标，用space选中一项，enter确认
//! 按esc返回main_view
//! 在form界面输入新增或改变task的信息后ctrl+s保存更改
//! 在主界面按ctrl+l选择排序模式，按再按p（按优先级排序），d（按截止日期排序），n（恢复默认顺序）
//! 在主界面按space切换选中task的done/undone
//! 在主界面按ctrl+e进入选中task的change表单
//! 在主界面按r刷新列表
//! 按q或ctrl+c退出tui

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::app::{App, AppState, ConfirmAction};
use super::views::{settings_options, task_options};
use crate::error::AppError;
use crate::task::Priority;
use crate::time::parse_deadline_input;
use crate::todo::{
    SortBy, add_task, delete_alldone_apply, delete_alldone_preview, undo_task_apply,
    undo_task_preview,
};

use crate::tui::form_state::{FormData, FormField, FormMode};
use crate::tui::text::{
    backspace_at_cursor, insert_at_cursor, move_cursor_left, move_cursor_right,
};

#[derive(Clone, Copy)]
enum TaskAction {
    Complete,
    Incomplete,
    Toggle,
    Delete,
}

/// 使用稳定 ID 在一次文件锁内定位并修改任务，避免长驻 TUI 的缓存序号失效。
fn apply_to_tasks(
    app: &App,
    task_refs: &[(usize, usize)], // (显示序号, 稳定 ID)
    action: TaskAction,
) -> Result<Option<bool>, AppError> {
    let mut toggled_to = None;
    app.store.update_with_backup(|tasks| {
        let mut positions = Vec::with_capacity(task_refs.len());
        for &(no, id) in task_refs {
            let position = tasks
                .iter()
                .position(|task| task.id() == id)
                .ok_or(AppError::TaskNotFound { no })?;
            positions.push(position);
        }
        positions.sort_unstable();
        positions.dedup();

        match action {
            TaskAction::Complete => {
                for position in positions {
                    tasks[position].complete();
                }
            }
            TaskAction::Incomplete => {
                for position in positions {
                    tasks[position].incomplete();
                }
            }
            TaskAction::Toggle => {
                let Some(&position) = positions.first() else {
                    return Ok(());
                };
                if tasks[position].is_complete() {
                    tasks[position].incomplete();
                    toggled_to = Some(false);
                } else {
                    tasks[position].complete();
                    toggled_to = Some(true);
                }
            }
            TaskAction::Delete => {
                for position in positions.into_iter().rev() {
                    tasks.remove(position);
                }
            }
        }
        Ok(())
    })?;
    Ok(toggled_to)
}

/// 键盘事件分发入口
///
/// 所有业务错误不在此处抛出,而是转化为底部提示信息
pub fn handle_key(app: &mut App, key: KeyEvent) {
    // ctrl+c在任何状态下退出
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }
    let result = match app.state {
        AppState::Main => handle_main(app, key),
        AppState::TaskOptions => handle_task_options(app, key),
        AppState::Settings => handle_settings(app, key),
        AppState::SearchInput => handle_search(app, key),
        AppState::MultiSelect => handle_multi(app, key),
        AppState::SortMode => handle_sort(app, key),
        AppState::Form(_) => handle_form(app, key),
        AppState::Confirm(_) => handle_confirm(app, key),
    };
    if let Err(err) = result {
        app.set_message(format!(":( {}", err.pack_to_tui_err()));
    }
}

/// 主界面
fn handle_main(app: &mut App, key: KeyEvent) -> Result<(), AppError> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('a') => {
                app.state = AppState::Form(FormData::add());
            }
            KeyCode::Char('d') => {
                let Some((no, task)) = app.selected_task().cloned() else {
                    app.state = AppState::Main;
                    return Ok(());
                };
                apply_to_tasks(app, &[(no, task.id())], TaskAction::Delete)?;
                app.set_message(format!("#_# Task {} deleted", no));
                app.reload()?;
                app.state = AppState::Main;
            }
            KeyCode::Char('e') => {
                let Some((no, task)) = app.selected_task().cloned() else {
                    app.state = AppState::Main;
                    return Ok(());
                };
                app.state = AppState::Form(FormData::change(no, &task));
            }
            KeyCode::Char('p') => {
                app.state = AppState::Settings;
                app.menu_index = 0;
                app.search_input.clear();
            }
            KeyCode::Char('f') => {
                app.search_input = app.find.clone().unwrap_or_default();
                app.search_cursor = app.search_input.chars().count();
                app.state = AppState::SearchInput;
            }
            KeyCode::Char('l') => app.state = AppState::SortMode,
            _ => {}
        }
        return Ok(());
    }
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Up | KeyCode::Char('k') => app.select_back(),
        KeyCode::Down | KeyCode::Char('j') => app.select_next(),
        KeyCode::Enter => {
            if app.selected_task().is_some() {
                app.state = AppState::TaskOptions;
                app.menu_index = 0;
            }
        }
        KeyCode::Char('r') => {
            app.reload()?;
            app.set_message(">>> Refreshed");
        }
        // 空格切换done/undone
        KeyCode::Char(' ') => {
            let Some((no, task)) = app.selected_task().cloned() else {
                return Ok(());
            };
            if apply_to_tasks(app, &[(no, task.id())], TaskAction::Toggle)? == Some(false) {
                app.set_message(format!("~_ Task {} undone", no));
            } else {
                app.set_message(format!("^_ Task {} done", no));
            }
            app.reload()?;
        }
        // 主界面按esc清除搜索过滤
        KeyCode::Esc if app.find.is_some() => {
            app.find = None;
            app.reload()?;
        }
        _ => {}
    }
    Ok(())
}

/// enter进入的选中task选项菜单: done|undone / change / delete
fn handle_task_options(app: &mut App, key: KeyEvent) -> Result<(), AppError> {
    let items_len = task_options::items(app).len();
    match key.code {
        KeyCode::Esc => app.state = AppState::Main,
        KeyCode::Up => app.menu_back(items_len),
        KeyCode::Down => app.menu_next(items_len),
        KeyCode::Enter => {
            let Some((no, task)) = app.selected_task().cloned() else {
                app.state = AppState::Main;
                return Ok(());
            };
            match app.menu_index {
                // done | undone
                0 => {
                    if apply_to_tasks(app, &[(no, task.id())], TaskAction::Toggle)? == Some(false) {
                        app.set_message(format!("~_ Task {} undone", no));
                    } else {
                        app.set_message(format!("^_ Task {} done", no));
                    }
                    app.reload()?;
                    app.state = AppState::Main;
                }
                // change
                1 => {
                    app.state = AppState::Form(FormData::change(no, &task));
                }
                // delete
                2 => {
                    apply_to_tasks(app, &[(no, task.id())], TaskAction::Delete)?;
                    app.set_message(format!("#_# Task {} deleted", no));
                    app.reload()?;
                    app.state = AppState::Main;
                }
                _ => unreachable!(),
            }
        }
        _ => {}
    }
    Ok(())
}

/// ctrl+p命令面板： search / add / multiple choices / delete all done / exit
fn handle_settings(app: &mut App, key: KeyEvent) -> Result<(), AppError> {
    match key.code {
        KeyCode::Esc => app.state = AppState::Main,
        KeyCode::Up => app.menu_back(settings_options::ITEMS.len()),
        KeyCode::Down => app.menu_next(settings_options::ITEMS.len()),
        KeyCode::Enter => match app.menu_index {
            // find
            0 => {
                app.search_input = app.find.clone().unwrap_or_default();
                app.search_cursor = app.search_input.chars().count();
                app.state = AppState::SearchInput;
            }
            // add
            1 => {
                app.state = AppState::Form(FormData::add());
            }
            // multiple select
            2 => {
                app.multi_selected.clear();
                app.multi_menu_open = false;
                app.state = AppState::MultiSelect;
                app.set_message("space to select, enter to confirm, esc to cancel");
            }
            // delete all done tasks
            3 => {
                let tasks = delete_alldone_preview(app.store)?;
                app.state = AppState::Confirm(ConfirmAction::DeleteAll(tasks));
            }
            // undo
            4 => {
                let tasks = undo_task_preview(app.store)?;
                app.state = AppState::Confirm(ConfirmAction::Undo(tasks));
            }
            // quit
            5 => app.should_quit = true,
            _ => unreachable!(),
        },
        _ => {}
    }
    Ok(())
}

/// 命令面板内嵌搜索输入框
fn handle_search(app: &mut App, key: KeyEvent) -> Result<(), AppError> {
    match key.code {
        KeyCode::Esc => app.state = AppState::Main,
        KeyCode::Enter => {
            let keyword = app.search_input.trim();
            app.find = if keyword.is_empty() {
                None
            } else {
                Some(keyword.to_string())
            };
            app.search_cursor = app.search_input.chars().count();
            app.reload()?;
            app.state = AppState::Main;
            match &app.find {
                Some(kw) => {
                    app.set_message(format!("Found {} task(s) for '{}'", app.tasks.len(), kw))
                }
                None => app.set_message("Search cleared"),
            }
        }
        KeyCode::Left => move_cursor_left(&app.search_input, &mut app.search_cursor),
        KeyCode::Right => move_cursor_right(&app.search_input, &mut app.search_cursor),
        KeyCode::Home => app.search_cursor = 0,
        KeyCode::End => app.search_cursor = app.search_input.chars().count(),
        KeyCode::Backspace => {
            backspace_at_cursor(&mut app.search_input, &mut app.search_cursor);
        }
        KeyCode::Char(c)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            insert_at_cursor(&mut app.search_input, &mut app.search_cursor, c)
        }
        _ => {}
    }
    Ok(())
}

/// 多选模式: 上下移动,space选中,enter确认弹出done/undone/delete菜单
fn handle_multi(app: &mut App, key: KeyEvent) -> Result<(), AppError> {
    if app.multi_menu_open {
        return handle_multi_menu(app, key);
    }
    match key.code {
        KeyCode::Esc => {
            app.multi_selected.clear();
            app.state = AppState::Main;
        }
        KeyCode::Up => app.select_back(),
        KeyCode::Down => app.select_next(),
        KeyCode::Char(' ') => {
            if let Some((_, task)) = app.selected_task() {
                let id = task.id();
                if !app.multi_selected.remove(&id) {
                    app.multi_selected.insert(id);
                }
            }
        }
        KeyCode::Enter => {
            if app.multi_selected.is_empty() {
                app.set_message(":( No tasks selected");
            } else {
                app.multi_menu_open = true;
                app.menu_index = 0;
            }
        }
        _ => {}
    }
    Ok(())
}

/// 多选确认后的done/undone/delete选项菜单
fn handle_multi_menu(app: &mut App, key: KeyEvent) -> Result<(), AppError> {
    match key.code {
        KeyCode::Esc => app.multi_menu_open = false,
        KeyCode::Up => app.menu_back(task_options::MULTI_ITEMS.len()),
        KeyCode::Down => app.menu_next(task_options::MULTI_ITEMS.len()),
        KeyCode::Enter => {
            let task_refs: Vec<(usize, usize)> = app
                .tasks
                .iter()
                .filter(|(_, task)| app.multi_selected.contains(&task.id()))
                .map(|(no, task)| (*no, task.id()))
                .collect();
            let count = task_refs.len();
            match app.menu_index {
                0 => apply_to_tasks(app, &task_refs, TaskAction::Complete)?,
                1 => apply_to_tasks(app, &task_refs, TaskAction::Incomplete)?,
                2 => apply_to_tasks(app, &task_refs, TaskAction::Delete)?,
                _ => unreachable!(),
            };
            let verb = match app.menu_index {
                0 => "done",
                1 => "undone",
                2 => "deleted",
                _ => unreachable!(),
            };
            app.set_message(format!(">>> {} task(s) {}", count, verb));
            app.multi_selected.clear();
            app.multi_menu_open = false;
            app.reload()?;
            app.state = AppState::Main;
        }
        _ => {}
    }
    Ok(())
}

/// delete all done / undo 的二次确认弹窗。
fn handle_confirm(app: &mut App, key: KeyEvent) -> Result<(), AppError> {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            let state = std::mem::replace(&mut app.state, AppState::Settings);
            let AppState::Confirm(action) = state else {
                return Ok(());
            };
            match action {
                ConfirmAction::DeleteAll(snapshot) => {
                    let count = snapshot.len();
                    delete_alldone_apply(app.store, &snapshot)?;
                    app.reload()?;
                    app.set_message(format!("'_? Deleted {} done task(s)", count));
                }
                ConfirmAction::Undo(snapshot) => {
                    let count = snapshot.len();
                    undo_task_apply(app.store, &snapshot)?;
                    app.reload()?;
                    app.set_message(format!("'_? Restored {} task(s)", count));
                }
            }
            app.state = AppState::Main;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.state = AppState::Settings;
            app.set_message(">_< Operation cancelled.");
        }
        _ => {}
    }
    Ok(())
}

/// ctrl+l排序模式: p按优先级排序, d按截止日期排序
fn handle_sort(app: &mut App, key: KeyEvent) -> Result<(), AppError> {
    match key.code {
        KeyCode::Char('p') => {
            app.sort = Some(SortBy::Priority);
            app.reload()?;
            app.state = AppState::Main;
            app.set_message(">>> Sorted by priority");
        }
        KeyCode::Char('d') => {
            app.sort = Some(SortBy::Deadline);
            app.reload()?;
            app.state = AppState::Main;
            app.set_message(">>> Sorted by deadline");
        }
        KeyCode::Char('n') => {
            app.sort = None;
            app.reload()?;
            app.state = AppState::Main;
            app.set_message(">>> Sorted by default");
        }
        KeyCode::Esc => app.state = AppState::Main,
        _ => {}
    }
    Ok(())
}

/// add / change 表单
fn handle_form(app: &mut App, key: KeyEvent) -> Result<(), AppError> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
        return save_form(app);
    }
    if key.code == KeyCode::Esc {
        app.state = AppState::Main;
        return Ok(());
    }
    let AppState::Form(form) = &mut app.state else {
        return Ok(());
    };
    match key.code {
        // 统一使用tab循环切换输入栏: content -> description -> deadline -> priority -> content
        KeyCode::Tab => form.focus = form.focus.next(),
        KeyCode::BackTab => form.focus = form.focus.back(),
        // 方向键只用于移动光标: description栏在多行文本中移动(自动滚动/软换行)
        KeyCode::Enter if form.focus == FormField::Description => form.desc_insert('\n'),
        KeyCode::Up if form.focus == FormField::Description => form.desc_up(),
        KeyCode::Down if form.focus == FormField::Description => form.desc_down(),
        KeyCode::Left if form.focus == FormField::Description => form.desc_left(),
        KeyCode::Right if form.focus == FormField::Description => form.desc_right(),
        KeyCode::Backspace if form.focus == FormField::Description => form.desc_backspace(),
        KeyCode::Char(c)
            if form.focus == FormField::Description
                && !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            form.desc_insert(c)
        }
        // priority栏使用左右键切换优先级
        KeyCode::Right if form.focus == FormField::Priority => {
            form.priority = match form.priority {
                Priority::High => Priority::Low,
                Priority::Medium => Priority::High,
                Priority::Low => Priority::Medium,
            };
        }
        KeyCode::Left if form.focus == FormField::Priority => {
            form.priority = match form.priority {
                Priority::High => Priority::Medium,
                Priority::Medium => Priority::Low,
                Priority::Low => Priority::High,
            };
        }
        // content/deadline单行输入框: 左右键移动光标
        KeyCode::Left if form.focus != FormField::Priority => {
            let (field, cursor) = single_field_mut(form);
            move_cursor_left(field, cursor);
        }
        KeyCode::Right if form.focus != FormField::Priority => {
            let (field, cursor) = single_field_mut(form);
            move_cursor_right(field, cursor);
        }
        KeyCode::Backspace if form.focus != FormField::Priority => {
            let (field, cursor) = single_field_mut(form);
            backspace_at_cursor(field, cursor);
        }
        KeyCode::Char(c)
            if form.focus != FormField::Priority
                && !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            let (field, cursor) = single_field_mut(form);
            insert_at_cursor(field, cursor, c);
        }
        _ => {}
    }
    Ok(())
}

/// 返回当前聚焦的单行输入框(content/deadline)的文本与光标(调用前需判断focus为Content或Deadline)
fn single_field_mut(form: &mut FormData) -> (&mut String, &mut usize) {
    match form.focus {
        FormField::Content => (&mut form.content, &mut form.content_cursor),
        FormField::Deadline => (&mut form.deadline, &mut form.deadline_cursor),
        _ => unreachable!(),
    }
}

/// ctrl+s保存表单
///
/// 输入非法或写入失败时保留表单，成功后保存并返回主界面。
fn save_form(app: &mut App) -> Result<(), AppError> {
    let Some(form) = app.form().cloned() else {
        return Ok(());
    };
    let content = form.content.trim().to_string();
    if content.is_empty() {
        app.set_message(":( Invalid content: content cannot be left blank.");
        return Ok(());
    }
    let description = match form.description.trim() {
        "" => None,
        desc => Some(desc.to_string()),
    };
    let deadline = match form.deadline.trim() {
        "" => None,
        input => match parse_deadline_input(input) {
            Ok(d) => Some(d),
            Err(err) => {
                app.set_message(format!(":( {}", err.pack_to_tui_err()));
                return Ok(());
            }
        },
    };
    let message = match form.mode {
        FormMode::Add => {
            add_task(
                app.store,
                content,
                description,
                deadline,
                Some(form.priority),
            )?;
            ">>> Task added".to_string()
        }
        FormMode::Change { no, id } => {
            // 表单预填了所有字段,保存时全部应用: 留空的desc/deadline视为清除
            app.store.update_with_backup(|tasks| {
                let task = tasks
                    .iter_mut()
                    .find(|task| task.id() == id)
                    .ok_or(AppError::TaskNotFound { no })?;
                task.set_content(content);
                task.set_description(description);
                task.set_deadline(deadline);
                task.set_priority(form.priority);
                Ok(())
            })?;
            format!(">>> Task {} changed", no)
        }
    };
    // 写入成功后立即离开表单；即使随后的刷新失败，也不会重复提交 add。
    app.state = AppState::Main;
    app.reload()?;
    app.set_message(message);
    Ok(())
}
