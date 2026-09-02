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
    tui::{form_state::FormData, text::InputLine},
};

/// App界面状态
pub enum AppState {
    /// 主界面
    Main,
    /// enter 选项菜单
    TaskOptions,
    /// ctrl+p 命令面板
    Settings,
    /// 命令面板内嵌搜索
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

/// Task可以进行的操作
#[derive(Clone, Copy)]
pub enum TaskAction {
    Complete,
    Incomplete,
    Delete,
}

/// 自动同步间隔: 距上次同步超过该时长时,主循环自动从磁盘刷新一次
///
/// 长驻TUI与多个CLI进程共享同一份task.json,后台定时刷新使外部修改
/// 无需用户操作即可在界面上可见
const SYNC_INTERVAL: Duration = Duration::from_secs(2);

/// TUI应用状态
pub struct App<'a> {
    pub store: &'a TaskStore,
    pub state: AppState,
    /// 当前展示的任务列表,(文件中的序号, 任务)
    ///
    /// 与all_tasks同为磁盘快照的两份视图,只在reload时同步刷新
    tasks: Vec<(usize, Task)>,
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
    /// 命令面板内嵌的搜索输入框
    pub search_line: InputLine,
    /// 未经过搜索过滤的任务，用于实时计算状态统计。
    ///
    /// 与tasks仅在reload中一同刷新,单独修改会使统计与列表不一致
    all_tasks: Vec<Task>,
    /// 底部提示信息(操作结果或错误),超过2秒自动清除
    ///
    /// 与message_time共同维护"是否会被自动清除",只能用set_message/set_notice/clear_message修改
    message: Option<String>,
    /// message的设置时间,用于定时清除
    message_time: Option<Instant>,
    /// 详情面板的垂直滚动偏移(显示行数)
    pub details_scroll: usize,
    /// 详情面板每次翻页的行数(渲染时更新)
    pub details_page: usize,
    /// 暂存待删除的任务ID，用于ctrl+d二次确认
    pub pending_delete: Option<usize>,
    /// 上次从磁盘同步的时刻,超过SYNC_INTERVAL时主循环触发自动同步
    last_sync: Instant,
    should_quit: bool,
}

/// 一次加载得到的两份任务视图: 带序号的展示列表 + 全量任务(供状态统计)
struct TaskViews {
    listed: Vec<(usize, Task)>,
    all: Vec<Task>,
}

/// 一次加载同时得到展示列表与全量任务列表
///
/// find未激活时展示列表即全量列表(带序号),从同一快照反解出all_tasks,
/// 省去第二次文件锁与解析,且保证两份视图来自同一快照;
/// find激活时展示列表是子集,无法反解全量,all_tasks需单独加载
fn load_views(
    store: &TaskStore,
    sort: Option<SortBy>,
    find: Option<&str>,
) -> Result<TaskViews, AppError> {
    let listed = list_tasks(store, sort, find.map(str::to_string))?.unwrap_or_default();
    let all = if find.is_none() {
        listed.iter().map(|(_, t)| t.clone()).collect()
    } else {
        store.load()?
    };
    Ok(TaskViews { listed, all })
}

impl<'a> App<'a> {
    pub fn new(store: &'a TaskStore) -> Result<Self, AppError> {
        let views = load_views(store, None, None)?;
        let mut list_state = ListState::default();
        if !views.listed.is_empty() {
            list_state.select(Some(0));
        }

        let mut app = App {
            store,
            state: AppState::Main,
            tasks: views.listed,
            list_state,
            all_tasks: views.all,
            sort: None,
            find: None,
            menu_index: 0,
            multi_selected: HashSet::new(),
            multi_menu_open: false,
            search_line: InputLine::new(String::new()),
            message: None,
            message_time: None,
            details_scroll: 0,
            details_page: 10,
            pending_delete: None,
            last_sync: Instant::now(),
            should_quit: false,
        };
        app.consume_notice();
        Ok(app)
    }

    /// 当前展示的任务列表,与all_tasks一同刷新
    pub fn tasks(&self) -> &[(usize, Task)] {
        &self.tasks
    }

    /// 底部提示信息,存在时优先占据footer
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// 退出主循环
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// 返回当前选中任务是否已完成
    ///
    /// 仅供菜单首项标签(done/undone)派生显示; 实际切换以磁盘真值为翻转基准,
    /// 见handler::toggle_status
    pub fn selected_done(&self) -> bool {
        self.selected_task().is_some_and(|(_, t)| t.is_complete())
    }

    /// 重新从磁盘加载任务列表和状态,保持当前的排序与搜索条件
    ///
    /// find未激活时两次刷新来自同一快照(见load_views);
    /// 刷新后选中跟随任务本身(按稳定ID重新定位)而非位置,外部进程增删/重排时
    /// 选中的仍是用户当时看着的那个任务,任务消失时回退到夹紧的旧位置;
    /// 选中任务未变化时保留详情面板滚动位置,避免后台定时刷新打断pgdn阅读
    pub fn reload(&mut self) -> Result<(), AppError> {
        // 刷新前先记住用户选中的任务与位置
        let prev = self.list_state.selected().and_then(|i| self.tasks.get(i));
        let prev_id = prev.map(|(_, t)| t.id());
        let prev_index = self.list_state.selected();

        let views = load_views(self.store, self.sort.clone(), self.find.as_deref())?;
        self.tasks = views.listed;
        self.all_tasks = views.all;

        let selected = match prev_id {
            // 任务已消失: 回退到旧位置并夹紧; 无旧位置(如空表新增)则选中首项
            Some(id) => self
                .tasks
                .iter()
                .position(|(_, t)| t.id() == id)
                .or_else(|| self.fallback_selection(prev_index)),
            // 刷新前无选中: 列表非空时选中首项
            None => self.fallback_selection(prev_index),
        };
        self.list_state.select(selected);
        // 刷新前后展示的任务不同(选中变化或列表空↔非空)时详情面板回到顶部;
        // 同一任务的背景刷新保留滚动位置,避免定时同步打断pgdn阅读
        let now_id = selected.and_then(|i| self.tasks.get(i).map(|(_, t)| t.id()));
        if now_id != prev_id {
            self.details_scroll = 0;
        }
        self.last_sync = Instant::now();
        Ok(())
    }

    /// 兜底选中: 列表非空时取旧位置(无旧位置则首项)并夹紧到新长度
    fn fallback_selection(&self, prev_index: Option<usize>) -> Option<usize> {
        if self.tasks.is_empty() {
            None
        } else {
            Some(prev_index.map_or(0, |i| i.min(self.tasks.len() - 1)))
        }
    }

    /// 距上次同步超过SYNC_INTERVAL时自动从磁盘刷新
    ///
    /// 由主循环每个循环周期调用; 错误由调用方降级为footer消息而不中断程序
    pub fn auto_sync(&mut self) -> Result<(), AppError> {
        if self.last_sync.elapsed() >= SYNC_INTERVAL {
            self.reload()
        } else {
            Ok(())
        }
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

    /// 把同步定时器回拨到"已到点"状态,仅供测试
    #[cfg(test)]
    pub(crate) fn backdate_sync_timer(&mut self) {
        self.last_sync = Instant::now() - SYNC_INTERVAL;
    }
}
