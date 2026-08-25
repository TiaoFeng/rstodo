//! test帮助函数
//!
//! 包括：
//! - TempGuard用于创建和清理测试产生的临时文件/目录

use std::{
    fs,
    path::{Path, PathBuf},
    process,
};

pub struct TempGuard {
    main: PathBuf,
    backup: PathBuf,
}

impl TempGuard {
    pub fn new(name: &str) -> Self {
        let main =
            std::env::temp_dir().join(format!("{}_rstodo_test_{}.json", process::id(), name));

        let backup = PathBuf::from(format!("{}.bak", main.to_string_lossy()));

        clean(&main);
        clean(&backup);

        TempGuard { main, backup }
    }

    /// 传String给TaskStore::new用
    pub fn main_path(&self) -> String {
        self.main.to_string_lossy().to_string()
    }
}

fn clean(path: &Path) {
    if fs::remove_file(path).is_err() {
        let _ = fs::remove_dir_all(path);
    }
}

/// 在Drop的时候顺带清理文件，
/// 就算测试发生了panic也会把测试文件清除
impl Drop for TempGuard {
    fn drop(&mut self) {
        clean(&self.main);
        clean(&self.backup);
    }
}
