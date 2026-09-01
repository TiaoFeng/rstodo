//! add / change界面

use crate::{
    task::{Priority, Task},
    time::to_local_time,
    tui::text::{InputLine, TextArea},
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

/// add/change表单弹窗宽度(含左右边框)
pub const FORM_POPUP_WIDTH: u16 = 62;

/// 输入框标签的显示宽度(标签12 + ": " 2)
pub const LABEL_WIDTH: u16 = 14;

/// description输入框的可见行数
pub const DESC_VISIBLE_LINES: usize = 3;

/// description文本区的初始软换行宽度
///
/// 由弹窗布局派生: 弹窗宽 - 左右边框2 - 标签宽; 首帧渲染前作为占位,
/// 渲染时会以实际文本区宽度覆盖(终端过窄时实际宽度更小)
const DESC_INIT_WRAP_WIDTH: usize = (FORM_POPUP_WIDTH - 2 - LABEL_WIDTH) as usize;

/// add / change 表单数据
///
/// 各输入框以字符串保存,保存时再进行解析和转换
/// content/deadline为单行输入框(text::InputLine),description为多行文本框(text::TextArea),
/// 各自的文本与光标/滚动状态由对应类型维护
#[derive(Clone)]
pub struct FormData {
    mode: FormMode,
    content: InputLine,
    description: TextArea,
    deadline: InputLine,
    priority: Priority,
    /// 打开表单时的原始Task快照(仅Change模式,Add模式为None),保存时用于字段级三方合并
    original: Option<Task>,
    /// 当前聚焦的输入框的类型
    pub focus: FormField,
}

impl FormData {
    /// 空白add表单
    pub fn add() -> Self {
        FormData {
            mode: FormMode::Add,
            content: InputLine::new(String::new()),
            description: TextArea::new(String::new(), DESC_VISIBLE_LINES, DESC_INIT_WRAP_WIDTH),
            deadline: InputLine::new(String::new()),
            priority: Priority::default(),
            original: None,
            focus: FormField::Content,
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
        FormData {
            mode: FormMode::Change { no, id: task.id() },
            content: InputLine::new(content),
            description: TextArea::new(description, DESC_VISIBLE_LINES, DESC_INIT_WRAP_WIDTH),
            deadline: InputLine::new(deadline),
            priority: task.priority(),
            original: Some(task.clone()),
            focus: FormField::Content,
        }
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

    /// description多行文本框
    ///
    /// 与content/deadline不同,多行文本框的读取面更广(显示行/光标位置/窗口偏移),
    /// 故返回编辑器本身,取文本用其value()
    pub fn description(&self) -> &TextArea {
        &self.description
    }

    /// description多行文本框,渲染与按键分发均直接操作它
    pub fn description_mut(&mut self) -> &mut TextArea {
        &mut self.description
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

    /// 打开表单时的原始Task快照(仅Change模式有值)
    pub fn original(&self) -> Option<&Task> {
        self.original.as_ref()
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
}
