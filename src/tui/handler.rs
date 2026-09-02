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
//! 在主界面按pgup/pgdn翻页查看选中task的详情
//! 在主界面按ctrl+e进入选中task的change表单
//! 在主界面按r刷新列表
//! 按q或ctrl+c退出tui

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    error::AppError,
    time::parse_deadline_input,
    todo::{
        SortBy, add_task, delete_alldone_apply, delete_alldone_preview, undo_task_apply,
        undo_task_preview,
    },
    tui::{
        app::{App, AppState, ConfirmAction, TaskAction},
        form_state::{FormData, FormField, FormMode},
        text::InputLine,
        views::{
            settings_options::SettingMenu,
            task_options::{MultiOpMenu, TaskOpMenu},
        },
    },
};

/// 判断按键是否带有ctrl/alt组合
///
/// 单字符快捷键(q/k/j/r、排序模式的p/d/n、确认弹窗的y/n等)使用它过滤,
/// 防止alt+q误退出、排序模式下ctrl+p误触发排序等组合键误触。
fn has_ctrl_or_alt(key: &KeyEvent) -> bool {
    key.modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
}

/// 光标上下移动
///
/// 方向键(↑/↓)与vim风格(j/k)的统一抽象, 列表与菜单共用
/// 这样所有上下移动的地方都可以用j/k而不用反复声明
enum Move {
    Back,
    Next,
}

/// 列表移动按键绑定方法
///
/// 可以使用 ↑|↓ 键和 'j' 'k' 移动光标
fn catch_key_for_list_move(key: &KeyEvent) -> Option<Move> {
    match key.code {
        KeyCode::Up => Some(Move::Back),
        KeyCode::Down => Some(Move::Next),
        // 不允许 ctrl / alt 混合输入
        KeyCode::Char(c) if !has_ctrl_or_alt(key) => match c.to_ascii_lowercase() {
            'k' => Some(Move::Back),
            'j' => Some(Move::Next),
            _ => None,
        },
        _ => None,
    }
}

/// task菜单中光标移动方法
///
/// 可以使用 ↑|↓ 键和 'j' 'k' 移动光标
fn task_move(app: &mut App, key: &KeyEvent) -> bool {
    let Some(m) = catch_key_for_list_move(key) else {
        return false;
    };
    match m {
        Move::Back => app.select_back(),
        Move::Next => app.select_next(),
    }
    true
}

/// 菜单中光标移动方法
///
/// 可以使用 ↑|↓ 键和 'j' 'k' 移动光标
/// 长度由调用方直接输入，而不每次都取一次 len()
fn menu_move(app: &mut App, key: &KeyEvent, len: usize) -> bool {
    let Some(m) = catch_key_for_list_move(key) else {
        return false;
    };
    match m {
        Move::Back => app.menu_back(len),
        Move::Next => app.menu_next(len),
    }
    true
}

/// 详情面板翻页按键(pgup上翻/pgdn下翻),步长为渲染时测得的可见行数
///
/// 规则: 详情面板完整可见的状态(Main/MultiSelect/SortMode)统一响应翻页;
/// 弹窗遮挡面板的状态(TaskOptions/Settings/Confirm)与输入状态(SearchInput/Form)不响应
fn detail_scroll(app: &mut App, key: &KeyEvent) -> bool {
    match key.code {
        KeyCode::PageUp => {
            app.details_scroll = app.details_scroll.saturating_sub(app.details_page.max(1));
        }
        KeyCode::PageDown => {
            app.details_scroll = app.details_scroll.saturating_add(app.details_page.max(1));
        }
        _ => return false,
    }
    true
}

/// 使用稳定 ID 在一次文件锁内定位并修改任务，避免长驻 TUI 的缓存序号失效。
fn apply_to_tasks(
    app: &App,
    task_refs: &[(usize, usize)], // (显示序号, 稳定 ID)
    action: TaskAction,
) -> Result<(), AppError> {
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
            TaskAction::Delete => {
                for position in positions.into_iter().rev() {
                    tasks.remove(position);
                }
            }
        }
        Ok(())
    })
}

/// 字段级三方判定: 按快照(snapshot)/磁盘(disk)/表单值(form_view)决定单字段是否需要写入
///
/// - 表单值与快照一致(用户未改该字段): 返回false,保留磁盘当前值,防止覆盖外部修改
/// - 用户已改且磁盘未变(或双方改成了相同结果): 返回true
/// - 双方都改了同一字段且结果不同: EditConflict
fn conflict_check<T: PartialEq>(snapshot: T, disk: T, form_view: T) -> Result<bool, AppError> {
    if form_view == snapshot {
        return Ok(false);
    }
    if disk == snapshot || disk == form_view {
        Ok(true)
    } else {
        Err(AppError::EditConflict)
    }
}

/// 打开change表单: 先同步列表再按稳定ID预填,避免用陈旧缓存
///
/// 一次reload同时刷新背景列表/状态统计/序号列,表单与列表同源;
/// ID在reload前从选中项捕获,外部进程重排时表单仍作用于用户选中的那个任务;
/// 任务已被并发删除时在打开阶段即报TaskNotFound,而不是等到保存时才失败
fn open_change_form(app: &mut App, cached_no: usize, id: usize) -> Result<(), AppError> {
    app.reload()?;
    let Some((no, task)) = app.tasks().iter().find(|(_, t)| t.id() == id) else {
        return Err(AppError::TaskNotFound { no: cached_no });
    };
    app.state = AppState::Form(FormData::change(*no, task));
    Ok(())
}

/// 打开add表单: 先同步列表再进入表单
///
/// add表单不引用已有任务,同步只为背景列表/统计/计数标题与磁盘一致
fn open_add_form(app: &mut App) -> Result<(), AppError> {
    app.reload()?;
    app.state = AppState::Form(FormData::add());
    Ok(())
}

/// 键盘事件分发入口
///
/// 所有业务错误不在此处抛出,而是转化为底部提示信息
pub fn handle_key(app: &mut App, key: KeyEvent) {
    // ctrl+c在任何状态下退出
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.quit();
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

/// 切换单个任务的完成状态
///
/// 以磁盘真值为翻转基准: 缓存可能落后一个同步窗口,按缓存决定目标会把无效操作谎报为已切换;
/// 提示消息按实际翻转结果生成。菜单首项标签仍由缓存派生,标签与结果最多差一个同步窗口
fn toggle_status(app: &mut App, no: usize, id: usize) -> Result<(), AppError> {
    let mut was_done = false;
    app.store.update_with_backup(|tasks| {
        let task = tasks
            .iter_mut()
            .find(|task| task.id() == id)
            .ok_or(AppError::TaskNotFound { no })?;
        was_done = task.is_complete();
        if was_done {
            task.incomplete();
        } else {
            task.complete();
        }
        Ok(())
    })?;
    app.set_message(if was_done {
        format!("~_ Task {} undone", no)
    } else {
        format!("^_ Task {} done", no)
    });
    app.reload()
}

/// 主界面
fn handle_main(app: &mut App, key: KeyEvent) -> Result<(), AppError> {
    // 任意非ctrl+d按键解除二次确认, 并清除残留的footer警告,
    // 保持"警告是否显示"与"ctrl+d是否处于挂起状态"严格一致
    if app.pending_delete.is_some()
        && !(key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('d'))
    {
        app.pending_delete = None;
        app.clear_message();
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('a') => open_add_form(app)?,
            KeyCode::Char('d') => {
                let Some((no, task)) = app.selected_task().cloned() else {
                    app.state = AppState::Main;
                    return Ok(());
                };
                match app.pending_delete {
                    Some(id) if id == task.id() => {
                        apply_to_tasks(app, &[(no, task.id())], TaskAction::Delete)?;
                        app.set_message(format!("#_# Task {} deleted", no));
                        app.pending_delete = None;
                        app.reload()?;
                        app.state = AppState::Main;
                    }
                    _ => {
                        app.pending_delete = Some(task.id());
                        app.set_notice(format!("Press Ctrl+D again to delete task #{}", no));
                    }
                }
            }
            KeyCode::Char('e') => {
                let Some((no, task)) = app.selected_task().cloned() else {
                    app.state = AppState::Main;
                    return Ok(());
                };
                open_change_form(app, no, task.id())?;
            }
            KeyCode::Char('p') => {
                app.state = AppState::Settings;
                app.menu_index = 0;
                app.search_line.clear();
            }
            KeyCode::Char('f') => {
                app.search_line = InputLine::new(app.find.clone().unwrap_or_default());
                app.state = AppState::SearchInput;
            }
            KeyCode::Char('l') => app.state = AppState::SortMode,
            _ => {}
        }
        return Ok(());
    }
    if task_move(app, &key) {
        return Ok(());
    }
    if detail_scroll(app, &key) {
        return Ok(());
    }
    match key.code {
        KeyCode::Enter => {
            if app.selected_task().is_some() {
                app.state = AppState::TaskOptions;
                app.menu_index = 0;
            }
        }
        // 主界面按esc清除搜索过滤
        KeyCode::Esc if app.find.is_some() => {
            app.find = None;
            app.reload()?;
        }
        // 单字符快捷键: 不允许与ctrl/alt组合(如alt+q不应退出), 大小写不敏感
        KeyCode::Char(c) if !has_ctrl_or_alt(&key) => match c.to_ascii_lowercase() {
            'q' => app.quit(),
            'r' => {
                app.reload()?;
                app.set_message(">>> Refreshed");
            }
            // 空格切换done/undone
            ' ' => {
                let Some((no, task)) = app.selected_task().cloned() else {
                    return Ok(());
                };
                toggle_status(app, no, task.id())?;
            }
            _ => {}
        },
        _ => {}
    }
    Ok(())
}

/// enter进入的选中task选项菜单: done|undone / change / delete
fn handle_task_options(app: &mut App, key: KeyEvent) -> Result<(), AppError> {
    if menu_move(app, &key, TaskOpMenu::ALL.len()) {
        return Ok(());
    }
    match key.code {
        KeyCode::Esc => app.state = AppState::Main,
        KeyCode::Enter => {
            let Some((no, task)) = app.selected_task().cloned() else {
                app.state = AppState::Main;
                return Ok(());
            };
            let Some(action) = TaskOpMenu::ALL.get(app.menu_index) else {
                return Ok(());
            };
            match action {
                // done | undone
                TaskOpMenu::StatusChange => {
                    toggle_status(app, no, task.id())?;
                    app.state = AppState::Main;
                }
                // change
                TaskOpMenu::Change => {
                    open_change_form(app, no, task.id())?;
                }
                // delete
                TaskOpMenu::Delete => {
                    apply_to_tasks(app, &[(no, task.id())], TaskAction::Delete)?;
                    app.set_message(format!("#_# Task {} deleted", no));
                    app.reload()?;
                    app.state = AppState::Main;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// ctrl+p命令面板： search / add / multiple choices / delete all done / exit
fn handle_settings(app: &mut App, key: KeyEvent) -> Result<(), AppError> {
    if menu_move(app, &key, SettingMenu::ALL.len()) {
        return Ok(());
    }
    match key.code {
        KeyCode::Esc => app.state = AppState::Main,
        KeyCode::Enter => {
            let Some(action) = SettingMenu::ALL.get(app.menu_index) else {
                return Ok(());
            };
            match action {
                SettingMenu::Search => {
                    app.search_line = InputLine::new(app.find.clone().unwrap_or_default());
                    app.state = AppState::SearchInput;
                }
                SettingMenu::Add => open_add_form(app)?,
                SettingMenu::MultipleChoices => {
                    app.multi_selected.clear();
                    app.multi_menu_open = false;
                    app.state = AppState::MultiSelect;
                    app.set_message("space to select, enter to confirm, esc to cancel");
                }
                SettingMenu::DeleteAllDone => {
                    let tasks = delete_alldone_preview(app.store)?;
                    app.state = AppState::Confirm(ConfirmAction::DeleteAll(tasks));
                }
                SettingMenu::Undo => {
                    let tasks = undo_task_preview(app.store)?;
                    app.state = AppState::Confirm(ConfirmAction::Undo(tasks));
                }
                SettingMenu::Exit => app.quit(),
            }
        }
        _ => {}
    }
    Ok(())
}

/// 单行输入框的通用编辑按键
///
///  ←/→/home/end 移动光标, backspace 删除, 可打印字符插入。
fn edit_line(key: KeyEvent, input: &mut InputLine) {
    match key.code {
        KeyCode::Left => input.left(),
        KeyCode::Right => input.right(),
        KeyCode::Home => input.home(),
        KeyCode::End => input.end(),
        KeyCode::Backspace => input.backspace(),
        KeyCode::Char(c) if !has_ctrl_or_alt(&key) => input.insert(c),
        _ => {}
    }
}

/// 命令面板内嵌搜索输入框
fn handle_search(app: &mut App, key: KeyEvent) -> Result<(), AppError> {
    match key.code {
        KeyCode::Esc => app.state = AppState::Main,
        KeyCode::Enter => {
            let keyword = app.search_line.value().trim();
            app.find = if keyword.is_empty() {
                None
            } else {
                Some(keyword.to_string())
            };
            app.reload()?;
            app.state = AppState::Main;
            match &app.find {
                Some(kw) => {
                    app.set_message(format!("Found {} task(s) for '{}'", app.tasks().len(), kw))
                }
                None => app.set_message("Search cleared"),
            }
        }
        _ => {
            edit_line(key, &mut app.search_line);
        }
    }
    Ok(())
}

/// 多选模式: 上下移动,space选中,enter确认弹出done/undone/delete菜单
fn handle_multi(app: &mut App, key: KeyEvent) -> Result<(), AppError> {
    if app.multi_menu_open {
        return handle_multi_menu(app, key);
    }
    if task_move(app, &key) {
        return Ok(());
    }
    if detail_scroll(app, &key) {
        return Ok(());
    }
    match key.code {
        KeyCode::Esc => {
            app.multi_selected.clear();
            app.state = AppState::Main;
        }
        // 与其余单字符快捷键同规: 不允许ctrl/alt组合, 防alt+space误切换勾选
        KeyCode::Char(' ') if !has_ctrl_or_alt(&key) => {
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
    if menu_move(app, &key, MultiOpMenu::ALL.len()) {
        return Ok(());
    }
    match key.code {
        KeyCode::Esc => app.multi_menu_open = false,
        KeyCode::Enter => {
            let task_refs: Vec<(usize, usize)> = app
                .tasks()
                .iter()
                .filter(|(_, task)| app.multi_selected.contains(&task.id()))
                .map(|(no, task)| (*no, task.id()))
                .collect();
            let count = task_refs.len();
            // 动作与标签同样由菜单项枚举派生
            let Some(op) = MultiOpMenu::ALL.get(app.menu_index) else {
                return Ok(());
            };
            apply_to_tasks(app, &task_refs, op.action())?;
            app.set_message(format!(">>> {} task(s) {}", count, op.label()));
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
        // 不允许ctrl/alt + y或者n的组合: ctrl+y不应意外确认删除; 大小写不敏感
        KeyCode::Char(c) if !has_ctrl_or_alt(&key) => match c.to_ascii_lowercase() {
            'y' => {
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
            'n' => {
                app.state = AppState::Settings;
                app.set_message(">_< Operation cancelled.");
            }
            _ => {}
        },
        KeyCode::Esc => {
            app.state = AppState::Settings;
            app.set_message(">_< Operation cancelled.");
        }
        _ => {}
    }
    Ok(())
}

/// ctrl+l排序模式: p按优先级排序, d按截止日期排序
fn handle_sort(app: &mut App, key: KeyEvent) -> Result<(), AppError> {
    if detail_scroll(app, &key) {
        return Ok(());
    }
    match key.code {
        KeyCode::Esc => app.state = AppState::Main,
        // 不允许ctrl/alt的按键组合，只允许选择排序方式或者esc退出;
        // 大小写不敏感
        KeyCode::Char(c) if !has_ctrl_or_alt(&key) => match c.to_ascii_lowercase() {
            'p' => {
                app.sort = Some(SortBy::Priority);
                app.reload()?;
                app.state = AppState::Main;
                app.set_message(">>> Sorted by priority");
            }
            'd' => {
                app.sort = Some(SortBy::Deadline);
                app.reload()?;
                app.state = AppState::Main;
                app.set_message(">>> Sorted by deadline");
            }
            'n' => {
                app.sort = None;
                app.reload()?;
                app.state = AppState::Main;
                app.set_message(">>> Sorted by default");
            }
            _ => {}
        },
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
        _ => {}
    }

    match form.focus {
        // 单行输入content和deadline套用一套逻辑
        FormField::Content | FormField::Deadline => {
            if let Some(input) = form.single_line_mut() {
                edit_line(key, input);
            }
        }
        FormField::Description => {
            let desc = form.description_mut();
            match key.code {
                // 方向键只用于移动光标: description栏在多行文本中移动(自动滚动/软换行)
                KeyCode::Enter => desc.insert('\n'),
                KeyCode::Up => desc.up(),
                KeyCode::Down => desc.down(),
                KeyCode::Left => desc.left(),
                KeyCode::Right => desc.right(),
                KeyCode::Backspace => desc.backspace(),
                KeyCode::Char(c) if !has_ctrl_or_alt(&key) => desc.insert(c),
                _ => {}
            }
        }
        // priority栏使用左右键循环切换优先级
        FormField::Priority => match key.code {
            KeyCode::Right => form.cycle_priority(true),
            KeyCode::Left => form.cycle_priority(false),
            _ => {}
        },
    }
    Ok(())
}

/// ctrl+s保存表单
///
/// 输入非法或写入失败时保留表单，成功后保存并返回主界面。
fn save_form(app: &mut App) -> Result<(), AppError> {
    let Some(form) = app.form().cloned() else {
        return Ok(());
    };
    let content = form.content().trim().to_string();
    if content.is_empty() {
        app.set_message(":( Invalid content: content cannot be left blank.");
        return Ok(());
    }
    let description = match form.description().value().trim() {
        "" => None,
        desc => Some(desc.to_string()),
    };
    let deadline = match form.deadline().trim() {
        "" => None,
        input => match parse_deadline_input(input) {
            Ok(d) => Some(d),
            Err(err) => {
                app.set_message(format!(":( {}", err.pack_to_tui_err()));
                return Ok(());
            }
        },
    };
    // 合并快路径：
    //
    // 用户未改任何字段 ⟺ 合并零写入(conflict_check只在form_view==snapshot时判为不写)，
    // 跳过写盘,仅刷新视图让外部修改可见,并如实提示未发生修改。
    // 注意：字段集须与下方 conflict_check 调用保持一致。
    if let (FormMode::Change { .. }, Some(orig)) = (form.mode(), form.original())
        && content == orig.content()
        && description.as_deref() == orig.description()
        && deadline == orig.deadline()
        && form.priority() == orig.priority()
    {
        app.state = AppState::Main;
        app.reload()?;
        app.set_message(">>> No changes");
        return Ok(());
    }
    // 正常合并路径
    let message = match form.mode() {
        FormMode::Add => {
            add_task(
                app.store,
                content,
                description,
                deadline,
                Some(form.priority()),
            )?;
            ">>> Task added".to_string()
        }
        FormMode::Change { no, id } => {
            // Change模式构造时必带原始快照,此分支仅为类型完备
            let Some(snapshot) = form.original() else {
                unreachable!(":( Error: Change mode must have original snapshot.");
            };
            // 字段级乐观合并: 逐字段按快照/磁盘当前值/表单值决策,再统一应用
            // - 用户未改的字段保留磁盘值(外部对未改字段的修改不会被覆盖)
            // - 仅双方都改了同一字段且结果不同才报EditConflict
            // - completed不属于表单字段,外部切换完成状态不会冲突也不会被覆盖
            // 闭包内的修改只是内存暂存: 任一字段冲突时闭包返回Err,
            // update在覆写文件前中止,已决策字段的写入随之丢弃,磁盘零改动
            app.store.update_with_backup(|tasks| {
                let idx = tasks
                    .iter()
                    .position(|task| task.id() == id)
                    .ok_or(AppError::TaskNotFound { no })?;
                // 决策与写分两阶段: 先只读比较定出要写的字段,再统一可变应用,
                // 借用互不重叠,无需堆分配来绕开同时借用
                let (write_content, write_desc, write_deadline, write_priority) = {
                    let task = &tasks[idx];
                    (
                        conflict_check(snapshot.content(), task.content(), content.as_str())?,
                        conflict_check(
                            snapshot.description(),
                            task.description(),
                            description.as_deref(),
                        )?,
                        conflict_check(snapshot.deadline(), task.deadline(), deadline)?,
                        conflict_check(snapshot.priority(), task.priority(), form.priority())?,
                    )
                };
                let task = &mut tasks[idx];
                if write_content {
                    task.set_content(content);
                }
                if write_desc {
                    task.set_description(description);
                }
                if write_deadline {
                    task.set_deadline(deadline);
                }
                if write_priority {
                    task.set_priority(form.priority());
                }
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
