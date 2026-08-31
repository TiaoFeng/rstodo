//! error.rs单元测试
#[cfg(test)]
mod tests {
    use crate::error::*;
    use std::{error::Error, path::Path};

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
