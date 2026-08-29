//! 颜色风格主题
//!
//! 现代简约色块

use ratatui::style::{Color, Modifier, Style};

/// 深色背景 + 纯色色块高亮(橙),标题使用紫色强调
pub struct Theme {
    /// 全局背景
    pub base: Color,
    /// 弹窗背景
    pub surface: Color,
    /// 边框
    pub border: Color,
    /// 主文本
    pub text: Color,
    /// 次要文本(提示、placeholder)
    pub muted: Color,
    /// 选中高亮(大时钟)
    pub peach: Color,
    /// 小标题
    pub purple: Color,
    /// 完成(绿)
    pub green: Color,
    /// 逾期/高优先级(红)
    pub red: Color,
    /// 中优先级(黄)
    pub yellow: Color,
}

pub const THEME: Theme = Theme {
    base: Color::Rgb(20, 20, 20),      // 背景
    surface: Color::Rgb(30, 30, 30),   // 弹框背景
    border: Color::Rgb(100, 100, 100), // 边框
    text: Color::Rgb(240, 240, 240),   // 文本
    muted: Color::Rgb(180, 180, 180),  // 次要文本
    peach: Color::Rgb(245, 169, 184),  // 选中高亮
    purple: Color::Rgb(91, 206, 250),  // 小标题
    green: Color::Rgb(120, 190, 32),   // 完成
    red: Color::Rgb(203, 51, 59),      // 逾期，高优先级
    yellow: Color::Rgb(255, 199, 44),  // 中优先级
};

impl Theme {
    /// 全局背景样式
    pub fn base_style(&self) -> Style {
        Style::default().bg(self.base).fg(self.text)
    }

    /// 弹窗背景样式
    pub fn surface_style(&self) -> Style {
        Style::default().bg(self.surface).fg(self.text)
    }

    /// 选中项色块高亮样式
    pub fn highlight(&self) -> Style {
        Style::default()
            .bg(self.peach)
            .fg(self.base)
            .add_modifier(Modifier::BOLD)
    }

    /// 区块标题样式
    pub fn title(&self) -> Style {
        Style::default()
            .fg(self.purple)
            .add_modifier(Modifier::BOLD)
    }

    /// 次要文本样式
    pub fn muted(&self) -> Style {
        Style::default().fg(self.muted)
    }
}
