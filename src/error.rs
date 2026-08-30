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

/// 单元测试
#[cfg(test)]
mod error_test {
    use super::*;
    use std::error::Error;

    fn create_io_error() -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::NotFound, "no such file")
    }

    fn create_serde_error() -> serde_json::Error {
        serde_json::from_str::<serde_json::Value>("{abcdefg}").unwrap_err()
    }

    #[test]
    fn test_display() {
        let src = create_io_error();
        let io = AppError::Io {
            operation: "write to file",
            path: "test1.json".to_string(),
            source: src,
        };
        assert_eq!(
            io.to_string(),
            "failed to write to file 'test1.json': no such file"
        );

        let corrupted = AppError::Corrupted {
            path: "test2.json".to_string(),
            source: create_serde_error(),
        };
        assert_eq!(
            corrupted.to_string(),
            format!(
                "task file 'test2.json' is corrupted: {}",
                corrupted.source().unwrap()
            )
        );

        assert_eq!(
            AppError::InvalidContent {
                input: "   ".to_string()
            }
            .to_string(),
            "Invalid content: '   '. The 'content' field cannot be left blank."
        );

        assert_eq!(
            AppError::InvalidDescription {
                input: "  ".to_string()
            }
            .to_string(),
            "Invalid description: '  '. The 'description' field cannot be left blank."
        );

        assert_eq!(
            AppError::InvalidDeadline {
                input: "ab-cd-ef".to_string()
            }
            .to_string(),
            "Invalid deadline format 'ab-cd-ef', expected {%Y-%m-%d} or {%Y-%m-%dT%H:%M:%S}. Example: 2000-1-1 or 2000-1-1T12:00:00"
        );

        assert_eq!(
            AppError::InvalidLocalTime.to_string(),
            "Time Conversion Error. It may be due to daylight saving time."
        );

        assert_eq!(
            AppError::NothingToChange.to_string(),
            "Nothing to change, Please enter one or more subcommands"
        );

        assert_eq!(AppError::NothingToDelete.to_string(), "Nothing to delete");

        assert_eq!(AppError::NothingToUndo.to_string(), "Nothing to undo");

        assert_eq!(
            AppError::TaskNotFound { no: 1 }.to_string(),
            "Task not found: no 1, run `list` to check current numbers"
        );

        assert_eq!(
            AppError::UndoConflict.to_string(),
            "Task list changed since the undo preview, please run 'undo' again",
        );

        assert_eq!(
            AppError::DeleteConflict.to_string(),
            "Task list changed since the delete preview, please run 'delete alldone' again",
        );

        assert_eq!(
            AppError::DeleteConflictOperations.to_string(),
            "You cannot enter both a serial number and 'alldone' at the same time."
        )
    }

    #[test]
    fn test_tui_display() {
        let src = create_io_error();
        let io = AppError::Io {
            operation: "write to file",
            path: "test1.json".to_string(),
            source: src,
        };
        assert_eq!(io.pack_to_tui_err().to_string(), "failed to write to file");

        assert_eq!(
            AppError::InvalidContent {
                input: "   ".to_string()
            }
            .pack_to_tui_err()
            .to_string(),
            "The 'content' field cannot be left blank."
        );

        assert_eq!(
            AppError::InvalidDescription {
                input: "  ".to_string()
            }
            .pack_to_tui_err()
            .to_string(),
            "The 'description' field cannot be left blank."
        );

        assert_eq!(
            AppError::InvalidDeadline {
                input: "ab-cd-ef".to_string()
            }
            .pack_to_tui_err()
            .to_string(),
            "Invalid deadline. Example: 2000-1-1 or 2000-1-1T12:00:00"
        );

        assert_eq!(
            AppError::NothingToChange.pack_to_tui_err().to_string(),
            "Nothing to change"
        );

        assert_eq!(
            AppError::TaskNotFound { no: 1 }
                .pack_to_tui_err()
                .to_string(),
            "Task not found: no 1"
        );

        assert_eq!(
            AppError::UndoConflict.pack_to_tui_err().to_string(),
            "Task list changed, please run 'undo' again"
        );

        assert_eq!(
            AppError::DeleteConflict.pack_to_tui_err().to_string(),
            "Task list changed, please run 'delete alldone' again"
        );

        assert_eq!(
            AppError::Tui(create_io_error())
                .pack_to_tui_err()
                .to_string(),
            "An error occurred in tui: no such file"
        );

        assert_eq!(
            AppError::NothingToUndo.pack_to_tui_err().to_string(),
            "Nothing to undo"
        );
    }

    #[test]
    fn test_source() {
        assert!(
            AppError::Corrupted {
                path: "test1.json".to_string(),
                source: create_serde_error()
            }
            .source()
            .is_some()
        );
        assert!(
            AppError::Io {
                operation: "test_op",
                path: "test2.json".to_string(),
                source: create_io_error()
            }
            .source()
            .is_some()
        );
        assert!(
            AppError::InvalidDeadline {
                input: "input1".to_string()
            }
            .source()
            .is_none()
        );
        assert!(AppError::InvalidLocalTime.source().is_none());
        assert!(AppError::NothingToChange.source().is_none());
        assert!(AppError::TaskNotFound { no: 1 }.source().is_none());
    }

    #[test]
    fn pub_trait() {
        let err = create_io_error();
        let io_err = io_err("test_op", Path::new("test1.json"), err);
        match io_err {
            AppError::Io {
                operation,
                path,
                source,
            } => {
                assert_eq!(operation, "test_op");
                assert_eq!(path, "test1.json".to_string());
                assert_eq!(source.to_string(), create_io_error().to_string());
            }
            _ => unreachable!(),
        }
    }
}
