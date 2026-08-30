//! App状态模块和主循环

use std::collections::HashSet;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use ratatui::text::Line;
use ratatui::widgets::ListState;
use unicode_segmentation::UnicodeSegmentation;

use crate::error::AppError;
use crate::io::storage::{TaskStore, recovered_from_backup_msg};
use crate::task::{Priority, Task};
use crate::time::to_local_time;
use crate::todo::{SortBy, list_tasks};

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

/// 编辑页面模式
#[derive(Clone, Copy)]
pub enum FormMode {
    /// 添加模式
    Add,
    /// 修改模式
    Change { no: usize, id: usize },
}

/// 修改界面当前聚焦的输入框类型
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormField {
    Content,
    Description,
    Deadline,
    Priority,
}

/// 实现输入框正向和反向的循环选择
impl FormField {
    pub fn next(self) -> Self {
        match self {
            FormField::Content => FormField::Description,
            FormField::Description => FormField::Deadline,
            FormField::Deadline => FormField::Priority,
            FormField::Priority => FormField::Content,
        }
    }

    pub fn back(self) -> Self {
        match self {
            FormField::Priority => FormField::Deadline,
            FormField::Deadline => FormField::Description,
            FormField::Description => FormField::Content,
            FormField::Content => FormField::Priority,
        }
    }
}

/// description输入框的可见行数
pub const DESC_VISIBLE_LINES: usize = 3;

/// 需要二次确认的行为
pub enum ConfirmAction {
    /// 删除所有完成的任务
    DeleteAll(Vec<Task>),
    /// 恢复上一个操作
    Undo(Vec<Task>),
}

/// add / change 表单数据
///
/// 各输入框以字符串保存,保存时再进行解析和转换
/// content/deadline为单行输入框,description为多行文本框:
/// - enter换行(保存为\n),方向键移动光标,内容超出可见行数时自动滚动
/// - 超出输入宽度的部分自动软换行显示(不写入\n)
#[derive(Clone)]
pub struct FormData {
    pub mode: FormMode,
    pub content: String,
    pub description: String,
    pub deadline: String,
    pub priority: Priority,
    /// 当前聚焦的输入框的类型
    pub focus: FormField,
    /// content的编辑光标(字符下标)
    pub content_cursor: usize,
    /// deadline的编辑光标(字符下标)
    pub deadline_cursor: usize,
    /// description的编辑光标(字符下标)
    desc_cursor: usize,
    /// description可见窗口的首个显示行下标
    desc_scroll: usize,
    /// description文本区的显示宽度(渲染时更新,编辑时用于计算软换行)
    desc_wrap_width: usize,
}

impl FormData {
    /// 空白add表单
    pub fn add() -> Self {
        FormData {
            mode: FormMode::Add,
            content: String::new(),
            description: String::new(),
            deadline: String::new(),
            priority: Priority::default(),
            focus: FormField::Content,
            content_cursor: 0,
            deadline_cursor: 0,
            desc_cursor: 0,
            desc_scroll: 0,
            desc_wrap_width: 46,
        }
    }

    /// 使用选中task的信息预填change表单,各栏光标置于文本末尾
    pub fn change(no: usize, task: &Task) -> Self {
        let content = task.content().to_string();
        let description = task.description().unwrap_or_default().to_string();
        let deadline = task
            .deadline()
            .map(|d| to_local_time(&d).format("%Y-%m-%dT%H:%M:%S").to_string())
            .unwrap_or_default();
        let mut form = FormData {
            mode: FormMode::Change { no, id: task.id() },
            priority: task.priority(),
            focus: FormField::Content,
            content_cursor: content.chars().count(),
            deadline_cursor: deadline.chars().count(),
            desc_cursor: description.chars().count(),
            desc_scroll: 0,
            desc_wrap_width: 46,
            content,
            description,
            deadline,
        };
        form.adjust_desc_scroll();
        form
    }

    /// 在description光标处插入字符(含换行符)
    pub fn desc_insert(&mut self, c: char) {
        insert_at_cursor(&mut self.description, &mut self.desc_cursor, c);
        self.adjust_desc_scroll();
    }

    /// 删除description光标前的一个字符(行首退格则与上一行合并)
    pub fn desc_backspace(&mut self) {
        backspace_at_cursor(&mut self.description, &mut self.desc_cursor);
        self.adjust_desc_scroll();
    }

    /// description光标左移一个字符(行首则移到上一行末尾)
    pub fn desc_left(&mut self) {
        move_cursor_left(&self.description, &mut self.desc_cursor);
        self.adjust_desc_scroll();
    }

    /// description光标右移一个字符(行尾则移到下一行行首)
    pub fn desc_right(&mut self) {
        move_cursor_right(&self.description, &mut self.desc_cursor);
        self.adjust_desc_scroll();
    }

    /// description光标上移一个显示行,列位置截断到目标行长度
    pub fn desc_up(&mut self) {
        let rows = self.desc_rows();
        let (row, _) = self.desc_cursor_row_col();
        if row == 0 {
            return;
        }
        let target_width = range_width(&self.description, rows[row].0, self.desc_cursor);
        let (start, end) = rows[row - 1];
        self.desc_cursor = cursor_at_width(&self.description, start, end, target_width);
        self.adjust_desc_scroll();
    }

    /// description光标下移一个显示行,列位置截断到目标行长度
    pub fn desc_down(&mut self) {
        let rows = self.desc_rows();
        let (row, _) = self.desc_cursor_row_col();
        if row + 1 >= rows.len() {
            return;
        }
        let target_width = range_width(&self.description, rows[row].0, self.desc_cursor);
        let (start, end) = rows[row + 1];
        self.desc_cursor = cursor_at_width(&self.description, start, end, target_width);
        self.adjust_desc_scroll();
    }

    /// 返回description按显式\n分行并按显示宽度软换行后的所有显示行
    ///
    /// 每个显示行为原字符串中的字符下标范围(起始, 结束)
    pub fn desc_rows(&self) -> Vec<(usize, usize)> {
        wrap_rows(&self.description, self.desc_wrap_width)
    }

    /// 返回description光标所在的(显示行, 列)
    pub fn desc_cursor_row_col(&self) -> (usize, usize) {
        cursor_row_col(&self.desc_rows(), self.desc_cursor)
    }

    /// 返回description可见窗口的首个显示行下标
    pub fn desc_scroll(&self) -> usize {
        self.desc_scroll
    }

    /// 设置description文本区的显示宽度(渲染时调用)
    pub fn set_desc_wrap_width(&mut self, width: usize) {
        self.desc_wrap_width = width.max(1);
    }

    /// 保持光标所在显示行处于可见窗口内,超出则滚动
    pub fn adjust_desc_scroll(&mut self) {
        let (row, _) = self.desc_cursor_row_col();
        if row < self.desc_scroll {
            self.desc_scroll = row;
        } else if row >= self.desc_scroll + DESC_VISIBLE_LINES {
            self.desc_scroll = row + 1 - DESC_VISIBLE_LINES;
        }
    }
}

// 以下是文本编辑的辅助函数,字符串下标均按字符计

/// 在字符串光标处插入字符
pub fn insert_at_cursor(s: &mut String, cursor: &mut usize, c: char) {
    let byte = byte_idx(s, *cursor);
    s.insert(byte, c);
    *cursor += 1;
}

/// 删除字符串光标前的一个字符
pub fn backspace_at_cursor(s: &mut String, cursor: &mut usize) {
    if *cursor == 0 {
        return;
    }
    let old_cursor = *cursor;
    move_cursor_left(s, cursor);
    let range = byte_idx(s, *cursor)..byte_idx(s, old_cursor);
    s.replace_range(range, "");
}

/// 将光标向左移动一个字素簇。
pub fn move_cursor_left(s: &str, cursor: &mut usize) {
    if *cursor == 0 {
        return;
    }
    let prefix: String = s.chars().take(*cursor).collect();
    let step = prefix
        .graphemes(true)
        .next_back()
        .map_or(1, |g| g.chars().count());
    *cursor = cursor.saturating_sub(step);
}

/// 将光标向右移动一个字素簇。
pub fn move_cursor_right(s: &str, cursor: &mut usize) {
    let suffix: String = s.chars().skip(*cursor).collect();
    if let Some(grapheme) = suffix.graphemes(true).next() {
        *cursor += grapheme.chars().count();
    }
}

/// 第char_idx个字符对应的字节下标
fn byte_idx(s: &str, char_idx: usize) -> usize {
    s.char_indices().nth(char_idx).map_or(s.len(), |(i, _)| i)
}

fn range_width(s: &str, start: usize, end: usize) -> usize {
    let text: String = s.chars().skip(start).take(end - start).collect();
    Line::from(text).width()
}

fn cursor_at_width(s: &str, start: usize, end: usize, target_width: usize) -> usize {
    let text: String = s.chars().skip(start).take(end - start).collect();
    let mut cursor = start;
    let mut width = 0;
    for grapheme in text.graphemes(true) {
        let next_width = width + Line::from(grapheme).width();
        if next_width > target_width {
            break;
        }
        width = next_width;
        cursor += grapheme.chars().count();
    }
    cursor
}

/// 将文本按显式\n分行，再按终端显示宽度软换行。
///
/// 返回所有显示行,每个显示行为原字符串中的字符下标范围(起始, 结束)
fn wrap_rows(s: &str, width: usize) -> Vec<(usize, usize)> {
    let width = width.max(1);
    let mut rows = Vec::new();
    let mut line_start = 0; // 当前逻辑行的起始字符下标
    for logical in s.split('\n') {
        let line_len = logical.chars().count();
        if line_len == 0 {
            rows.push((line_start, line_start));
        } else {
            let mut row_start = 0;
            let mut row_width = 0;
            let mut char_index = 0;
            for grapheme in logical.graphemes(true) {
                let char_count = grapheme.chars().count();
                let grapheme_width = Line::from(grapheme).width();
                if row_width + grapheme_width > width && char_index > row_start {
                    rows.push((line_start + row_start, line_start + char_index));
                    row_start = char_index;
                    row_width = 0;
                }
                row_width += grapheme_width;
                char_index += char_count;
            }
            rows.push((line_start + row_start, line_start + line_len));
        }
        line_start += line_len + 1; // 跳过\n
    }
    rows
}

/// 光标字符下标对应的(显示行, 列)
///
/// 光标位于软换行边界时归入下一行行首
fn cursor_row_col(rows: &[(usize, usize)], idx: usize) -> (usize, usize) {
    let row = rows
        .iter()
        .rposition(|&(start, _)| start <= idx)
        .unwrap_or(0);
    (row, idx - rows[row].0)
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
        Ok(())
    }

    /// 光标上移一项(循环)
    pub fn select_previous(&mut self) {
        let len = self.tasks.len();
        if len == 0 {
            return;
        }
        let i = self
            .list_state
            .selected()
            .map_or(0, |i| (i + len - 1) % len);
        self.list_state.select(Some(i));
    }

    /// 光标下移一项(循环)
    pub fn select_next(&mut self) {
        let len = self.tasks.len();
        if len == 0 {
            return;
        }
        let i = self.list_state.selected().map_or(0, |i| (i + 1) % len);
        self.list_state.select(Some(i));
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
    pub fn menu_previous(&mut self, len: usize) {
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
            self.message = None;
            self.message_time = None;
        }
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
