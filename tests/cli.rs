use std::{
    fs,
    process::{Command, Output},
};

fn run(args: &[&str], file: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rstodo"))
        .arg("--file")
        .arg(file)
        .args(args)
        .output()
        .expect("failed to run rstodo")
}

fn temp_path(name: &str) -> String {
    std::env::temp_dir()
        .join(format!("rstodo_tests_cli_{}", name))
        .to_string_lossy()
        .to_string()
}

fn out_tostring(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).unwrap()
}

fn err_tostring(out: &Output) -> String {
    String::from_utf8(out.stderr.clone()).unwrap()
}

#[test]
fn test_add_list_data() {
    let file = temp_path("add_list");
    let _ = fs::remove_file(&file);

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
        &file,
    );
    assert!(out.status.success());
    assert!(out.stderr.is_empty());

    let out = run(&["list"], &file);
    assert!(out.status.success());
    assert!(out.stderr.is_empty());
    let out_string = out_tostring(&out);
    assert!(out_string.contains("cli_test1"));
    assert!(out_string.contains("2000-01-01"));
    assert!(out_string.contains("23:59:59"));
    assert!(out_string.contains("High"));
    assert!(out_string.contains("Show desc"));

    let empty_file = temp_path("empty");
    let out = run(&["list"], &empty_file);
    assert!(out_tostring(&out).contains("No tasks"));

    let _ = fs::remove_file(&file);
    let _ = fs::remove_file(&empty_file);
}

#[test]
fn test_sort() {
    let file = temp_path("sort");
    let _ = fs::remove_file(&file);
    assert!(
        run(&["add", "cli_test1", "-D", "cli_desc1", "-p", "low"], &file)
            .stderr
            .is_empty()
    );
    assert!(
        run(&["add", "cli_test2", "-D", "cli_desc2"], &file)
            .stderr
            .is_empty()
    );
    assert!(
        run(&["add", "cli_test3", "-p", "high"], &file)
            .stderr
            .is_empty()
    );
    assert!(
        run(&["add", "cli_test4", "-p", "medium"], &file)
            .stderr
            .is_empty()
    );

    let out = run(&["list", "p"], &file);
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
    let _ = fs::remove_file(&file);
}

#[test]
fn change_task() {
    let file = temp_path("change");
    let _ = fs::remove_file(&file);

    assert!(run(&["add", "cli_test1"], &file).stderr.is_empty());
    assert!(run(&["add", "cli_test2"], &file).stderr.is_empty());
    assert!(run(&["done", "1"], &file).stderr.is_empty());
    let out_string = out_tostring(&run(&["list"], &file));
    assert!(out_string.contains("✓"));

    assert!(run(&["undone", "1"], &file).stderr.is_empty());
    let out_string = out_tostring(&run(&["list"], &file));
    assert!(!out_string.contains("✓"));

    assert!(
        run(&["change", "1", "-c", "cli_test1_change"], &file)
            .stderr
            .is_empty()
    );
    let out_string = out_tostring(&run(&["list"], &file));
    assert!(out_string.contains("cli_test1_change"));

    assert!(out_string.contains("cli_test2"));
    assert!(run(&["delete", "2"], &file).stderr.is_empty());
    let out_string = out_tostring(&run(&["list"], &file));
    assert!(!out_string.contains("cli_test2"));
    assert!(out_string.contains("cli_test1_change"));

    let _ = fs::remove_file(&file);
}

#[test]
fn test_error() {
    let file = temp_path("error");
    let _ = fs::remove_file(&file);

    let out = run(&["change", "99", "-c", "error_change"], &file);
    assert_eq!(out.status.code(), Some(1));
    let err = err_tostring(&out);
    assert!(err.contains("Task not found"));

    let out = run(&["delete", "0"], &file);
    assert_eq!(out.status.code(), Some(1));
    let err = err_tostring(&out);
    assert!(err.contains("Task not found"));

    let out = run(&["change", "1"], &file);
    assert_eq!(out.status.code(), Some(1));
    let err = err_tostring(&out);
    assert!(err.contains("Nothing to change"));

    let out = run(&["add", "err_add", "-d", "not-a-date"], &file);
    assert_eq!(out.status.code(), Some(1));
    let err = err_tostring(&out);
    assert!(err.contains("{%Y-%m-%d}"));
    assert!(err.contains("{%Y-%m-%dT%H:%M:%S}"));
    assert!(fs::read_to_string(&file).unwrap().is_empty());
    let _ = fs::remove_file(&file);
}
