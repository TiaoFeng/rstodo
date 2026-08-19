use std::{error::Error, fmt};

#[derive(Debug)]
pub enum AppError {
    Corrupted {
        path: String,
        source: serde_json::Error,
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
    TaskNotFound {
        no: usize,
    },
}

pub fn io_err(operation: &'static str, path: String, err: std::io::Error) -> AppError {
    AppError::Io {
        operation,
        path,
        source: err,
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Corrupted { path, source } => {
                write!(f, "task file '{}' is corrupted: {}", path, source)
            }
            AppError::InvalidDeadline { input } => {
                write!(
                    f,
                    "Invalid deadline format '{}', expected {{%Y-%m-%d}} or {{%Y-%m-%dT%H:%M:%S}}",
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
            AppError::TaskNotFound { no } => {
                write!(
                    f,
                    "Task not found: no {}, run `list` to check current numbers",
                    no
                )
            }
        }
    }
}

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
