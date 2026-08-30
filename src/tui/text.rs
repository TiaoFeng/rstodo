//! 文本编辑的辅助函数
//!
//! 字符串下标均按字符计

use ratatui::text::Line;
use unicode_segmentation::UnicodeSegmentation;

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
pub fn byte_idx(s: &str, char_idx: usize) -> usize {
    s.char_indices().nth(char_idx).map_or(s.len(), |(i, _)| i)
}

pub fn range_width(s: &str, start: usize, end: usize) -> usize {
    let text: String = s.chars().skip(start).take(end - start).collect();
    Line::from(text).width()
}

pub fn cursor_at_width(s: &str, start: usize, end: usize, target_width: usize) -> usize {
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
    let row = rows
        .iter()
        .rposition(|&(start, _)| start <= idx)
        .unwrap_or(0);
    (row, idx - rows[row].0)
}
