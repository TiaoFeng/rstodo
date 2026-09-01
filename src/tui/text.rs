//! 文本编辑的辅助函数
//!
//! 字符串下标均按字符计

use ratatui::text::Line;
use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};

/// 单行输入框: 文本与其编辑光标
///
/// 文本与光标必须同时更新才能保证"光标始终指向文本中的某个字符边界"这一不变量,
/// 由本类型统一封装: 外部只能读取,所有编辑都经过方法
#[derive(Clone)]
pub struct InputLine {
    value: String,
    cursor: usize,
}

impl InputLine {
    /// 以给定文本创建输入框,光标置于末尾
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        let cursor = value.chars().count();
        InputLine { value, cursor }
    }

    /// 在光标处插入字符
    pub fn insert(&mut self, c: char) {
        insert_at_cursor(&mut self.value, &mut self.cursor, c);
    }

    /// 删除光标前的一个字素簇
    pub fn backspace(&mut self) {
        backspace_at_cursor(&mut self.value, &mut self.cursor);
    }

    /// 光标左移一个字素簇
    pub fn left(&mut self) {
        move_cursor_left(&self.value, &mut self.cursor);
    }

    /// 光标右移一个字素簇
    pub fn right(&mut self) {
        move_cursor_right(&self.value, &mut self.cursor);
    }

    /// 光标移到行首
    pub fn home(&mut self) {
        self.cursor = 0;
    }

    /// 光标移到行尾
    pub fn end(&mut self) {
        self.cursor = self.value.chars().count();
    }

    /// 清空输入内容
    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }
}

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
    let byte_pos = byte_idx(s, *cursor);
    let mut gc = GraphemeCursor::new(byte_pos, s.len(), true);
    if let Ok(Some(prev_boundary)) = gc.prev_boundary(s, 0) {
        s.replace_range(prev_boundary..byte_pos, "");
        *cursor = s[..prev_boundary].chars().count();
    }
}

/// 将光标向左移动一个字素簇
pub fn move_cursor_left(s: &str, cursor: &mut usize) {
    if *cursor == 0 {
        return;
    }
    let byte_pos = byte_idx(s, *cursor);
    let mut gc = GraphemeCursor::new(byte_pos, s.len(), true);
    if let Ok(Some(prev_boundary)) = gc.prev_boundary(s, 0) {
        *cursor = s[..prev_boundary].chars().count();
    }
}

/// 将光标向右移动一个字素簇
pub fn move_cursor_right(s: &str, cursor: &mut usize) {
    let byte_pos = byte_idx(s, *cursor);
    let mut gc = GraphemeCursor::new(byte_pos, s.len(), true);
    if let Ok(Some(next_boundary)) = gc.next_boundary(s, 0) {
        *cursor = s[..next_boundary].chars().count();
    }
}

/// 第char_idx个字符对应的字节下标
pub fn byte_idx(s: &str, char_idx: usize) -> usize {
    s.char_indices().nth(char_idx).map_or(s.len(), |(i, _)| i)
}

/// 计算从start到end的宽度（上下移动光标保持对齐）
pub fn range_width(s: &str, start: usize, end: usize) -> usize {
    let start_byte = byte_idx(s, start);
    let end_byte = byte_idx(s, end);
    Line::from(&s[start_byte..end_byte]).width()
}

/// 计算从start开始end结束，不超过target_width的最大位置（上下移动光标保持对齐）
pub fn cursor_at_width(s: &str, start: usize, end: usize, target_width: usize) -> usize {
    let start_byte = byte_idx(s, start);
    let end_byte = byte_idx(s, end);
    let slice = &s[start_byte..end_byte];
    let mut cursor = start;
    let mut width = 0;
    for grapheme in slice.graphemes(true) {
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
pub fn wrap_rows(s: &str, width: usize) -> Vec<(usize, usize)> {
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
pub fn cursor_row_col(rows: &[(usize, usize)], idx: usize) -> (usize, usize) {
    let row = rows.partition_point(|&(start, _)| start <= idx);
    let row = if row == 0 { 0 } else { row - 1 };
    (row, idx - rows[row].0)
}
