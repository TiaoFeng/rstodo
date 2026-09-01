//! add / change界面

use crate::{
    task::{Priority, Task},
    time::to_local_time,
    tui::text::{
        InputLine, backspace_at_cursor, cursor_at_width, cursor_row_col, insert_at_cursor,
        move_cursor_left, move_cursor_right, range_width, wrap_rows,
    },
};

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

/// add / change 表单数据
///
/// 各输入框以字符串保存,保存时再进行解析和转换
/// content/deadline为单行输入框,description为多行文本框:
/// - enter换行(保存为\n),方向键移动光标,内容超出可见行数时自动滚动
/// - 超出输入宽度的部分自动软换行显示(不写入\n)
#[derive(Clone)]
pub struct FormData {
    mode: FormMode,
    content: InputLine,
    description: String,
    deadline: InputLine,
    priority: Priority,
    /// 当前聚焦的输入框的类型
    pub focus: FormField,
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
            content: InputLine::new(String::new()),
            description: String::new(),
            deadline: InputLine::new(String::new()),
            priority: Priority::default(),
            focus: FormField::Content,
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
            content: InputLine::new(content),
            deadline: InputLine::new(deadline),
            priority: task.priority(),
            focus: FormField::Content,
            desc_cursor: description.chars().count(),
            desc_scroll: 0,
            desc_wrap_width: 46,
            description,
        };
        form.adjust_desc_scroll();
        form
    }

    pub fn mode(&self) -> FormMode {
        self.mode
    }

    pub fn content(&self) -> &str {
        self.content.value()
    }

    /// content的编辑光标(字符下标)
    pub fn content_cursor(&self) -> usize {
        self.content.cursor()
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn deadline(&self) -> &str {
        self.deadline.value()
    }

    /// deadline的编辑光标(字符下标)
    pub fn deadline_cursor(&self) -> usize {
        self.deadline.cursor()
    }

    pub fn priority(&self) -> Priority {
        self.priority
    }

    /// priority栏左右键循环切换优先级
    ///
    /// forward为true时按 High -> Low -> Medium 顺序,反向则倒序
    pub fn cycle_priority(&mut self, forward: bool) {
        self.priority = match (self.priority, forward) {
            // → : High -> Low -> Medium -> High
            (Priority::High, true) => Priority::Low,
            (Priority::Low, true) => Priority::Medium,
            (Priority::Medium, true) => Priority::High,
            // ← : 反向
            (Priority::High, false) => Priority::Medium,
            (Priority::Medium, false) => Priority::Low,
            (Priority::Low, false) => Priority::High,
        };
    }

    /// 返回当前聚焦的单行输入框(content/deadline)
    ///
    /// 多行文本框和priority返回None
    pub fn single_line_mut(&mut self) -> Option<&mut InputLine> {
        match self.focus {
            FormField::Content => Some(&mut self.content),
            FormField::Deadline => Some(&mut self.deadline),
            FormField::Description | FormField::Priority => None,
        }
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
