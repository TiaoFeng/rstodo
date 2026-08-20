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
            AppError::InvalidDeadline {
                input: "ab-cd-ef".to_string()
            }
            .to_string(),
            "Invalid deadline format 'ab-cd-ef', expected {%Y-%m-%d} or {%Y-%m-%dT%H:%M:%S}"
        );

        assert_eq!(
            AppError::InvalidLocalTime.to_string(),
            "Time Conversion Error. It may be due to daylight saving time."
        );

        assert_eq!(
            AppError::NothingToChange.to_string(),
            "Nothing to change, Please enter one or more subcommands"
        );
        assert_eq!(
            AppError::TaskNotFound { no: 1 }.to_string(),
            "Task not found: no 1, run `list` to check current numbers"
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
        let io_err = io_err("test_op", "test1.json".to_string(), err);
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
