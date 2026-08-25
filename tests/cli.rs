//! CLI外部测试模块
//!
//! 用于模拟用户端测试CLI是否正常工作
use std::{
    fs,
    process::{Command, Output},
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
    assert!(run(&["done", "1"], &guard.main_path()).stderr.is_empty());
    let out_string = out_tostring(&run(&["list"], &guard.main_path()));
    assert!(out_string.contains("✓"));

    assert!(run(&["undone", "1"], &guard.main_path()).stderr.is_empty());
    let out_string = out_tostring(&run(&["list"], &guard.main_path()));
    assert!(!out_string.contains("✓"));

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
    assert!(run(&["delete", "2"], &guard.main_path()).stderr.is_empty());
    let out_string = out_tostring(&run(&["list"], &guard.main_path()));
    assert!(!out_string.contains("cli_test2"));
    assert!(out_string.contains("cli_test1_change"));
}

/// 测试undo功能
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
