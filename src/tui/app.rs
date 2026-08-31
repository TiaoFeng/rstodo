//! App状态模块

use chrono::{DateTime, Utc};
use ratatui::widgets::ListState;
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

use crate::{
    error::AppError,
    io::storage::{TaskStore, recovered_from_backup_msg},
    task::Task,
    todo::{SortBy, list_tasks},
    tui::form_state::FormData,
};

/// App界面状态
pub enum AppState {
    /// 主界面
    Main,
    /// enter 选项菜单
    TaskOptions,
    /// ctrl+p 命令面板
    Settings,
    /// 命令面干内嵌搜索
    SearchInput,
    /// 多选模式
    MultiSelect,
    /// ctrl+l 排序模式
    SortMode,
    /// add / change 表单
    Form(FormData),
    /// 二次确认界面
    Confirm(ConfirmAction),
}

/// 需要二次确认的行为
pub enum ConfirmAction {
    /// 删除所有完成的任务
    DeleteAll(Vec<Task>),
    /// 恢复上一个操作
    Undo(Vec<Task>),
}

/// TUI应用状态
pub struct App<'a> {
    pub store: &'a TaskStore,
    pub state: AppState,
    /// 当前展示的任务列表,(文件中的序号, 任务)
    pub tasks: Vec<(usize, Task)>,
    pub list_state: ListState,
    /// 当前排序方式,None表示默认(文件顺序)
    pub sort: Option<SortBy>,
    /// 当前搜索关键词,None表示未搜索
    pub find: Option<String>,
    /// 弹出选项菜单的光标下标
    pub menu_index: usize,
    /// 多选模式下被选中的稳定任务 ID 集合
    pub multi_selected: HashSet<usize>,
    /// 多选确认后的done/undone/delete选项菜单是否展开
    pub multi_menu_open: bool,
    /// 命令面板内嵌搜索的输入内容
    pub search_input: String,
    /// 未经过搜索过滤的任务，用于实时计算状态统计。
    pub all_tasks: Vec<Task>,
    /// 底部提示信息(操作结果或错误),超过2秒自动清除
    pub message: Option<String>,
    /// message的设置时间,用于定时清除
    pub message_time: Option<Instant>,
    /// 搜索输入框的编辑光标(字符下标)
    pub search_cursor: usize,
    /// 详情面板的垂直滚动偏移(显示行数)
    pub details_scroll: usize,
    /// 详情面板每次翻页的行数(渲染时更新)
    pub details_page: usize,
    /// 暂存待删除的任务ID，用于ctrl+d二次确认
    pub pending_delete: Option<usize>,
    pub should_quit: bool,
}

impl<'a> App<'a> {
    pub fn new(store: &'a TaskStore) -> Result<Self, AppError> {
        let tasks = list_tasks(store, None, None)?.unwrap_or_default();
        let all_tasks = store.load()?;
        let mut list_state = ListState::default();
        if !tasks.is_empty() {
            list_state.select(Some(0));
        }

        let mut app = App {
            store,
            state: AppState::Main,
            tasks,
            list_state,
            all_tasks,
            sort: None,
            find: None,
            menu_index: 0,
            multi_selected: HashSet::new(),
            multi_menu_open: false,
            search_input: String::new(),
            message: None,
            message_time: None,
            search_cursor: 0,
            details_scroll: 0,
            details_page: 10,
            pending_delete: None,
            should_quit: false,
        };
        app.consume_notice();
        Ok(app)
    }

    /// 重新从磁盘加载任务列表和状态,保持当前的排序与搜索条件
    pub fn reload(&mut self) -> Result<(), AppError> {
        self.tasks =
            list_tasks(self.store, self.sort.clone(), self.find.clone())?.unwrap_or_default();
        self.all_tasks = self.store.load()?;
        let selected = match self.list_state.selected() {
            Some(i) if !self.tasks.is_empty() => Some(i.min(self.tasks.len() - 1)),
            None if !self.tasks.is_empty() => Some(0),
            _ => None,
        };
        self.list_state.select(selected);
        // 任务列表刷新后详情面板回到顶部
        self.details_scroll = 0;
        Ok(())
    }

    /// 光标上移一项(循环)
    pub fn select_back(&mut self) {
        let len = self.tasks.len();
        if len == 0 {
            return;
        }
        let i = self
            .list_state
            .selected()
            .map_or(0, |i| (i + len - 1) % len);
        self.list_state.select(Some(i));
        // 切换选中任务后详情面板回到顶部
        self.details_scroll = 0;
    }

    /// 光标下移一项(循环)
    pub fn select_next(&mut self) {
        let len = self.tasks.len();
        if len == 0 {
            return;
        }
        let i = self.list_state.selected().map_or(0, |i| (i + 1) % len);
        self.list_state.select(Some(i));
        // 切换选中任务后详情面板回到顶部
        self.details_scroll = 0;
    }

    /// 返回当前选中的(序号, 任务)
    pub fn selected_task(&self) -> Option<&(usize, Task)> {
        self.list_state.selected().and_then(|i| self.tasks.get(i))
    }

    pub fn form(&self) -> Option<&FormData> {
        match &self.state {
            AppState::Form(form) => Some(form),
            _ => None,
        }
    }

    /// 根据当前时刻实时计算状态，避免 deadline 跨越后统计停留在旧值。
    pub fn status_counts(&self, now: DateTime<Utc>) -> (usize, usize, usize, usize) {
        let total = self.all_tasks.len();
        let done = self
            .all_tasks
            .iter()
            .filter(|task| task.is_complete())
            .count();
        let overdue = self
            .all_tasks
            .iter()
            .filter(|task| task.is_overdue(now))
            .count();
        (total, done, total - done, overdue)
    }

    /// 菜单光标上移一项(循环)
    pub fn menu_back(&mut self, len: usize) {
        if len > 0 {
            self.menu_index = (self.menu_index + len - 1) % len;
        }
    }

    /// 菜单光标下移一项(循环)
    pub fn menu_next(&mut self, len: usize) {
        if len > 0 {
            self.menu_index = (self.menu_index + 1) % len;
        }
    }

    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = Some(message.into());
        self.message_time = Some(Instant::now());
    }

    /// 清除超过2秒的提示信息,恢复footer按键提示
    pub fn expire_message(&mut self) {
        if self
            .message_time
            .is_some_and(|set_at| set_at.elapsed() >= Duration::from_secs(2))
        {
            self.clear_message();
        }
    }

    /// 立即清除底部提示信息
    pub fn clear_message(&mut self) {
        self.message = None;
        self.message_time = None;
    }

    /// 常驻提示: 不设message_time,不会被2秒清除; 下次set_message覆盖
    pub fn set_notice(&mut self, message: impl Into<String>) {
        self.message = Some(message.into());
        self.message_time = None;
    }

    /// 消费存储层的待处理通知并转为footer常驻警告
    pub fn consume_notice(&mut self) {
        if self.store.take_notice().is_some() {
            self.set_notice(recovered_from_backup_msg(
                self.store.backup_path(),
                self.store.interface_type(),
            ));
        }
    }
}
