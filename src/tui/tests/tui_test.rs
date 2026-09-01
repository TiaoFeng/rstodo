//! 使用TestBackend对TUI进行无头渲染与按键流测试
#[cfg(test)]
mod tests {
    use crate::{
        UserInterfaceTypes,
        io::storage::TaskStore,
        task::{Priority, Task},
        tests::test_helpers::*,
        todo::{SortBy, add_task, delete_task},
        tui::{
            app::{self, App, AppState},
            form_state::{FormData, FormField, FormMode},
            handler,
            text::InputLine,
            ui,
            views::task_options::TaskOpMenu,
        },
    };
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

    /// 渲染一帧并取回缓冲区,用于检查实际渲染内容(而非仅状态变量)
    fn render_capture(app: &mut App) -> ratatui::buffer::Buffer {
        let backend = TestBackend::new(120, 35);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| ui::draw(frame, app)).unwrap();
        terminal.backend().buffer().clone()
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
        app.search_line = InputLine::new("task");
        render(&mut app);

        app.state = AppState::MultiSelect;
        app.multi_selected.insert(1);
        render(&mut app);
        app.multi_menu_open = true;
        render(&mut app);

        app.state = AppState::SortMode;
        render(&mut app);

        let mut form = FormData::add();
        if let Some(input) = form.single_line_mut() {
            for c in "new task".chars() {
                input.insert(c);
            }
        }
        form.description_mut().insert('\n');
        form.description_mut().insert('x');
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
        assert_eq!(app.tasks().len(), 1);
        assert_eq!(app.tasks()[0].1.content(), "task2");

        // esc清除搜索过滤
        handler::handle_key(&mut app, key(KeyCode::Esc));
        assert_eq!(app.tasks().len(), 2);

        // ctrl+p -> multiple choices -> space选中 -> enter -> delete
        handler::handle_key(&mut app, ctrl('p'));
        handler::handle_key(&mut app, key(KeyCode::Down)); // add
        handler::handle_key(&mut app, key(KeyCode::Down)); // multiple choices
        handler::handle_key(&mut app, key(KeyCode::Enter));
        assert!(matches!(app.state, AppState::MultiSelect));
        // esc清除过滤后选中跟随任务本身(用户在过滤视图里看的是task2),而非回到原位置
        handler::handle_key(&mut app, key(KeyCode::Char(' ')));
        assert!(app.multi_selected.contains(&2));
        handler::handle_key(&mut app, key(KeyCode::Enter));
        assert!(app.multi_menu_open);
        handler::handle_key(&mut app, key(KeyCode::Down)); // undone
        handler::handle_key(&mut app, key(KeyCode::Down)); // delete
        handler::handle_key(&mut app, key(KeyCode::Enter));
        assert_eq!(store.load().unwrap().len(), 1);
        assert_eq!(store.load().unwrap()[0].content(), "task1");

        // q退出
        handler::handle_key(&mut app, key(KeyCode::Char('q')));
        assert!(app.should_quit());
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
            app.form().map(|form| form.mode()),
            Some(FormMode::Change { no: 1, .. })
        ));
        let form = app.form().unwrap();
        assert_eq!(form.content(), "task1");
        assert_eq!(form.description().value(), "desc1");

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

    /// 含空格的超长英文行: 渲染按词边界换行,行数统计用line_count与渲染一致,
    /// 含空格的超长英文行: 渲染按词边界换行,行数统计用line_count与渲染一致,
    /// 连续pgdn到底后scroll不再增长; 且到达最大偏移的那一帧渲染的是内容而非越界空白
    #[test]
    fn details_scroll_clamps_at_word_wrapped_bottom() {
        let guard = TempGuard::new("tui_details_word_wrap");
        let store = TaskStore::new(Some(guard.main_path()), UserInterfaceTypes::Tui);
        // 单行约1840字符,120宽终端下详情面板约70列,词边界换行后远超一页(25行)
        let long_line = "the quick brown fox jumps over the lazy dog. ".repeat(40);
        add_task(&store, "prose".to_string(), Some(long_line), None, None).unwrap();
        let mut app = App::new(&store).unwrap();
        let _ = render_capture(&mut app);

        // 连续pgdn直到夹紧: 第二次到底后scroll不再增长
        handler::handle_key(&mut app, key(KeyCode::PageDown));
        let buffer = render_capture(&mut app); // 旧缺陷恰好在此帧渲染越界空白
        let first = app.details_scroll;
        handler::handle_key(&mut app, key(KeyCode::PageDown));
        let second_buffer = render_capture(&mut app);
        let second = app.details_scroll;
        assert!(first > 0);
        assert_eq!(first, second, "连续pgdn应停在夹紧的最大偏移");

        // 夹紧后的帧必须真的显示内容: 详情面板内区底行(y=32)与首行(y=8)均非空白
        // (120x35: 列表区下方为详情面板,边框内区x=49..119,y=8..33)
        for (name, buffer) in [("first", &buffer), ("second", &second_buffer)] {
            let bottom_row = &buffer.content[(32 * 120 + 49)..(32 * 120 + 119)];
            let top_row = &buffer.content[(8 * 120 + 49)..(8 * 120 + 119)];
            assert!(
                bottom_row.iter().any(|cell| cell.symbol() != " "),
                "{name}帧夹紧后详情面板底行不应是空白"
            );
            assert!(
                top_row.iter().any(|cell| cell.symbol() != " "),
                "{name}帧夹紧后详情面板首行不应是空白"
            );
        }
    }

    /// 多选模式下详情面板同样可pgup/pgdn翻页,移动光标后滚动归零
    #[test]
    fn multi_select_details_scroll() {
        let guard = TempGuard::new("tui_multi_scroll");
        let store = setup_store(&guard);
        let mut app = App::new(&store).unwrap();

        handler::handle_key(&mut app, ctrl('p'));
        handler::handle_key(&mut app, key(KeyCode::Down)); // add
        handler::handle_key(&mut app, key(KeyCode::Down)); // multiple choices
        handler::handle_key(&mut app, key(KeyCode::Enter));
        assert!(matches!(app.state, AppState::MultiSelect));

        // pgdn翻页: 步长为渲染时测得的页大小
        render(&mut app);
        handler::handle_key(&mut app, key(KeyCode::PageDown));
        assert_eq!(app.details_scroll, app.details_page);

        // 移动光标后滚动归零(与主界面一致)
        handler::handle_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.details_scroll, 0);

        // pgup从顶部不会下溢
        handler::handle_key(&mut app, key(KeyCode::PageUp));
        assert_eq!(app.details_scroll, 0);
    }

    /// description多行编辑: enter换行/自动滚动/上下移动光标/退格合并行
    #[test]
    fn test_description_multiline_edit() {
        let mut form = FormData::add();
        for c in "line1\nline2\nline3\nline4".chars() {
            form.description_mut().insert(c);
        }
        // 光标在第4行末尾,可见窗口向上滚动保持3行
        assert_eq!(form.description().cursor_row_col(), (3, 5));
        assert_eq!(form.description().scroll(), 1);

        // 上移光标回到第一行,窗口随之滚回顶部
        form.description_mut().up();
        form.description_mut().up();
        form.description_mut().up();
        assert_eq!(form.description().cursor_row_col(), (0, 5));
        assert_eq!(form.description().scroll(), 0);
        // 顶部继续上移保持不变
        form.description_mut().up();
        assert_eq!(form.description().cursor_row_col(), (0, 5));

        // 在第一行中间插入字符,编辑上面的文字
        form.description_mut().backspace();
        assert!(form.description().value().starts_with("line\n"));

        // 行首退格与上一行合并
        let mut form = FormData::add();
        for c in "ab".chars() {
            form.description_mut().insert(c);
        }
        form.description_mut().insert('\n');
        assert_eq!(form.description().cursor_row_col(), (1, 0));
        form.description_mut().backspace();
        assert_eq!(form.description().value(), "ab");
        assert_eq!(form.description().cursor_row_col(), (0, 2));

        // 移动到较短行时列位置截断
        let mut form = FormData::add();
        for c in "abcd\nx".chars() {
            form.description_mut().insert(c);
        }
        form.description_mut().up();
        assert_eq!(form.description().cursor_row_col(), (0, 1));
        // 插入字符使列变长后下移,列截断到短行长度
        for c in "XYZ".chars() {
            form.description_mut().insert(c);
        }
        form.description_mut().down();
        assert_eq!(form.description().cursor_row_col(), (1, 1));

        // change表单预填多行description,光标在末尾且窗口滚到底部
        let task = Task::new(
            1,
            "t".to_string(),
            Some("a\nb\nc\nd\ne".to_string()),
            None,
            Priority::Low,
        );
        let form = FormData::change(1, &task);
        assert_eq!(form.description().cursor_row_col(), (4, 1));
        assert_eq!(form.description().scroll(), 2);
    }

    /// description软换行: 超宽自动换行显示但不写入\n,上下键在显示行间移动
    #[test]
    fn test_description_soft_wrap() {
        let mut form = FormData::add();
        form.description_mut().set_wrap_width(5);
        for c in "abcdefgh".chars() {
            form.description_mut().insert(c);
        }
        // 8字符按宽度5软换行为两个显示行,文本中没有\n
        assert_eq!(form.description().value(), "abcdefgh");
        assert_eq!(form.description().rows(), vec![(0, 5), (5, 8)]);
        assert_eq!(form.description().cursor_row_col(), (1, 3));

        // 上下键在软换行出的显示行间移动
        form.description_mut().up();
        assert_eq!(form.description().cursor_row_col(), (0, 3));
        form.description_mut().down();
        assert_eq!(form.description().cursor_row_col(), (1, 3));

        // 显式换行与软换行共存
        form.description_mut().insert('\n');
        form.description_mut().insert('x');
        assert_eq!(form.description().value(), "abcdefgh\nx");
        assert_eq!(form.description().rows(), vec![(0, 5), (5, 8), (9, 10)]);
        assert_eq!(form.description().cursor_row_col(), (2, 1));

        // 左右键跨过软换行边界
        form.description_mut().left();
        form.description_mut().left();
        assert_eq!(form.description().cursor_row_col(), (1, 3));
        form.description_mut().left();
        assert_eq!(form.description().cursor_row_col(), (1, 2));
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
            app.form().map(|form| form.mode()),
            Some(FormMode::Add)
        ));

        // content: 输入"abc",左移一位插入"X"
        for c in "abc".chars() {
            handler::handle_key(&mut app, key(KeyCode::Char(c)));
        }
        handler::handle_key(&mut app, key(KeyCode::Left));
        handler::handle_key(&mut app, key(KeyCode::Char('X')));
        assert_eq!(app.form().unwrap().content(), "abXc");
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
        assert_eq!(app.form().unwrap().description().value(), "ab\ncd");
        assert_eq!(app.form().unwrap().description().cursor_row_col(), (1, 2));
        handler::handle_key(&mut app, key(KeyCode::Left)); // (1,1)
        handler::handle_key(&mut app, key(KeyCode::Left)); // (1,0)
        handler::handle_key(&mut app, key(KeyCode::Left)); // 跨行到(0,2)
        assert_eq!(app.form().unwrap().description().cursor_row_col(), (0, 2));
        handler::handle_key(&mut app, key(KeyCode::Right)); // 回到(1,0)
        assert_eq!(app.form().unwrap().description().cursor_row_col(), (1, 0));

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
        assert_eq!(app.form().unwrap().priority(), Priority::Medium);
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
        assert_eq!(app.search_line.value(), "taXsk");
        assert_eq!(app.search_line.cursor(), 3);

        // ctrl+c退出
        handler::handle_key(&mut app, ctrl('c'));
        assert!(app.should_quit());
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

    /// ctrl+d二次确认: 任意其他按键解除挂起并清除常驻警告, 解除后再按ctrl+d只重新挂起
    #[test]
    fn ctrl_d_double_confirm_clears_notice() {
        let guard = TempGuard::new("tui_ctrl_d");
        let store = setup_store(&guard);
        let mut app = App::new(&store).unwrap();

        // 第一次ctrl+d挂起, 显示常驻警告
        handler::handle_key(&mut app, ctrl('d'));
        assert!(app.pending_delete.is_some());
        assert!(app.message().is_some());

        // 任意其他按键解除挂起并清除残留警告
        handler::handle_key(&mut app, key(KeyCode::Down));
        assert!(app.pending_delete.is_none());
        assert!(app.message().is_none());

        // 解除后再按ctrl+d只挂起, 不会删除
        handler::handle_key(&mut app, ctrl('d'));
        assert_eq!(store.load().unwrap().len(), 2);

        // 挂起状态下连续ctrl+d才执行删除
        handler::handle_key(&mut app, ctrl('d'));
        assert_eq!(store.load().unwrap().len(), 1);
        assert!(app.message().is_some_and(|m| m.contains("deleted")));
    }

    /// 单字符快捷键禁止使用ctrl/alt组合, 且大小写不敏感
    #[test]
    fn single_char_ignore_ctrl_alt_modifiers() {
        let guard = TempGuard::new("single_char_ignore_ctrl_alt_modifiers");
        let store = setup_store(&guard);
        let mut app = App::new(&store).unwrap();

        // alt+q不应退出
        handler::handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('q'), KeyModifiers::ALT),
        );
        assert!(!app.should_quit());

        // alt+space不应切换done
        handler::handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char(' '), KeyModifiers::ALT),
        );
        assert!(!store.load().unwrap()[0].is_complete());

        // 排序模式下ctrl+p不触发排序, 停留在排序模式
        handler::handle_key(&mut app, ctrl('l'));
        handler::handle_key(&mut app, ctrl('p'));
        assert!(matches!(app.state, AppState::SortMode));
        assert!(app.sort.is_none());

        // 纯字符大写P按优先级排序(兼容大写锁定)
        handler::handle_key(&mut app, key(KeyCode::Char('P')));
        assert!(matches!(app.state, AppState::Main));
        assert!(matches!(app.sort, Some(SortBy::Priority)));
    }

    /// 确认弹窗不允许ctrl+y等组合键, 只有纯字符y才确认
    #[test]
    fn confirm_ignores() {
        let guard = TempGuard::new("confirm_ignores");
        let store = setup_store(&guard);
        let mut app = App::new(&store).unwrap();

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

        // ctrl+y不应确认, 停留在确认弹窗
        handler::handle_key(&mut app, ctrl('y'));
        assert!(matches!(app.state, AppState::Confirm(_)));
        assert_eq!(store.load().unwrap().len(), 1);

        // 纯字符y确认恢复
        handler::handle_key(&mut app, key(KeyCode::Char('y')));
        assert_eq!(store.load().unwrap().len(), 2);
        assert!(matches!(app.state, AppState::Main));
    }

    #[test]
    fn failed_change_keeps_form_and_message() {
        let guard = TempGuard::new("tui_form_error");
        let store = setup_store(&guard);
        let mut app = App::new(&store).unwrap();

        handler::handle_key(&mut app, ctrl('e'));
        delete_task(vec![1], &store).unwrap();
        // 用户已修改content: 走合并路径,TaskNotFound时表单保留
        // (未改动的保存走No changes快速路径,见untouched_form_save_reports_no_changes)
        handler::handle_key(&mut app, key(KeyCode::Char('X')));
        handler::handle_key(&mut app, ctrl('s'));

        assert!(app.form().is_some());
        assert!(
            app.message()
                .is_some_and(|text| text.contains("Task not found"))
        );
        render(&mut app); // 错误状态仍可正常渲染表单
    }

    /// 表单与外部进程修改不同字段时能够合并: 用户改的字段写入,未改的字段保留磁盘值
    #[test]
    fn change_form_merges_with_external_field_edits() {
        let guard = TempGuard::new("tui_edit_merge_fields");
        let store = setup_store(&guard);
        let mut app = App::new(&store).unwrap();

        handler::handle_key(&mut app, ctrl('e'));
        // 另一进程修改用户不会碰的字段(priority)
        store
            .update_with_backup(|tasks| {
                tasks[0].set_priority(Priority::Low);
                Ok(())
            })
            .unwrap();

        // 用户修改content后保存
        handler::handle_key(&mut app, key(KeyCode::Char('X')));
        handler::handle_key(&mut app, ctrl('s'));

        assert!(matches!(app.state, AppState::Main));
        let tasks = store.load().unwrap();
        assert_eq!(tasks[0].content(), "task1X"); // 用户修改生效
        assert_eq!(tasks[0].priority(), Priority::Low); // 外部修改保留
        assert_eq!(tasks[0].description(), Some("desc1"));
    }

    /// 双方都修改了description时拒绝保存: 外部值保持不变,表单保留等待重开
    #[test]
    fn change_form_rejects_same_field_conflict() {
        let guard = TempGuard::new("tui_edit_conflict");
        let store = setup_store(&guard);
        let mut app = App::new(&store).unwrap();

        handler::handle_key(&mut app, ctrl('e'));
        // 另一进程修改了与用户相同的字段(description)
        store
            .update_with_backup(|tasks| {
                tasks[0].set_description(Some("external".to_string()));
                Ok(())
            })
            .unwrap();

        // 用户也修改description: tab切到description栏输入
        handler::handle_key(&mut app, key(KeyCode::Tab));
        handler::handle_key(&mut app, key(KeyCode::Char('X')));
        handler::handle_key(&mut app, ctrl('s'));

        // 保存被拒: 表单保留,footer提示冲突,磁盘未被写入
        assert!(app.form().is_some());
        assert!(
            app.message()
                .is_some_and(|text| text.contains("changed elsewhere"))
        );
        let tasks = store.load().unwrap();
        assert_eq!(tasks[0].description(), Some("external"));
        assert_eq!(tasks[0].content(), "task1");
    }

    /// 外部进程只切换完成状态时表单可正常保存,完成状态不属于表单字段不会被覆盖
    #[test]
    fn change_form_save_preserves_external_status_change() {
        let guard = TempGuard::new("tui_edit_status_free");
        let store = setup_store(&guard);
        let mut app = App::new(&store).unwrap();

        handler::handle_key(&mut app, ctrl('e'));
        // 另一进程把任务标记为完成
        store
            .update_with_backup(|tasks| {
                tasks[0].complete();
                Ok(())
            })
            .unwrap();

        // 用户修改content后保存
        handler::handle_key(&mut app, key(KeyCode::Char('X')));
        handler::handle_key(&mut app, ctrl('s'));

        assert!(matches!(app.state, AppState::Main));
        let tasks = store.load().unwrap();
        assert_eq!(tasks[0].content(), "task1X"); // 用户修改生效
        assert!(tasks[0].is_complete()); // 外部的完成状态保留
    }

    /// 外部修改后未刷新的TUI打开表单且不改动任何内容: 保存不写盘,如实提示No changes,
    /// 磁盘保持外部值,刷新后列表可见外部修改(旧实现误报">>> Task 1 changed")
    #[test]
    fn untouched_form_save_reports_no_changes() {
        let guard = TempGuard::new("tui_form_untouched");
        let store = setup_store(&guard);
        let mut app = App::new(&store).unwrap();

        // 另一进程修改priority, TUI不刷新
        store
            .update_with_backup(|tasks| {
                tasks[0].set_priority(Priority::Low);
                Ok(())
            })
            .unwrap();
        handler::handle_key(&mut app, ctrl('e'));
        // 用户不修改任何内容,直接保存
        handler::handle_key(&mut app, ctrl('s'));

        assert!(matches!(app.state, AppState::Main));
        assert!(app.message().is_some_and(|m| m.contains("No changes")));
        // 磁盘未被TUI触碰,保持外部值
        assert_eq!(store.load().unwrap()[0].priority(), Priority::Low);
        // 刷新后列表与统计追上外部修改
        assert_eq!(app.tasks()[0].1.priority(), Priority::Low);
    }

    /// 打开change表单时按稳定ID重读磁盘: 外部修改在预填中可见,而非陈旧缓存值
    #[test]
    fn open_change_form_prefills_fresh_disk_values() {
        let guard = TempGuard::new("tui_form_fresh_prefill");
        let store = setup_store(&guard);
        let mut app = App::new(&store).unwrap();

        // 另一进程修改priority与description, TUI不刷新
        store
            .update_with_backup(|tasks| {
                tasks[0].set_priority(Priority::Low);
                tasks[0].set_description(Some("external".to_string()));
                Ok(())
            })
            .unwrap();
        handler::handle_key(&mut app, ctrl('e'));

        // 表单预填的是磁盘当前值而非陈旧缓存值,显示序号按当前文件位置重算
        let form = app.form().unwrap();
        assert_eq!(form.priority(), Priority::Low);
        assert_eq!(form.description().value(), "external");
        assert!(matches!(form.mode(), FormMode::Change { no: 1, .. }));
    }

    /// 任务已被并发删除时,打开表单阶段即报TaskNotFound,不再先开表单再在保存时失败
    #[test]
    fn open_change_form_reports_deleted_task_immediately() {
        let guard = TempGuard::new("tui_form_open_deleted");
        let store = setup_store(&guard);
        let mut app = App::new(&store).unwrap();

        delete_task(vec![1], &store).unwrap();
        handler::handle_key(&mut app, ctrl('e'));

        assert!(app.form().is_none());
        assert!(matches!(app.state, AppState::Main));
        assert!(app.message().is_some_and(|m| m.contains("Task not found")));
    }

    /// reload后选中跟随任务本身: 外部进程重排顺序时,选中的仍是用户当时看着的任务
    /// 任务被删除时回退到夹紧的旧位置,与旧行为一致
    #[test]
    fn reload_keeps_selection_on_the_same_task() {
        let guard = TempGuard::new("tui_selection_follows_task");
        let store = setup_store(&guard);
        add_task(&store, "task3".to_string(), None, None, None).unwrap();
        let mut app = App::new(&store).unwrap();

        // 移动光标到第三项(task3)
        handler::handle_key(&mut app, key(KeyCode::Down));
        handler::handle_key(&mut app, key(KeyCode::Down));
        assert_eq!(app.tasks()[2].1.content(), "task3");

        // 另一进程把task3挪到队首(模拟外部重排)
        store
            .update_with_backup(|tasks| {
                let task = tasks.remove(2);
                tasks.insert(0, task);
                Ok(())
            })
            .unwrap();
        app.reload().unwrap();

        // 选中跟随task3到新位置0,而不是停在旧下标2(那里现在是task2)
        assert_eq!(app.list_state.selected(), Some(0));
        assert_eq!(app.selected_task().unwrap().1.content(), "task3");

        // 选中任务被外部删除: 回退到夹紧的旧位置(此时选中下标为0,
        // task3删除后原位置显示的就是上移后的task1,与"保持同一行"的旧行为一致)
        delete_task(vec![1], &store).unwrap(); // 删除队首的task3
        app.reload().unwrap();
        assert_eq!(app.list_state.selected(), Some(0));
        assert_eq!(app.selected_task().unwrap().1.content(), "task1");
    }

    /// 选中任务未变化时reload保留详情滚动位置(定时同步不打断pgdn阅读);
    /// 选中变化时仍归零
    #[test]
    fn reload_preserves_details_scroll_when_selection_unchanged() {
        let guard = TempGuard::new("tui_reload_scroll");
        let store = TaskStore::new(Some(guard.main_path()), UserInterfaceTypes::Tui);
        let long_desc = (0..40)
            .map(|i| format!("description line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        add_task(&store, "long".to_string(), Some(long_desc), None, None).unwrap();
        add_task(&store, "task2".to_string(), None, None, None).unwrap();
        let mut app = App::new(&store).unwrap();
        render(&mut app); // 测得页大小

        // pgdn产生滚动
        handler::handle_key(&mut app, key(KeyCode::PageDown));
        render(&mut app);
        assert!(app.details_scroll > 0);

        // 外部进程只改另一个任务的内容,选中任务不变: reload保留滚动
        store
            .update_with_backup(|tasks| {
                tasks[1].set_content("changed".to_string());
                Ok(())
            })
            .unwrap();
        app.reload().unwrap();
        assert!(app.details_scroll > 0, "选中任务未变,滚动位置应保留");

        // 切换选中任务: 归零(现有行为)
        handler::handle_key(&mut app, key(KeyCode::Down));
        assert_eq!(app.details_scroll, 0);
    }

    /// 距上次同步超过SYNC_INTERVAL时auto_sync从磁盘刷新,未到点则不动作
    #[test]
    fn auto_sync_refreshes_after_interval() {
        let guard = TempGuard::new("tui_auto_sync");
        let store = setup_store(&guard);
        let mut app = App::new(&store).unwrap();

        // 外部进程追加一个任务
        add_task(&store, "external".to_string(), None, None, None).unwrap();

        // 未到点: 不刷新,列表仍为2项
        app.auto_sync().unwrap();
        assert_eq!(app.tasks().len(), 2);

        // 回拨定时器到已到点: 刷新,列表追上外部修改
        app.backdate_sync_timer();
        app.auto_sync().unwrap();
        assert_eq!(app.tasks().len(), 3);
        assert_eq!(app.tasks()[2].1.content(), "external");
    }

    /// 表单入口先同步再开: 背景列表与表单同源一致
    #[test]
    fn open_form_syncs_list_with_disk() {
        let guard = TempGuard::new("tui_form_entry_sync");
        let store = setup_store(&guard);
        let mut app = App::new(&store).unwrap();

        // 另一进程追加任务并修改task1的优先级, TUI不刷新
        add_task(&store, "external".to_string(), None, None, None).unwrap();
        store
            .update_with_backup(|tasks| {
                tasks[0].set_priority(Priority::Low);
                Ok(())
            })
            .unwrap();

        // ctrl+a: 列表同步后再开add表单
        handler::handle_key(&mut app, ctrl('a'));
        assert!(app.form().is_some());
        assert_eq!(app.tasks().len(), 3, "打开add表单前列表已同步");

        // esc回主界面, ctrl+e: 表单与列表同源
        handler::handle_key(&mut app, key(KeyCode::Esc));
        handler::handle_key(&mut app, ctrl('e'));
        let form = app.form().unwrap();
        assert_eq!(form.priority(), Priority::Low, "表单预填外部修改后的值");
        assert_eq!(app.tasks().len(), 3, "打开change表单前列表已同步");
    }

    #[test]
    fn unicode_editing_and_input_window_use_terminal_width() {
        let mut input = InputLine::new("e\u{301}");
        input.backspace();
        assert!(input.value().is_empty()); // 一次删除整个组合字素
        assert_eq!(input.cursor(), 0);

        assert_eq!(ui::display_width("e\u{301}"), 1);
        assert_eq!(ui::display_width("👩‍💻"), 2);
        assert_eq!(ui::input_window("abcdefgh", 8, 4), ("fgh".to_string(), 3));

        let mut form = FormData::add();
        form.description_mut().set_wrap_width(2);
        for c in "e\u{301}中".chars() {
            form.description_mut().insert(c);
        }
        assert_eq!(form.description().rows(), vec![(0, 2), (2, 3)]);
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

    /// content/deadline与搜索框共用一套编辑按键: home/end 同样可用
    #[test]
    fn single_line_home_end() {
        let guard = TempGuard::new("single_line_home_end");
        let store = setup_store(&guard);
        let mut app = App::new(&store).unwrap();

        // ctrl+a进入add表单: content输入"abc" -> home插入X -> end插入Y
        handler::handle_key(&mut app, ctrl('a'));
        assert!(matches!(
            app.form().map(|form| form.mode()),
            Some(FormMode::Add)
        ));
        for c in "abc".chars() {
            handler::handle_key(&mut app, key(KeyCode::Char(c)));
        }
        handler::handle_key(&mut app, key(KeyCode::Home));
        handler::handle_key(&mut app, key(KeyCode::Char('X')));
        assert_eq!(app.form().unwrap().content(), "Xabc");
        assert_eq!(app.form().unwrap().content_cursor(), 1);
        handler::handle_key(&mut app, key(KeyCode::End));
        handler::handle_key(&mut app, key(KeyCode::Char('Y')));
        assert_eq!(app.form().unwrap().content(), "XabcY");
        assert_eq!(app.form().unwrap().content_cursor(), 5);

        // tab两次切到deadline, home/end 同样生效
        handler::handle_key(&mut app, key(KeyCode::Tab)); // description
        handler::handle_key(&mut app, key(KeyCode::Tab)); // deadline
        assert_eq!(app.form().unwrap().focus, FormField::Deadline);
        for c in "2026".chars() {
            handler::handle_key(&mut app, key(KeyCode::Char(c)));
        }
        handler::handle_key(&mut app, key(KeyCode::Home));
        handler::handle_key(&mut app, key(KeyCode::Char('1')));
        assert_eq!(app.form().unwrap().deadline(), "12026");
        assert_eq!(app.form().unwrap().deadline_cursor(), 1);
    }

    /// 搜索框的 home/end 与表单单行输入框行为一致
    #[test]
    fn search_input_home_end() {
        let guard = TempGuard::new("tui_search_home_end");
        let store = setup_store(&guard);
        let mut app = App::new(&store).unwrap();

        handler::handle_key(&mut app, ctrl('p'));
        handler::handle_key(&mut app, key(KeyCode::Enter)); // search
        for c in "task".chars() {
            handler::handle_key(&mut app, key(KeyCode::Char(c)));
        }
        handler::handle_key(&mut app, key(KeyCode::Home));
        assert_eq!(app.search_line.cursor(), 0);
        handler::handle_key(&mut app, key(KeyCode::Char('X')));
        assert_eq!(app.search_line.value(), "Xtask");
        handler::handle_key(&mut app, key(KeyCode::End));
        handler::handle_key(&mut app, key(KeyCode::Char('Y')));
        assert_eq!(app.search_line.value(), "XtaskY");
        assert_eq!(app.search_line.cursor(), 6);
    }

    /// 任务列表与选项菜单统一支持 j/k 移动
    #[test]
    fn jk_moves_in_list_and_menu() {
        let guard = TempGuard::new("tui_jk_move");
        let store = setup_store(&guard);
        let mut app = App::new(&store).unwrap();

        // 主界面: j/k 与 ↓/↑ 等价
        handler::handle_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.list_state.selected(), Some(1));
        handler::handle_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.list_state.selected(), Some(0)); // 循环回顶部
        handler::handle_key(&mut app, key(KeyCode::Char('k')));
        assert_eq!(app.list_state.selected(), Some(1)); // 循环到底部

        // 多选模式: 同样支持 j/k
        handler::handle_key(&mut app, ctrl('p'));
        handler::handle_key(&mut app, key(KeyCode::Down)); // add
        handler::handle_key(&mut app, key(KeyCode::Down)); // multiple choices
        handler::handle_key(&mut app, key(KeyCode::Enter));
        assert!(matches!(app.state, AppState::MultiSelect));
        handler::handle_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.list_state.selected(), Some(0));

        // 选项菜单: j/k 移动菜单光标
        handler::handle_key(&mut app, key(KeyCode::Char(' ')));
        assert!(app.multi_selected.contains(&1));
        handler::handle_key(&mut app, key(KeyCode::Enter));
        assert!(app.multi_menu_open);
        assert_eq!(app.menu_index, 0);
        handler::handle_key(&mut app, key(KeyCode::Char('j'))); // undone
        assert_eq!(app.menu_index, 1);
        handler::handle_key(&mut app, key(KeyCode::Char('k')));
        assert_eq!(app.menu_index, 0);
        handler::handle_key(&mut app, key(KeyCode::Esc));
        assert!(!app.multi_menu_open);
        assert!(matches!(app.state, AppState::MultiSelect));
    }

    /// 菜单首项标签(done/undone)与回车执行的动作一致
    #[test]
    fn task_options_label_matches_action() {
        let guard = TempGuard::new("tui_label_matches_action");
        let store = setup_store(&guard);
        let mut app = App::new(&store).unwrap();

        // 未完成: 首项显示 Done, 回车后置为完成
        assert!(!app.selected_done());
        assert_eq!(TaskOpMenu::StatusChange.label(false), "Done");
        handler::handle_key(&mut app, key(KeyCode::Enter));
        handler::handle_key(&mut app, key(KeyCode::Enter));
        assert!(store.load().unwrap()[0].is_complete());
        assert!(app.message().is_some_and(|m| m.contains("Task 1 done")));

        // 已完成: 首项显示 Undone, 回车后置为未完成, 提示与标签一致
        assert!(app.selected_done());
        assert_eq!(TaskOpMenu::StatusChange.label(true), "Undone");
        handler::handle_key(&mut app, key(KeyCode::Enter));
        handler::handle_key(&mut app, key(KeyCode::Enter));
        assert!(!store.load().unwrap()[0].is_complete());
        assert!(app.message().is_some_and(|m| m.contains("Task 1 undone")));
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
