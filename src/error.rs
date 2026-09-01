//! 错误模块
//!
//! 定义了可能出现的错误类型，并实现了错误的输出trait

use std::{error::Error, fmt, path::Path};

/// Tui重新包装Error
///
/// 元组
/// - error AppError错误
pub struct TuiError<'a>(&'a AppError);

/// 项目错误类型枚举
#[derive(Debug)]
pub enum AppError {
    // 文件损毁
    Corrupted {
        path: String,
        source: serde_json::Error,
    },
    // 非法的Content
    InvalidContent {
        input: String,
    },
    // 非法的Description
    InvalidDescription {
        input: String,
    },
    // 非法的Deadline
    InvalidDeadline {
        input: String,
    },
    // IO操作错误
    Io {
        operation: &'static str,
        path: String,
        source: std::io::Error,
    },
    // 时间转换错误
    InvalidLocalTime,
    // 没有要修改的内容
    NothingToChange,
    // 没有要删除的内容
    NothingToDelete,
    // 没有要恢复的内容
    NothingToUndo,
    // 输入序号在Task列表中未找到
    TaskNotFound {
        no: usize,
    },
    // 删除参数冲突
    DeleteConflictOperations,
    // 删除文件预览与现状冲突
    DeleteConflict,
    // 恢复文件预览与现状冲突
    UndoConflict,
    // 编辑冲突: 表单保存时任务被其他进程修改了相同字段
    EditConflict,
    // Tui渲染相关错误，仅供Tui使用
    Tui(std::io::Error),
}

/// 用于使用端快速的生成 `AppError::Io` 这种错误类型
pub fn io_err(operation: &'static str, path: &Path, err: std::io::Error) -> AppError {
    AppError::Io {
        operation,
        path: path.to_string_lossy().to_string(),
        source: err,
    }
}

impl AppError {
    /// 为Tui重新包装错误
    pub fn pack_to_tui_err(&self) -> TuiError<'_> {
        TuiError(self)
    }

    /// 错误信息输出
    ///
    /// 使用is_short开关控制是否输出短小的错误信息提示
    fn fmt_with(&self, f: &mut fmt::Formatter<'_>, is_short: bool) -> fmt::Result {
        match self {
            AppError::Corrupted { path, source } => {
                write!(f, "task file '{}' is corrupted: {}", path, source)
            }
            AppError::InvalidContent { input } => {
                if is_short {
                    write!(f, "The 'content' field cannot be left blank.")
                } else {
                    write!(
                        f,
                        "Invalid content: '{}'. The 'content' field cannot be left blank.",
                        input
                    )
                }
            }
            AppError::InvalidDeadline { input } => {
                if is_short {
                    write!(
                        f,
                        "Invalid deadline. Example: 2000-1-1 or 2000-1-1T12:00:00",
                    )
                } else {
                    write!(
                        f,
                        "Invalid deadline format '{}', expected {{%Y-%m-%d}} or {{%Y-%m-%dT%H:%M:%S}}. Example: 2000-1-1 or 2000-1-1T12:00:00",
                        input
                    )
                }
            }
            AppError::InvalidDescription { input } => {
                if is_short {
                    write!(f, "The 'description' field cannot be left blank.")
                } else {
                    write!(
                        f,
                        "Invalid description: '{}'. The 'description' field cannot be left blank.",
                        input
                    )
                }
            }
            AppError::Io {
                operation,
                path,
                source,
            } => {
                if is_short {
                    write!(f, "failed to {}", operation)
                } else {
                    write!(f, "failed to {} '{}': {}", operation, path, source)
                }
            }
            AppError::InvalidLocalTime => {
                write!(
                    f,
                    "Time Conversion Error. It may be due to daylight saving time."
                )
            }
            AppError::NothingToChange => {
                if is_short {
                    write!(f, "Nothing to change")
                } else {
                    write!(f, "Nothing to change, Please enter one or more subcommands")
                }
            }
            AppError::NothingToUndo => {
                write!(f, "Nothing to undo")
            }
            AppError::NothingToDelete => {
                write!(f, "Nothing to delete")
            }
            AppError::TaskNotFound { no } => {
                if is_short {
                    write!(f, "Task not found: no {}", no)
                } else {
                    write!(
                        f,
                        "Task not found: no {}, run `list` to check current numbers",
                        no
                    )
                }
            }
            AppError::UndoConflict => {
                if is_short {
                    write!(f, "Task list changed, please run 'undo' again",)
                } else {
                    write!(
                        f,
                        "Task list changed since the undo preview, please run 'undo' again",
                    )
                }
            }
            AppError::DeleteConflict => {
                if is_short {
                    write!(f, "Task list changed, please run 'delete alldone' again")
                } else {
                    write!(
                        f,
                        "Task list changed since the delete preview, please run 'delete alldone' again"
                    )
                }
            }
            AppError::EditConflict => {
                if is_short {
                    write!(f, "Task changed elsewhere, please reopen the form")
                } else {
                    write!(
                        f,
                        "Task changed since the form was opened, please reopen the form to see the latest values"
                    )
                }
            }
            AppError::DeleteConflictOperations => {
                write!(
                    f,
                    "You cannot enter both a serial number and 'alldone' at the same time."
                )
            }
            AppError::Tui(err) => {
                write!(f, "An error occurred in tui: {}", err)
            }
        }
    }
}

/// 为AppError容纳std:io:Error
///
/// 装下Tui渲染产生的错误
impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        AppError::Tui(value)
    }
}

/// 为 `AppError` 实现 `fmt::Display` trait
///
/// cli走AppError的默认输出，错误信息为长
impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_with(f, false)
    }
}

/// 为`TuiError`实现`fmt::Display` trait
///
/// 重新包装后的Tui错误信息走短的输出
impl fmt::Display for TuiError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt_with(f, true)
    }
}

/// 为 `AppError`实现 Error trait，用于传播错误链
impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            AppError::Corrupted { path: _, source } => Some(source),
            AppError::Io {
                operation: _,
                path: _,
                source,
            } => Some(source),
            _ => None,
        }
    }
}
