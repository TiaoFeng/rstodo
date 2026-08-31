//! # storage模块
//!
//! 负责读取和保存数据：
//! - 包括主文件和备份文件
//! - 若主文件损坏会尝试读取备份文件
//! - 通过文件锁防止发生并发的读写冲突
use crate::UserInterfaceTypes;
use crate::error::{AppError, io_err};
use crate::task::Task;
use std::cell::RefCell;
use std::fs::File;
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// 文件来源枚举，用于标记读取任务列表的来源
/// - Main 主文件
/// - Backup 备份文件
#[derive(Debug, PartialEq)]
enum Origin {
    Main,
    Backup,
}

/// 存储通知供UI使用take_notice调用并打印
#[derive(Debug, PartialEq)]
pub enum StorageNotice {
    RecoveredFromBackup,
}

/// 核心tasks结构体，负责保存文件地址，实现公开的读写方法
///
/// 包括字段：
/// - main 主文件地址
/// - backup 备份文件地址
/// - types UI类型字段
/// - notice 通知（使用RefCell创造内部可变性，不改变结构体整体可变性）
pub struct TaskStore {
    main: PathBuf,
    backup: PathBuf,
    types: UserInterfaceTypes,
    notice: RefCell<Option<StorageNotice>>,
}

impl TaskStore {
    /// 用于初始化创建TaskStore实例
    ///
    /// ### Args:
    /// custom: `Option<String>` 用户自定义文件保存地址
    pub fn new(custom: Option<String>, types: UserInterfaceTypes) -> Self {
        let main = custom.map(PathBuf::from).unwrap_or_else(Self::default_main);
        let mut backup = main.clone().into_os_string();
        backup.push(".bak");
        Self {
            main,
            backup: PathBuf::from(backup),
            types,
            notice: RefCell::new(None), // 新增时消息为空
        }
    }

    /// 在未传入自定义地址时，利用data_dir生成默认地址，供new使用
    fn default_main() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rstodo")
            .join("task.json")
    }

    /// 返回主文件地址
    pub fn main_path(&self) -> &Path {
        &self.main
    }

    /// 返回备份文件地址
    pub fn backup_path(&self) -> &Path {
        &self.backup
    }

    /// 返回用户交互界面类型
    pub fn interface_type(&self) -> UserInterfaceTypes {
        self.types
    }

    /// 读取并清空当前存储通知（仅供单次消费）
    pub fn take_notice(&self) -> Option<StorageNotice> {
        self.notice.borrow_mut().take()
    }

    /// Tasks只读方法
    ///
    /// 逻辑：
    /// - 若主文件正常（可以正常解析tasks，包含`[]`），只读主文件
    /// - 若主文件不存在，走load_backup_fallback函数尝试读取备份文件
    /// - 若主文件为0字节，走load_backup_fallback函数尝试读取备份文件
    /// - 若主文件损坏，走load_backup_fallback函数尝试读取备份文件，若备份文件为0字节 |`[]` | 损坏，报错而不覆盖
    pub fn load(&self) -> Result<Vec<Task>, AppError> {
        let file = match open_read(self.main_path()) {
            Ok(file) => file,
            Err(err) if is_not_found(&err) => {
                // 将原有load_backup函数输出的警告上升到此处保存notice
                let tasks_from_backup = load_backup_fallback(self.backup_path())?;
                if !tasks_from_backup.is_empty() {
                    // 从备份中读取，且读取的task不是空的，输出提示
                    // 如果备份的task是空的，没有必要提示
                    *self.notice.borrow_mut() = Some(StorageNotice::RecoveredFromBackup);
                }
                return Ok(tasks_from_backup);
            }
            Err(e) => return Err(e),
        };
        lock_share(&file, self.main_path())?;
        match read_task_from(&file, self.main_path()) {
            Ok(Some(tasks)) => Ok(tasks),
            Ok(None) => {
                // 从备份中读取，且读取的task不是空的，输出提示
                let tasks_from_backup = load_backup_fallback(self.backup_path())?;
                if !tasks_from_backup.is_empty() {
                    *self.notice.borrow_mut() = Some(StorageNotice::RecoveredFromBackup);
                }
                Ok(tasks_from_backup)
            }
            Err(AppError::Corrupted { path, source }) => {
                match load_backup_fallback(self.backup_path()) {
                    Ok(tasks) if !tasks.is_empty() => {
                        // 从备份中读取，且读取的task不是空的，输出提示
                        *self.notice.borrow_mut() = Some(StorageNotice::RecoveredFromBackup);
                        Ok(tasks)
                    }
                    _ => Err(AppError::Corrupted { path, source }),
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Tasks更新方法，同步更新备份
    ///
    /// 逻辑：
    /// - 使用load_for_update函数读取(来源，任务列表)
    /// - 若读取的是主文件，将主文件备份到副文件，之后再将操作覆盖到主文件
    pub fn update_with_backup(
        &self,
        f: impl FnOnce(&mut Vec<Task>) -> Result<(), AppError>,
    ) -> Result<(), AppError> {
        self.update(f, true)
    }

    /// Tasks更新方法，不更新备份
    ///
    /// 适用于排序，不改变备份文件，这样undo可以回到排序前的操作
    pub fn update_without_backup(
        &self,
        f: impl FnOnce(&mut Vec<Task>) -> Result<(), AppError>,
    ) -> Result<(), AppError> {
        self.update(f, false)
    }

    /// Task更新方法的实现
    ///
    /// 通过内部的bool开关refresh_backup判断是否执行刷新动作
    fn update(
        &self,
        f: impl FnOnce(&mut Vec<Task>) -> Result<(), AppError>,
        refresh_backup: bool,
    ) -> Result<(), AppError> {
        create_dir(self.main_path())?;

        let main = open_read_write(self.main_path())?;
        let backup = open_read_write(self.backup_path())?;

        lock_private(&main, self.main_path())?;
        lock_private(&backup, self.backup_path())?;

        let (origin, mut tasks) =
            load_for_update(&main, self.main_path(), &backup, self.backup_path())?;
        // 从备份中读取，且读取的task不是`[]`，输出从备份中恢复的提示
        if origin == Origin::Backup && !tasks.is_empty() {
            *self.notice.borrow_mut() = Some(StorageNotice::RecoveredFromBackup);
        }
        f(&mut tasks)?;
        if refresh_backup && origin == Origin::Main {
            copy_main_to_backup(&main, self.main_path(), &backup, self.backup_path())?;
        }

        overwrite(
            &main,
            serialize_tasks(&tasks, self.main_path())?.as_bytes(),
            self.main_path(),
        )?;
        Ok(())
    }

    /// 只读备份文件方法
    ///
    /// 调用load_backup_strict只读备份文件
    pub fn load_backup(&self) -> Result<Vec<Task>, AppError> {
        load_backup_strict(self.backup_path())
    }

    /// 提取备份文件并覆盖主文件
    ///
    /// 将备份文件提取，并通过restore_main_from_backup覆盖到主文件，以实现undo
    ///
    /// snapshot：
    /// - 用于对比undo预览的快照与目前进行操作的backup文件，以免被篡改
    /// - 若snapshot与目前的备份文件不匹配，返回错误UndoConflict
    pub fn restore_backup(&self, snapshot: &[Task]) -> Result<(), AppError> {
        create_dir(self.main_path())?;
        let main = open_read_write(self.main_path())?;
        let backup = open_read_write(self.backup_path())?;
        lock_private(&main, self.main_path())?;
        lock_private(&backup, self.backup_path())?;

        let now = read_task_from(&backup, self.backup_path())?.unwrap_or_default();
        if snapshot == now {
            restore_main_from_backup(&main, self.main_path(), &backup, self.backup_path())
        } else {
            Err(AppError::UndoConflict)
        }
    }
}

// tier1
/// 将读取的字符串切片使用serde_json序列化为`Vec<Task>`
///
/// 只有在文件0字节时返回`Ok(None)`
/// 其余都尝试解析json
fn parse_tasks(text: &str, path: &Path) -> Result<Option<Vec<Task>>, AppError> {
    if text.trim().is_empty() {
        return Ok(None);
    }
    let tasks = serde_json::from_str(text).map_err(|err| AppError::Corrupted {
        path: path.to_string_lossy().to_string(),
        source: err,
    })?;
    Ok(Some(tasks))
}

/// 将`&[Task]`的数组使用serde_json格式化为用于保存的`String`
fn serialize_tasks(tasks: &[Task], path: &Path) -> Result<String, AppError> {
    let data =
        serde_json::to_string_pretty(tasks).map_err(|serde_json_err| AppError::Corrupted {
            path: path.to_string_lossy().to_string(),
            source: serde_json_err,
        })?;
    Ok(data)
}

// tier2
/// 捕获NotFound(未找到需要打开的文件)错误，用于后续特殊处理
fn is_not_found(err: &AppError) -> bool {
    matches!(
        err,
        AppError::Io {
            operation: _,
            path: _,
            source
        } if source.kind() == ErrorKind::NotFound
    )
}

/// 从备份中恢复数据的警告信息
///
/// - cli：长消息
/// - tui：短消息
pub fn recovered_from_backup_msg(backup_path: &Path, types: UserInterfaceTypes) -> String {
    match types {
        UserInterfaceTypes::Cli => {
            format!(
                ">_< Warning: main task file is missing or corrupted, loaded data from backup file '{}'. Changes since the last successful save may be lost.",
                backup_path.display()
            )
        }
        UserInterfaceTypes::Tui => {
            ">_< Warning: Restoring from a backup may result in data loss".to_string()
        }
    }
}

/// 根据地址，创建需要的文件夹
fn create_dir(path: &Path) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| io_err("create dir", path, err))?;
    };
    Ok(())
}

/// 传入文件`&Path`，打开并返回只读的文件句柄，不创建
fn open_read(path: &Path) -> Result<File, AppError> {
    let file = File::options()
        .read(true)
        .write(false)
        .create(false)
        .truncate(false)
        .open(path)
        .map_err(|err| io_err("open read", path, err))?;
    Ok(file)
}

/// 传入文件`&Path`，打开并返回可读写的文件句柄，可创建
fn open_read_write(path: &Path) -> Result<File, AppError> {
    let file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|err| io_err("open read write", path, err))?;
    Ok(file)
}

/// 将传入的`&File`句柄使用lock锁定，用于读写修改
fn lock_private(file: &File, path: &Path) -> Result<(), AppError> {
    file.lock()
        .map_err(|err| io_err("lock private", path, err))?;
    Ok(())
}

/// 将传入的`&File`句柄使用lock_shared锁定，用于只读项目
fn lock_share(file: &File, path: &Path) -> Result<(), AppError> {
    file.lock_shared()
        .map_err(|err| io_err("lock shared", path, err))?;
    Ok(())
}

/// 预读取传入的`&File`句柄中的文件内容，返回文件全文的`String`字符串，用于序列化读取
fn read_string(file: &File, path: &Path) -> Result<String, AppError> {
    let mut f = file
        .try_clone()
        .map_err(|err| io_err("try clone", path, err))?;
    f.seek(SeekFrom::Start(0))
        .map_err(|err| io_err("seek to start", path, err))?;
    let mut text = String::new();
    f.read_to_string(&mut text)
        .map_err(|err| io_err("read to string", path, err))?;
    Ok(text)
}

/// 使用传入的`&[u8]`字符比特，覆写到`&File`传入的文件中
fn overwrite(file: &File, data: &[u8], path: &Path) -> Result<(), AppError> {
    let mut f = file
        .try_clone()
        .map_err(|err| io_err("try clone", path, err))?;
    f.seek(SeekFrom::Start(0))
        .map_err(|err| io_err("seek to start", path, err))?;
    f.set_len(0).map_err(|err| io_err("set len 0", path, err))?;
    f.write_all(data)
        .map_err(|err| io_err("write all", path, err))?;
    f.flush().map_err(|err| io_err("flush", path, err))?;
    Ok(())
}

// tier3
/// 使用read_string读取文件的内容，并通过parse_tasks序列化为`Option<Vec<Task>>`返回
///
/// 由于parse_tasks函数的特性，只有在file为0字节时返回为`Ok(None)`
fn read_task_from(file: &File, path: &Path) -> Result<Option<Vec<Task>>, AppError> {
    let text = read_string(file, path)?;
    let tasks = parse_tasks(&text, path)?;
    Ok(tasks)
}

/// 将主文件通过read_task读取，调用serialize_tasks序列化后，利用overwrite覆写入备份文件
fn copy_main_to_backup(
    main: &File,
    main_path: &Path,
    backup: &File,
    backup_path: &Path,
) -> Result<(), AppError> {
    let tasks = read_task_from(main, main_path)?;
    // 要注意，若tasks.json文件解析出来是0比特，有tasks = None
    // 此时应该往bak文件写入[]的空列表
    let data = serialize_tasks(tasks.as_deref().unwrap_or(&[]), main_path)?;
    overwrite(backup, data.as_bytes(), backup_path)
}

// tier4
/// 从主文件或备份中读取任务列表，返回文件来源和任务列表
///
/// 逻辑：
/// - 若主文件正常（可以解析为Tasks），返回(Main, 主文件任务列表)
/// - 若主文件为0字节或损坏，尝试读取备份文件，若备份文件存在内容（可解析为Tasks），返回(Backup, 备份文件任务列表)
/// - 若主文件为0字节，尝试读取备份文件，若备份文件0字节或`[]`，返回空列表重新开始
/// - 若主文件损坏，尝试读取备份文件，若备份文件为0字节，返回错误
fn load_for_update(
    main: &File,
    main_path: &Path,
    backup: &File,
    backup_path: &Path,
) -> Result<(Origin, Vec<Task>), AppError> {
    match read_task_from(main, main_path) {
        Ok(Some(tasks)) => Ok((Origin::Main, tasks)),
        Ok(None) => match read_task_from(backup, backup_path)? {
            Some(tasks) => Ok((Origin::Backup, tasks)),
            None => Ok((Origin::Main, Vec::new())),
        },
        Err(err) => match read_task_from(backup, backup_path)? {
            Some(tasks) => Ok((Origin::Backup, tasks)),
            None => Err(err),
        },
    }
}

/// 为TaskStore中只读的方法提供主文件为空或损坏后的回落
///
/// 逻辑：
/// - 若备份文件存在可解析为Tasks的内容，抛出警告并返回备份文件中的内容
/// - 若备份文件为0字节或不存在，返回空列表
/// - 若备份文件损坏，返回错误
///
/// 备注：
/// - 返回的`Vec`可以为`[]`若需要分别，由上一级处理
fn load_backup_fallback(backup_path: &Path) -> Result<Vec<Task>, AppError> {
    let backup_file = match open_read(backup_path) {
        Ok(file) => file,
        Err(err) if is_not_found(&err) => return Ok(Vec::new()),
        Err(err) => return Err(err),
    };
    lock_share(&backup_file, backup_path)?;
    match read_task_from(&backup_file, backup_path) {
        Ok(Some(tasks)) => Ok(tasks),
        Ok(None) => Ok(Vec::new()),
        Err(err) => Err(err),
    }
}

/// 为TaskStore中只读的读取备份文件提供实现函数
///
/// 逻辑：
/// - 若备份文件不存在，返回`NothingToUndo`错误
/// - 若备份文件可以解析出Tasks列表，返回备份文件中的任务列表
/// - 其他情况（包括备份为0字节），返回`NothingToUndo`错误
fn load_backup_strict(backup_path: &Path) -> Result<Vec<Task>, AppError> {
    let backup_file = match open_read(backup_path) {
        Ok(file) => file,
        Err(err) if is_not_found(&err) => return Err(AppError::NothingToUndo),
        Err(e) => return Err(e),
    };
    lock_share(&backup_file, backup_path)?;
    match read_task_from(&backup_file, backup_path)? {
        Some(tasks) => Ok(tasks),             // 只要备份的tasks存在就都可以回退
        None => Err(AppError::NothingToUndo), // 只有备份tasks文件不存在为零字节才不能回退
    }
}

/// 为TaskStore中使用备份覆盖主文件的undo操作提供实现函数
///
/// 逻辑：
/// - 读取备份文件中的任务列表，只要备份文件不是0字节，都可以恢复
/// - 如果备份文件是0字节或不存在，返回NothingToUndo
/// - 读取备份文件的字符比特，覆写主文件，完成undo
fn restore_main_from_backup(
    main: &File,
    main_path: &Path,
    backup: &File,
    backup_path: &Path,
) -> Result<(), AppError> {
    let tasks = match read_task_from(backup, backup_path)? {
        Some(t) => t, // 只要备份文件存在都可以备份
        None => return Err(AppError::NothingToUndo),
    };
    let data = serialize_tasks(&tasks, backup_path)?;
    overwrite(main, data.as_bytes(), main_path)
}
