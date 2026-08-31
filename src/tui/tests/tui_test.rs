/// 使用TestBackend对TUI进行无头渲染与按键流测试
#[cfg(test)]
mod tests {
    use crate::io::storage::TaskStore;
    use crate::task::{Priority, Task};
    use crate::tests::test_helpers::TempGuard;
    use crate::todo::{SortBy, add_task, delete_task};
    use crate::tui::app::AppState;
    use crate::tui::app::{self, App};
    use crate::tui::form_state::{FormData, FormField, FormMode};
    use crate::tui::ui;
    use crate::{UserInterfaceTypes, tui::handler};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::backend::TestBackend;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    fn setup_store(guard: &TempGuard) -> TaskStore {
        let store = TaskStore::new(Some(guard.main_path()), UserInterfaceTypes::Tui);
        add_task(
            &store,
            "task1".to_string(),
            Some("desc1".to_string()),
            None,
            Some(Priority::High),
        )
        .unwrap();
        add_task(&store, "task2".to_string(), None, None, None).unwrap();
        store
    }

    fn render(app: &mut App) {
        render_at(app, 120, 35);
    }

    fn render_at(app: &mut App, width: u16, height: u16) {
        let backend = TestBackend::new(width, height);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| ui::draw(frame, app)).unwrap();
    }

    /// 所有界面状态都能无panic渲染
    #[test]
    fn smoke_render_all_states() {
        let guard = TempGuard::new("tui_render");
        let store = setup_store(&guard);
        let mut app = App::new(&store).unwrap();

        render(&mut app);

        app.state = AppState::TaskOptions;
        render(&mut app);

        app.state = AppState::Settings;
        render(&mut app);

        app.state = AppState::SearchInput;
        app.search_input = "task".to_string();
        render(&mut app);

        app.state = AppState::MultiSelect;
        app.multi_selected.insert(1);
        render(&mut app);
        app.multi_menu_open = true;
        render(&mut app);

        app.state = AppState::SortMode;
        render(&mut app);

        let mut form = FormData::add();
        form.content = "new task".to_string();
        form.desc_insert('\n');
        form.desc_insert('x');
        app.state = AppState::Form(form);
        render(&mut app);

        app.state = AppState::Confirm(app::ConfirmAction::DeleteAll(Vec::<Task>::new()));
        render(&mut app);

        app.state = AppState::Confirm(app::ConfirmAction::Undo(vec![Task::new(
            1,
            "t".to_string(),
            None,
            None,
            Priority::Low,
        )]));
        render(&mut app);
    }

    /// 模拟按键流: 移动/done切换/搜索/排序/多选删除/退出
    #[test]
    fn smoke_handler_flow() {
        let guard = TempGuard::new("tui_handler");
        let store = setup_store(&guard);
        let mut app = App::new(&store).unwrap();

        // 主界面移动光标
        handler::handle_key(&mut app, key(KeyCode::Down));
        assert_eq!(app.list_state.selected(), Some(1));
        handler::handle_key(&mut app, key(KeyCode::Down));
        assert_eq!(app.list_state.selected(), Some(0)); // 循环回顶部

        // enter进入选项菜单,选择done
        handler::handle_key(&mut app, key(KeyCode::Enter));
        assert!(matches!(app.state, AppState::TaskOptions));
        handler::handle_key(&mut app, key(KeyCode::Enter));
        assert!(matches!(app.state, AppState::Main));
        assert!(store.load().unwrap()[0].is_complete());

        // ctrl+l后按p按优先级排序
        handler::handle_key(&mut app, ctrl('l'));
        assert!(matches!(app.state, AppState::SortMode));
        handler::handle_key(&mut app, key(KeyCode::Char('p')));
        assert!(matches!(app.state, AppState::Main));
        assert!(matches!(app.sort, Some(SortBy::Priority)));

        // ctrl+p进入命令面板,选择search,输入关键词过滤
        handler::handle_key(&mut app, ctrl('p'));
        assert!(matches!(app.state, AppState::Settings));
        handler::handle_key(&mut app, key(KeyCode::Enter)); // search
        assert!(matches!(app.state, AppState::SearchInput));
        for c in "task2".chars() {
            handler::handle_key(&mut app, key(KeyCode::Char(c)));
        }
        handler::handle_key(&mut app, key(KeyCode::Enter));
        assert_eq!(app.tasks.len(), 1);
        assert_eq!(app.tasks[0].1.content(), "task2");

        // esc清除搜索过滤
        handler::handle_key(&mut app, key(KeyCode::Esc));
        assert_eq!(app.tasks.len(), 2);

        // ctrl+p -> multiple choices -> space选中 -> enter -> delete
        handler::handle_key(&mut app, ctrl('p'));
        handler::handle_key(&mut app, key(KeyCode::Down)); // add
        handler::handle_key(&mut app, key(KeyCode::Down)); // multiple choices
        handler::handle_key(&mut app, key(KeyCode::Enter));
        assert!(matches!(app.state, AppState::MultiSelect));
        handler::handle_key(&mut app, key(KeyCode::Char(' ')));
        assert!(app.multi_selected.contains(&1));
        handler::handle_key(&mut app, key(KeyCode::Enter));
        assert!(app.multi_menu_open);
        handler::handle_key(&mut app, key(KeyCode::Down)); // undone
        handler::handle_key(&mut app, key(KeyCode::Down)); // delete
        handler::handle_key(&mut app, key(KeyCode::Enter));
        assert_eq!(store.load().unwrap().len(), 1);
        assert_eq!(store.load().unwrap()[0].content(), "task2");

        // q退出
        handler::handle_key(&mut app, key(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    /// 主界面space切换done/undone,ctrl+e进入change表单并预填内容
    #[test]
    fn smoke_space_toggle_and_change() {
        let guard = TempGuard::new("tui_space_change");
        let store = setup_store(&guard);
        let mut app = App::new(&store).unwrap();

        // space切换done
        handler::handle_key(&mut app, key(KeyCode::Char(' ')));
        assert!(store.load().unwrap()[0].is_complete());
        assert!(matches!(app.state, AppState::Main));
        // 再按space切换回undone
        handler::handle_key(&mut app, key(KeyCode::Char(' ')));
        assert!(!store.load().unwrap()[0].is_complete());

        // ctrl+e进入change表单,预填content/description
        handler::handle_key(&mut app, ctrl('e'));
        assert!(matches!(
            app.form().map(|form| form.mode),
            Some(FormMode::Change { no: 1, .. })
        ));
        let form = app.form().unwrap();
        assert_eq!(form.content, "task1");
        assert_eq!(form.description, "desc1");

        // esc取消返回主界面
        handler::handle_key(&mut app, key(KeyCode::Esc));
        assert!(matches!(app.state, AppState::Main));
        assert!(app.form().is_none());
    }

    /// 详情面板pgup/pgdn翻页查看长description,切换选中任务后回到顶部
    #[test]
    fn details_panel_pgup_pgdn_scroll() {
        let guard = TempGuard::new("tui_details_scroll");
        let store = TaskStore::new(Some(guard.main_path()), UserInterfaceTypes::Tui);
        let long_desc = (0..40)
            .map(|i| format!("description line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        add_task(&store, "long".to_string(), Some(long_desc), None, None).unwrap();
        add_task(&store, "task2".to_string(), None, None, None).unwrap();
        let mut app = App::new(&store).unwrap();

        render(&mut app); // 120x35: 详情面板可见25行
        assert_eq!(app.details_page, 25);

        // pgdn向下翻页,渲染时夹紧到最大偏移(45行内容 - 25行可见)
        handler::handle_key(&mut app, key(KeyCode::PageDown));
        render(&mut app);
        assert_eq!(app.details_scroll, 20);

        // pgup回到顶部
        handler::handle_key(&mut app, key(KeyCode::PageUp));
        assert_eq!(app.details_scroll, 0);

        // 翻页后切换选中任务,滚动回到顶部
        handler::handle_key(&mut app, key(KeyCode::PageDown));
        render(&mut app);
        assert!(app.details_scroll > 0);
        handler::handle_key(&mut app, key(KeyCode::Down));
        assert_eq!(app.details_scroll, 0);

        // 短description任务翻页不产生滚动
        handler::handle_key(&mut app, key(KeyCode::PageDown));
        render(&mut app);
        assert_eq!(app.details_scroll, 0);
    }

    /// description多行编辑: enter换行/自动滚动/上下移动光标/退格合并行
    #[test]
    fn test_description_multiline_edit() {
        let mut form = FormData::add();
        for c in "line1\nline2\nline3\nline4".chars() {
            form.desc_insert(c);
        }
        // 光标在第4行末尾,可见窗口向上滚动保持3行
        assert_eq!(form.desc_cursor_row_col(), (3, 5));
        assert_eq!(form.desc_scroll(), 1);

        // 上移光标回到第一行,窗口随之滚回顶部
        form.desc_up();
        form.desc_up();
        form.desc_up();
        assert_eq!(form.desc_cursor_row_col(), (0, 5));
        assert_eq!(form.desc_scroll(), 0);
        // 顶部继续上移保持不变
        form.desc_up();
        assert_eq!(form.desc_cursor_row_col(), (0, 5));

        // 在第一行中间插入字符,编辑上面的文字
        form.desc_backspace();
        assert!(form.description.starts_with("line\n"));

        // 行首退格与上一行合并
        let mut form = FormData::add();
        for c in "ab".chars() {
            form.desc_insert(c);
        }
        form.desc_insert('\n');
        assert_eq!(form.desc_cursor_row_col(), (1, 0));
        form.desc_backspace();
        assert_eq!(form.description, "ab");
        assert_eq!(form.desc_cursor_row_col(), (0, 2));

        // 移动到较短行时列位置截断
        let mut form = FormData::add();
        for c in "abcd\nx".chars() {
            form.desc_insert(c);
        }
        form.desc_up();
        assert_eq!(form.desc_cursor_row_col(), (0, 1));
        // 插入字符使列变长后下移,列截断到短行长度
        for c in "XYZ".chars() {
            form.desc_insert(c);
        }
        form.desc_down();
        assert_eq!(form.desc_cursor_row_col(), (1, 1));

        // change表单预填多行description,光标在末尾且窗口滚到底部
        let task = Task::new(
            1,
            "t".to_string(),
            Some("a\nb\nc\nd\ne".to_string()),
            None,
            Priority::Low,
        );
        let form = FormData::change(1, &task);
        assert_eq!(form.desc_cursor_row_col(), (4, 1));
        assert_eq!(form.desc_scroll(), 2);
    }

    /// description软换行: 超宽自动换行显示但不写入\n,上下键在显示行间移动
    #[test]
    fn test_description_soft_wrap() {
        let mut form = FormData::add();
        form.set_desc_wrap_width(5);
        for c in "abcdefgh".chars() {
            form.desc_insert(c);
        }
        // 8字符按宽度5软换行为两个显示行,文本中没有\n
        assert_eq!(form.description, "abcdefgh");
        assert_eq!(form.desc_rows(), vec![(0, 5), (5, 8)]);
        assert_eq!(form.desc_cursor_row_col(), (1, 3));

        // 上下键在软换行出的显示行间移动
        form.desc_up();
        assert_eq!(form.desc_cursor_row_col(), (0, 3));
        form.desc_down();
        assert_eq!(form.desc_cursor_row_col(), (1, 3));

        // 显式换行与软换行共存
        form.desc_insert('\n');
        form.desc_insert('x');
        assert_eq!(form.description, "abcdefgh\nx");
        assert_eq!(form.desc_rows(), vec![(0, 5), (5, 8), (9, 10)]);
        assert_eq!(form.desc_cursor_row_col(), (2, 1));

        // 左右键跨过软换行边界
        form.desc_left();
        form.desc_left();
        assert_eq!(form.desc_cursor_row_col(), (1, 3));
        form.desc_left();
        assert_eq!(form.desc_cursor_row_col(), (1, 2));
    }

    /// 表单内方向键只移动光标不切换栏位,tab循环切换栏位
    #[test]
    fn test_form_cursor_keys() {
        let guard = TempGuard::new("tui_form_cursor");
        let store = setup_store(&guard);
        let mut app = App::new(&store).unwrap();

        // ctrl+p -> add -> enter 打开add表单
        handler::handle_key(&mut app, ctrl('p'));
        handler::handle_key(&mut app, key(KeyCode::Down));
        handler::handle_key(&mut app, key(KeyCode::Enter));
        assert!(matches!(
            app.form().map(|form| form.mode),
            Some(FormMode::Add)
        ));

        // content: 输入"abc",左移一位插入"X"
        for c in "abc".chars() {
            handler::handle_key(&mut app, key(KeyCode::Char(c)));
        }
        handler::handle_key(&mut app, key(KeyCode::Left));
        handler::handle_key(&mut app, key(KeyCode::Char('X')));
        assert_eq!(app.form().unwrap().content, "abXc");
        // 上下键不再切换栏位
        handler::handle_key(&mut app, key(KeyCode::Up));
        handler::handle_key(&mut app, key(KeyCode::Down));
        assert_eq!(app.form().unwrap().focus, FormField::Content);

        // tab切换到description,输入两行,左右键跨行移动光标
        handler::handle_key(&mut app, key(KeyCode::Tab));
        assert_eq!(app.form().unwrap().focus, FormField::Description);
        for c in "ab".chars() {
            handler::handle_key(&mut app, key(KeyCode::Char(c)));
        }
        handler::handle_key(&mut app, key(KeyCode::Enter)); // 换行
        for c in "cd".chars() {
            handler::handle_key(&mut app, key(KeyCode::Char(c)));
        }
        assert_eq!(app.form().unwrap().description, "ab\ncd");
        assert_eq!(app.form().unwrap().desc_cursor_row_col(), (1, 2));
        handler::handle_key(&mut app, key(KeyCode::Left)); // (1,1)
        handler::handle_key(&mut app, key(KeyCode::Left)); // (1,0)
        handler::handle_key(&mut app, key(KeyCode::Left)); // 跨行到(0,2)
        assert_eq!(app.form().unwrap().desc_cursor_row_col(), (0, 2));
        handler::handle_key(&mut app, key(KeyCode::Right)); // 回到(1,0)
        assert_eq!(app.form().unwrap().desc_cursor_row_col(), (1, 0));

        // tab循环: deadline -> priority -> content
        handler::handle_key(&mut app, key(KeyCode::Tab));
        assert_eq!(app.form().unwrap().focus, FormField::Deadline);
        handler::handle_key(&mut app, key(KeyCode::Tab));
        assert_eq!(app.form().unwrap().focus, FormField::Priority);
        handler::handle_key(&mut app, key(KeyCode::Tab));
        assert_eq!(app.form().unwrap().focus, FormField::Content);

        // priority栏左右键切换优先级
        handler::handle_key(&mut app, key(KeyCode::BackTab));
        assert_eq!(app.form().unwrap().focus, FormField::Priority);
        handler::handle_key(&mut app, key(KeyCode::Right));
        assert_eq!(app.form().unwrap().priority, Priority::Medium);
    }

    /// undo恢复 / 排序n恢复默认 / 搜索框左右移动光标
    #[test]
    fn smoke_undo_sort_search_cursor() {
        let guard = TempGuard::new("tui_undo");
        let store = setup_store(&guard);
        let mut app = App::new(&store).unwrap();

        // 删除task1后,通过undo二次确认恢复
        delete_task(vec![1], &store).unwrap();
        handler::handle_key(&mut app, ctrl('p'));
        handler::handle_key(&mut app, key(KeyCode::Down)); // add
        handler::handle_key(&mut app, key(KeyCode::Down)); // multiple choices
        handler::handle_key(&mut app, key(KeyCode::Down)); // delete all done
        handler::handle_key(&mut app, key(KeyCode::Down)); // undo
        handler::handle_key(&mut app, key(KeyCode::Enter));
        assert!(matches!(
            app.state,
            AppState::Confirm(app::ConfirmAction::Undo(_))
        ));
        handler::handle_key(&mut app, key(KeyCode::Char('y')));
        assert_eq!(store.load().unwrap().len(), 2);
        assert!(matches!(app.state, AppState::Main));
        assert!(!matches!(app.state, AppState::Confirm(_)));

        // 排序: p按优先级后,n恢复默认
        handler::handle_key(&mut app, ctrl('l'));
        handler::handle_key(&mut app, key(KeyCode::Char('p')));
        assert!(matches!(app.sort, Some(SortBy::Priority)));
        handler::handle_key(&mut app, ctrl('l'));
        handler::handle_key(&mut app, key(KeyCode::Char('n')));
        assert!(app.sort.is_none());
        assert!(matches!(app.state, AppState::Main));

        // 搜索框: 输入后左移光标插入字符
        handler::handle_key(&mut app, ctrl('p'));
        handler::handle_key(&mut app, key(KeyCode::Enter)); // search
        for c in "task".chars() {
            handler::handle_key(&mut app, key(KeyCode::Char(c)));
        }
        handler::handle_key(&mut app, key(KeyCode::Left));
        handler::handle_key(&mut app, key(KeyCode::Left));
        handler::handle_key(&mut app, key(KeyCode::Char('X')));
        assert_eq!(app.search_input, "taXsk");
        assert_eq!(app.search_cursor, 3);

        // ctrl+c退出
        handler::handle_key(&mut app, ctrl('c'));
        assert!(app.should_quit);
    }

    #[test]
    fn stale_display_number_still_updates_the_same_task() {
        let guard = TempGuard::new("tui_stable_id");
        let store = setup_store(&guard);
        let mut app = App::new(&store).unwrap();

        handler::handle_key(&mut app, key(KeyCode::Down)); // 缓存中的 #2 task2
        delete_task(vec![1], &store).unwrap(); // 另一个进程使 task2 变成 #1
        handler::handle_key(&mut app, key(KeyCode::Char(' ')));

        let tasks = store.load().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].content(), "task2");
        assert!(tasks[0].is_complete());
    }

    #[test]
    fn failed_change_keeps_form_and_message() {
        let guard = TempGuard::new("tui_form_error");
        let store = setup_store(&guard);
        let mut app = App::new(&store).unwrap();

        handler::handle_key(&mut app, ctrl('e'));
        delete_task(vec![1], &store).unwrap();
        handler::handle_key(&mut app, ctrl('s'));

        assert!(app.form().is_some());
        assert!(
            app.message
                .as_deref()
                .is_some_and(|text| text.contains("Task not found"))
        );
        render(&mut app); // 错误状态仍可正常渲染表单
    }

    #[test]
    fn unicode_editing_and_input_window_use_terminal_width() {
        let mut text = "e\u{301}".to_string();
        let mut cursor = text.chars().count();
        crate::tui::text::backspace_at_cursor(&mut text, &mut cursor);
        assert!(text.is_empty()); // 一次删除整个组合字素
        assert_eq!(cursor, 0);

        assert_eq!(ui::display_width("e\u{301}"), 1);
        assert_eq!(ui::display_width("👩‍💻"), 2);
        assert_eq!(ui::input_window("abcdefgh", 8, 4), ("fgh".to_string(), 3));

        let mut form = FormData::add();
        form.set_desc_wrap_width(2);
        for c in "e\u{301}中".chars() {
            form.desc_insert(c);
        }
        assert_eq!(form.desc_rows(), vec![(0, 2), (2, 3)]);
    }

    #[test]
    fn overdue_status_uses_the_requested_time() {
        let guard = TempGuard::new("tui_live_status");
        let store = TaskStore::new(Some(guard.main_path()), UserInterfaceTypes::Tui);
        let deadline = chrono::Utc::now() + chrono::Duration::hours(1);
        add_task(&store, "deadline".to_string(), None, Some(deadline), None).unwrap();
        let app = App::new(&store).unwrap();

        assert_eq!(
            app.status_counts(deadline - chrono::Duration::seconds(1)).3,
            0
        );
        assert_eq!(
            app.status_counts(deadline + chrono::Duration::seconds(1)).3,
            1
        );
    }

    #[test]
    fn narrow_terminal_renders_fallback_without_panicking() {
        let guard = TempGuard::new("tui_narrow");
        let store = setup_store(&guard);
        let mut app = App::new(&store).unwrap();
        render_at(&mut app, 20, 8);
        app.state = AppState::Form(FormData::add());
        render_at(&mut app, 59, 15);
    }
}
