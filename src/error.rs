//! 错误模块
//!
//! 定义了可能出现的错误类型，并实现了错误的输出trait
use crate::UserInterfaceTypes;
use std::{error::Error, fmt, path::Path};

/// 项目错误类型枚举
///
/// 新增ui字段
pub struct UiError<'a> {
    error: &'a AppError,
    ui_type: UserInterfaceTypes,
}

/// 项目错误类型枚举
#[derive(Debug)]
pub enum AppError {
    Corrupted {
        path: String,
        source: serde_json::Error,
    },
    InvalidContent {
        input: String,
    },
    InvalidDescription {
        input: String,
    },
    InvalidDeadline {
        input: String,
    },
    Io {
        operation: &'static str,
        path: String,
        source: std::io::Error,
    },
    InvalidLocalTime,
    NothingToChange,
    NothingToDelete,
    NothingToUndo,
    TaskNotFound {
        no: usize,
    },
    DeleteConflictOperations,
    DeleteConflict,
    UndoConflict,
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
    pub fn with_ui(&self, ui_type: UserInterfaceTypes) -> UiError<'_> {
        UiError {
            error: self,
            ui_type,
        }
    }

    fn fmt_full(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Corrupted { path, source } => {
                write!(f, "task file '{}' is corrupted: {}", path, source)
            }
            AppError::InvalidContent { input } => {
                write!(
                    f,
                    "Invalid content: '{}'. The 'content' field cannot be left blank.",
                    input
                )
            }
            AppError::InvalidDeadline { input } => {
                write!(
                    f,
                    "Invalid deadline format '{}', expected {{%Y-%m-%d}} or {{%Y-%m-%dT%H:%M:%S}}. Example: 2000-1-1 or 2000-1-1T12:00:00",
                    input
                )
            }
            AppError::InvalidDescription { input } => {
                write!(
                    f,
                    "Invalid description: '{}'. The 'description' field cannot be left blank.",
                    input
                )
            }
            AppError::Io {
                operation,
                path,
                source,
            } => {
                write!(f, "failed to {} '{}': {}", operation, path, source)
            }
            AppError::InvalidLocalTime => {
                write!(
                    f,
                    "Time Conversion Error. It may be due to daylight saving time."
                )
            }
            AppError::NothingToChange => {
                write!(f, "Nothing to change, Please enter one or more subcommands")
            }
            AppError::NothingToUndo => {
                write!(f, "Nothing to undo")
            }
            AppError::NothingToDelete => {
                write!(f, "Nothing to delete")
            }
            AppError::TaskNotFound { no } => {
                write!(
                    f,
                    "Task not found: no {}, run `list` to check current numbers",
                    no
                )
            }
            AppError::UndoConflict => {
                write!(
                    f,
                    "Task list changed since the undo preview, please run 'undo' again",
                )
            }
            AppError::DeleteConflict => {
                write!(
                    f,
                    "Task list changed since the delete preview, please run 'delete alldone' again"
                )
            }
            AppError::DeleteConflictOperations => {
                write!(
                    f,
                    "You cannot enter both a serial number and 'alldone' at the same time."
                )
            }
            AppError::Tui(err) => {
                write!(f, "Something wrong: {}", err)
            }
        }
    }

    fn fmt_short(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Corrupted { path, source } => {
                write!(f, "task file '{}' is corrupted: {}", path, source)
            }
            AppError::InvalidContent { input: _ } => {
                write!(f, "The 'content' field cannot be left blank.")
            }
            AppError::InvalidDeadline { input: _ } => {
                write!(
                    f,
                    "Invalid deadline. Example: 2000-1-1 or 2000-1-1T12:00:00",
                )
            }
            AppError::InvalidDescription { input: _ } => {
                write!(f, "The 'description' field cannot be left blank.",)
            }
            AppError::Io {
                operation,
                path: _,
                source: _,
            } => {
                write!(f, "failed to {}", operation)
            }
            AppError::InvalidLocalTime => {
                write!(
                    f,
                    "Time Conversion Error. It may be due to daylight saving time."
                )
            }
            AppError::NothingToChange => {
                write!(f, "Nothing to change")
            }
            AppError::NothingToUndo => {
                write!(f, "Nothing to undo")
            }
            AppError::NothingToDelete => {
                write!(f, "Nothing to delete")
            }
            AppError::TaskNotFound { no } => {
                write!(f, "Task not found: no {}", no)
            }
            AppError::UndoConflict => {
                write!(f, "Task list changed, please run undo alldone again",)
            }
            AppError::DeleteConflict => {
                write!(f, "Task list changed, please run delete alldone again")
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

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        AppError::Tui(value)
    }
}

/// 为 `AppError` 实现 `fmt::Display` trait，用于定义每种错误的输出内容
impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_full(f)
    }
}

impl fmt::Display for UiError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.ui_type {
            UserInterfaceTypes::Cli => self.error.fmt_full(f),
            UserInterfaceTypes::Tui => self.error.fmt_short(f),
        }
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
