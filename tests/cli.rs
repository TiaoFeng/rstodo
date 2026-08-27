//! CLI外部测试模块
//!
//! 用于模拟用户端测试CLI是否正常工作
use std::{
    fs,
    io::Write,
    process::{Command, Output, Stdio},
};

mod utils;
use utils::temp_guard::TempGuard;

fn run(args: &[&str], file: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rstodo"))
        .arg("--file")
        .arg(file)
        .args(args)
        .output()
        .expect("failed to run rstodo")
}

/// 带有输入的run进程
fn run_with_input(args: &[&str], file: &str, input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rstodo"))
        .arg("--file")
        .arg(file)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn out_tostring(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).unwrap()
}

fn err_tostring(out: &Output) -> String {
    String::from_utf8(out.stderr.clone()).unwrap()
}

#[cfg(test)]
mod tests_add_list {
    use super::*;

    #[test]
    fn test_add_list() {
        let guard = TempGuard::new("add_list");

        let out = run(
            &[
                "add",
                "cli_test1",
                "-D",
                "cli_desc1",
                "-d",
                "2000-1-1",
                "-p",
                "high",
            ],
            &guard.main_path(),
        );
        assert!(out.status.success());
        assert!(out.stderr.is_empty());

        let out = run(&["list"], &guard.main_path());
        assert!(out.status.success());
        assert!(out.stderr.is_empty());
        let out_string = out_tostring(&out);
        assert!(out_string.contains("cli_test1"));
        assert!(out_string.contains("2000-01-01"));
        assert!(out_string.contains("23:59:59"));
        assert!(out_string.contains("High"));
        assert!(out_string.contains("Show desc"));
    }

    #[test]
    fn test_list_empty() {
        let guard = TempGuard::new("empty");
        let out = run(&["list"], &guard.main_path());
        assert!(out_tostring(&out).contains("No tasks"));
    }
}

#[test]
fn test_sort() {
    let guard = TempGuard::new("sort");

    assert!(
        run(
            &["add", "cli_test1", "-D", "cli_desc1", "-p", "low"],
            &guard.main_path()
        )
        .stderr
        .is_empty()
    );
    assert!(
        run(&["add", "cli_test2", "-D", "cli_desc2"], &guard.main_path())
            .stderr
            .is_empty()
    );
    assert!(
        run(&["add", "cli_test3", "-p", "high"], &guard.main_path())
            .stderr
            .is_empty()
    );
    assert!(
        run(&["add", "cli_test4", "-p", "medium"], &guard.main_path())
            .stderr
            .is_empty()
    );

    let out = run(&["list", "p"], &guard.main_path());
    let out_string = out_tostring(&out);
    let high = out_string
        .find("cli_test3")
        .unwrap_or_else(|| panic!("Not found in: \n {}", out_string));
    let mid = out_string
        .find("cli_test4")
        .unwrap_or_else(|| panic!("Not found in: \n {}", out_string));
    let low1 = out_string
        .find("cli_test1")
        .unwrap_or_else(|| panic!("Not found in: \n {}", out_string));
    let low2 = out_string
        .find("cli_test2")
        .unwrap_or_else(|| panic!("Not found in: \n {}", out_string));
    assert!(high < mid && mid < low1 && mid < low2);
}

#[test]
fn test_change_task() {
    let guard = TempGuard::new("change");

    assert!(
        run(&["add", "cli_test1"], &guard.main_path())
            .stderr
            .is_empty()
    );
    assert!(
        run(&["add", "cli_test2"], &guard.main_path())
            .stderr
            .is_empty()
    );

    assert!(
        run(
            &["change", "1", "-c", "cli_test1_change"],
            &guard.main_path()
        )
        .stderr
        .is_empty()
    );
    let out_string = out_tostring(&run(&["list"], &guard.main_path()));
    assert!(out_string.contains("cli_test1_change"));
    assert!(out_string.contains("cli_test2"));

    assert!(
        run(
            &["change", "2", "-d", "2000-1-1", "-p", "high"],
            &guard.main_path()
        )
        .stderr
        .is_empty()
    );
    let out_string = out_tostring(&run(&["list"], &guard.main_path()));
    assert!(out_string.contains("2000-01-01 23:59:59"));
    assert!(out_string.contains("High"));
}

#[test]
fn test_done() {
    let guard = TempGuard::new("test_done");
    assert!(
        run(&["add", "cli_test1"], &guard.main_path())
            .stderr
            .is_empty()
    );
    assert!(
        run(&["add", "cli_test2"], &guard.main_path())
            .stderr
            .is_empty()
    );
    assert!(
        run(&["add", "cli_test3"], &guard.main_path())
            .stderr
            .is_empty()
    );

    assert!(run(&["done", "1"], &guard.main_path()).stderr.is_empty());
    let out_string = out_tostring(&run(&["list"], &guard.main_path()));
    assert!(out_string.contains("✓"));

    assert!(run(&["undone", "1"], &guard.main_path()).stderr.is_empty());
    let out_string = out_tostring(&run(&["list"], &guard.main_path()));
    assert!(!out_string.contains("✓"));

    assert!(
        run(&["done", "1", "3"], &guard.main_path())
            .stderr
            .is_empty()
    );
    let out_string = out_tostring(&run(&["list"], &guard.main_path()));
    assert!(out_string.contains("✓"));
    assert!(run(&["undone", "3"], &guard.main_path()).stderr.is_empty());
    let out_string = out_tostring(&run(&["list"], &guard.main_path()));
    assert!(out_string.contains("✓"));
    assert!(
        run(&["undone", "1", "3", "2"], &guard.main_path())
            .stderr
            .is_empty()
    );
    let out_string = out_tostring(&run(&["list"], &guard.main_path()));
    assert!(!out_string.contains("✓"));
}

#[cfg(test)]
mod tests_delete {
    use super::*;

    #[test]
    fn test_delete() {
        let guard = TempGuard::new("test_delete");
        assert!(
            run(&["add", "cli_test1"], &guard.main_path())
                .stderr
                .is_empty()
        );
        assert!(
            run(&["add", "cli_test2"], &guard.main_path())
                .stderr
                .is_empty()
        );

        assert!(run(&["delete", "2"], &guard.main_path()).stderr.is_empty());
        let out_string = out_tostring(&run(&["list"], &guard.main_path()));
        assert!(out_string.contains("cli_test1"));
        assert!(!out_string.contains("cli_test2"));

        assert!(
            run(&["add", "cli_test3"], &guard.main_path())
                .stderr
                .is_empty()
        );
        assert!(
            run(&["add", "cli_test4"], &guard.main_path())
                .stderr
                .is_empty()
        );
        assert!(
            run(&["add", "cli_test5"], &guard.main_path())
                .stderr
                .is_empty()
        );

        assert!(
            run(
                &["delete", "1", "3", "1", "1", "3", "3"],
                &guard.main_path()
            )
            .stderr
            .is_empty()
        );
        let out_string = out_tostring(&run(&["list"], &guard.main_path()));
        assert!(!out_string.contains("cli_test1"));
        assert!(!out_string.contains("cli_test2"));
        assert!(out_string.contains("cli_test3"));
        assert!(!out_string.contains("cli_test4"));
        assert!(out_string.contains("cli_test5"));
    }

    #[test]
    fn test_delete_alldone() {
        let guard = TempGuard::new("test_delete_alldone");
        assert!(
            run(&["add", "cli_test1"], &guard.main_path())
                .stderr
                .is_empty()
        );
        assert!(
            run(&["add", "cli_test2"], &guard.main_path())
                .stderr
                .is_empty()
        );
        assert!(
            run(&["add", "cli_test3"], &guard.main_path())
                .stderr
                .is_empty()
        );
        assert!(
            run(&["done", "1", "3"], &guard.main_path())
                .stderr
                .is_empty()
        );
        assert!(
            run(&["delete", "--alldone", "-y"], &guard.main_path())
                .stderr
                .is_empty()
        );

        let out_string = out_tostring(&run(&["list"], &guard.main_path()));
        assert!(!out_string.contains("cli_test1"));
        assert!(out_string.contains("cli_test2"));
        assert!(!out_string.contains("cli_test3"));
    }
}

/// 测试undo功能
#[cfg(test)]
mod tests_undo {
    use super::*;

    #[test]
    fn test_undo() {
        let guard = TempGuard::new("undo");

        assert!(
            run(&["add", "cli_test1"], &guard.main_path())
                .stderr
                .is_empty()
        );
        assert!(
            run(&["add", "cli_test2"], &guard.main_path())
                .stderr
                .is_empty()
        );
        let out_string = out_tostring(&run(&["list"], &guard.main_path()));
        assert!(out_string.contains("cli_test1"));
        assert!(out_string.contains("cli_test2"));

        assert!(run(&["undo", "-y"], &guard.main_path()).stderr.is_empty());
        let out_string = out_tostring(&run(&["list"], &guard.main_path()));
        assert!(out_string.contains("cli_test1"));
        assert!(!out_string.contains("cli_test2"));
    }

    #[test]
    fn test_undo_confirm_yes() {
        let guard = TempGuard::new("test_undo_confirm_yes");

        assert!(
            run(&["add", "cli_test1"], &guard.main_path())
                .stderr
                .is_empty()
        );
        assert!(
            run(&["add", "cli_test2"], &guard.main_path())
                .stderr
                .is_empty()
        );
        let out_string = out_tostring(&run(&["list"], &guard.main_path()));
        assert!(out_string.contains("cli_test1"));
        assert!(out_string.contains("cli_test2"));

        assert!(
            run_with_input(&["undo"], &guard.main_path(), "yes")
                .stderr
                .is_empty()
        );
        let out_string = out_tostring(&run(&["list"], &guard.main_path()));
        assert!(out_string.contains("cli_test1"));
        assert!(!out_string.contains("cli_test2"));
    }

    #[test]
    fn test_undo_confirm_no() {
        let guard = TempGuard::new("test_undo_confirm_no");

        assert!(
            run(&["add", "cli_test1"], &guard.main_path())
                .stderr
                .is_empty()
        );
        assert!(
            run(&["add", "cli_test2"], &guard.main_path())
                .stderr
                .is_empty()
        );
        let out_string = out_tostring(&run(&["list"], &guard.main_path()));
        assert!(out_string.contains("cli_test1"));
        assert!(out_string.contains("cli_test2"));

        let out = run_with_input(&["undo"], &guard.main_path(), "no");
        assert!(out.stderr.is_empty());
        assert!(out_tostring(&out).contains("cancelled"));
    }
}

#[cfg(test)]
mod tests_status {
    use super::*;

    fn status_count(out: &Output, item: &str) -> usize {
        let out_string = out_tostring(out);
        let line = out_string
            .lines()
            .find(|l| l.trim_start().starts_with(item))
            .unwrap();
        line.split_whitespace()
            .next_back()
            .unwrap()
            .parse()
            .unwrap()
    }

    #[test]
    fn test_status() {
        let guard = TempGuard::new("test_status");

        let out = run(&["status"], &guard.main_path());
        assert!(out.stderr.is_empty());
        assert_eq!(status_count(&out, "Total"), 0);
        assert_eq!(status_count(&out, "Done"), 0);
        assert_eq!(status_count(&out, "Undone"), 0);

        assert!(
            run(&["add", "cli_test1"], &guard.main_path())
                .stderr
                .is_empty()
        );
        assert!(
            run(&["add", "cli_test2"], &guard.main_path())
                .stderr
                .is_empty()
        );
        assert!(
            run(&["add", "cli_test3"], &guard.main_path())
                .stderr
                .is_empty()
        );
        let out = run(&["status"], &guard.main_path());
        assert!(out.stderr.is_empty());
        assert_eq!(status_count(&out, "Total"), 3);
        assert_eq!(status_count(&out, "Done"), 0);
        assert_eq!(status_count(&out, "Undone"), 3);

        assert!(run(&["done", "1"], &guard.main_path()).stderr.is_empty());
        let out = run(&["status"], &guard.main_path());
        assert!(out.stderr.is_empty());
        assert_eq!(status_count(&out, "Total"), 3);
        assert_eq!(status_count(&out, "Done"), 1);
        assert_eq!(status_count(&out, "Undone"), 2);

        assert!(run(&["delete", "1"], &guard.main_path()).stderr.is_empty());
        assert!(run(&["delete", "1"], &guard.main_path()).stderr.is_empty());
        assert!(run(&["delete", "1"], &guard.main_path()).stderr.is_empty());
        let out = run(&["status"], &guard.main_path());
        assert!(out.stderr.is_empty());
        assert_eq!(status_count(&out, "Total"), 0);
        assert_eq!(status_count(&out, "Done"), 0);
        assert_eq!(status_count(&out, "Undone"), 0);
    }
}

#[cfg(test)]
mod tests_error {
    use super::*;

    #[test]
    fn test_error_basic() {
        let guard = TempGuard::new("test_error_basic");

        // change输入越界的序号
        let out = run(&["change", "99", "-c", "error_change"], &guard.main_path());
        assert_eq!(out.status.code(), Some(1));
        let err = err_tostring(&out);
        assert!(err.contains("Task not found"));

        // delete输入越界的序号
        let out = run(&["delete", "0"], &guard.main_path());
        assert_eq!(out.status.code(), Some(1));
        let err = err_tostring(&out);
        assert!(err.contains("Task not found"));

        // change未输入任何修改
        let out = run(&["change", "1"], &guard.main_path());
        assert_eq!(out.status.code(), Some(1));
        let err = err_tostring(&out);
        assert!(err.contains("Nothing to change"));

        // 输入不合法的日期
        let out = run(&["add", "err_add", "-d", "not-a-date"], &guard.main_path());
        assert_eq!(out.status.code(), Some(1));
        let err = err_tostring(&out);
        assert!(err.contains("{%Y-%m-%d}"));
        assert!(err.contains("{%Y-%m-%dT%H:%M:%S}"));
        assert!(fs::read_to_string(guard.main_path()).unwrap().is_empty());
    }

    #[test]
    fn test_add_change_empty() {
        let guard = TempGuard::new("test_add_change_empty");

        // 输入空的content
        let out = run(&["add", "  "], &guard.main_path());
        assert!(
            err_tostring(&out)
                .contains("Invalid content: '  '. The 'content' field cannot be left blank.")
        );
        // 输入空的description
        let out = run(&["add", "test", "-D", "  "], &guard.main_path());
        assert!(
            err_tostring(&out).contains(
                "Invalid description: '  '. The 'description' field cannot be left blank."
            )
        );

        assert!(
            run(&["add", "cli_test1"], &guard.main_path())
                .stderr
                .is_empty()
        );
        assert!(
            run(&["add", "cli_test2"], &guard.main_path())
                .stderr
                .is_empty()
        );
        // change为空的content
        let out = run(&["change", "1", "-c", "  "], &guard.main_path());
        assert!(
            err_tostring(&out)
                .contains("Invalid content: '  '. The 'content' field cannot be left blank.")
        );
        // change为空的description
        let out = run(&["change", "1", "-D", "  "], &guard.main_path());
        assert!(
            err_tostring(&out).contains(
                "Invalid description: '  '. The 'description' field cannot be left blank."
            )
        );
    }

    #[test]
    fn test_done_undone_delete_multiple() {
        let guard = TempGuard::new("test_done_undone_delete_multiple");

        assert!(
            run(&["add", "cli_test1"], &guard.main_path())
                .stderr
                .is_empty()
        );
        assert!(
            run(&["add", "cli_test2"], &guard.main_path())
                .stderr
                .is_empty()
        );

        // done输入越界的序号
        let out = run(&["done", "1", "2", "3", "4", "5", "3"], &guard.main_path());
        assert_eq!(out.status.code(), Some(1));
        let err = err_tostring(&out);
        assert!(err.contains("Task not found"));

        // undone输入越界的序号
        let out = run(
            &["undone", "1", "2", "3", "4", "5", "3"],
            &guard.main_path(),
        );
        assert_eq!(out.status.code(), Some(1));
        let err = err_tostring(&out);
        assert!(err.contains("Task not found"));

        // delete输入越界的序号
        let out = run(
            &["delete", "1", "2", "3", "4", "5", "3"],
            &guard.main_path(),
        );
        assert_eq!(out.status.code(), Some(1));
        let err = err_tostring(&out);
        assert!(err.contains("Task not found"));

        // 没有序号输入done
        let out = run(&["done"], &guard.main_path());
        let err = err_tostring(&out);
        assert!(err.contains("<NOS>"));

        // 没有序号输入undone
        let out = run(&["undone"], &guard.main_path());
        let err = err_tostring(&out);
        assert!(err.contains("<NOS>"));
    }

    #[test]
    fn test_delete_multiple_alldone() {
        let guard = TempGuard::new("test_delete_multiple_alldone");
        assert!(
            run(&["add", "cli_test1"], &guard.main_path())
                .stderr
                .is_empty()
        );
        assert!(
            run(&["add", "cli_test2"], &guard.main_path())
                .stderr
                .is_empty()
        );
        assert!(
            run(&["add", "cli_test3"], &guard.main_path())
                .stderr
                .is_empty()
        );

        let out = run(&["delete", "1", "2", "--alldone", "-y"], &guard.main_path());
        assert_eq!(out.status.code(), Some(1));
        let err = err_tostring(&out);
        assert!(
            err.contains("You cannot enter both a serial number and 'alldone' at the same time.")
        );

        let out = run(&["delete", "--alldone", "-y"], &guard.main_path());
        let out_string = out_tostring(&out);
        assert!(out_string.contains("+_+ Nothing to delete"));
    }

    /// undo部分错误（其实只是提示）测试
    #[test]
    fn test_repeat_undo() {
        let guard = TempGuard::new("test_repeat_undo");

        // 重复undo
        assert!(
            run(&["add", "cli_test1"], &guard.main_path())
                .stderr
                .is_empty()
        );
        assert!(
            run(&["add", "cli_test2"], &guard.main_path())
                .stderr
                .is_empty()
        );
        assert!(run(&["undo", "-y"], &guard.main_path()).stderr.is_empty());
        let out_string = out_tostring(&run(&["list"], &guard.main_path()));
        assert!(out_string.contains("cli_test1"));
        assert!(!out_string.contains("cli_test2"));
        let out = run(&["undo", "-y"], &guard.main_path());
        assert!(out.stderr.is_empty());
        assert!(out_tostring(&out).contains("Nothing to undo"));
        let out_string = out_tostring(&run(&["list"], &guard.main_path()));
        assert!(out_string.contains("cli_test1"));
        assert!(!out_string.contains("cli_test2"));
    }

    #[test]
    fn test_notexist_undo() {
        let guard = TempGuard::new("test_notexist_undo");

        // 不存在bak文件undo
        assert!(
            run(&["add", "cli_test1"], &guard.main_path())
                .stderr
                .is_empty()
        );
        assert!(
            run(&["add", "cli_test2"], &guard.main_path())
                .stderr
                .is_empty()
        );

        let _ = fs::remove_file(guard.backup_path());
        let out = run(&["undo", "-y"], &guard.main_path());
        assert!(out.stderr.is_empty());
        assert!(out_tostring(&out).contains("Nothing to undo"));
        let out_string = out_tostring(&run(&["list"], &guard.main_path()));
        assert!(out_string.contains("cli_test1"));
        assert!(out_string.contains("cli_test2"));
    }

    #[test]
    fn test_empty_undo() {
        let guard = TempGuard::new("test_empty_undo");

        // bak文件为空
        assert!(
            run(&["add", "cli_test1"], &guard.main_path())
                .stderr
                .is_empty()
        );
        assert!(
            run(&["add", "cli_test2"], &guard.main_path())
                .stderr
                .is_empty()
        );
        fs::write(guard.backup_path(), "").unwrap();
        assert!(fs::read_to_string(guard.backup_path()).unwrap().is_empty());
        let out = run(&["undo", "-y"], &guard.main_path());
        assert!(out.stderr.is_empty());
        assert!(out_tostring(&out).contains("Nothing to undo"));
        let out_string = out_tostring(&run(&["list"], &guard.main_path()));
        assert!(out_string.contains("cli_test1"));
        assert!(out_string.contains("cli_test2"));
    }
}
