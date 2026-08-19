# rstodo

**简体中文** | [English](README.en.md)

[![Latest Release](https://img.shields.io/github/v/release/TiaoFeng/rust-todo-cli)](https://github.com/TiaoFeng/rust-todo-cli/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)

一个使用 Rust 编写的命令行 todo 工具。支持任务的增、删、改、查，以及优先级与截止时间管理。数据以 JSON 文件存储。本项目虽是作者初学 Rust 的练习项目，但目前仍在进一步优化与更新，已经具备比较完善的功能，可以作为日常的轻量化工具使用。

## 特性

- 支持任务的添加、修改、删除、完成/取消完成
- 支持设置任务优先级（High / Medium / Low）与截止时间
- 支持按截止时间或优先级排序展示
- 支持任务详情查看（查看描述信息）
- 基于 JSON 文件实现本地持久化
- 终端使用类似 markdown 样式格式化输出

## 安装

下载预编译的最新版本：
[![Latest Release](https://img.shields.io/github/v/release/TiaoFeng/rust-todo-cli)](https://github.com/TiaoFeng/rust-todo-cli/releases/latest)
或从源码构建（见下方「构建」章节）。

## 快速开始（示例）
```
$ cargo run -- add "提交issue"
$ cargo run -- add "修改代码" -d 2000-1-1
$ cargo run -- add "提交commit" -d 2000-1-2 -D "修复潜在的bug"
$ cargo run -- add "审查代码" -d 2000-1-2T12:30:00 -p high
$ cargo run -- list
| status | no | priority |       deadline      |    task    |    more   |
|--------|----|----------|---------------------|------------|-----------|
|        | 1  |    Low   |          No         | 提交issue  |           |
|        | 2  |    Low   | 2000-01-01 23:59:59 | 修改代码   |           |
|        | 3  |    Low   | 2000-01-02 23:59:59 | 提交commit | Show desc |
|        | 4  |   High   | 2000-01-02 12:30:00 | 审查代码   |           |

$ cargo run -- done 1
$ cargo run -- list
| status | no | priority |       deadline      |    task    |    more   |
|--------|----|----------|---------------------|------------|-----------|
|    ✓   | 1  |    Low   |          No         | 提交issue  |           |
|        | 2  |    Low   | 2000-01-01 23:59:59 | 修改代码   |           |
|        | 3  |    Low   | 2000-01-02 23:59:59 | 提交commit | Show desc |
|        | 4  |   High   | 2000-01-02 12:30:00 | 审查代码   |           |

$ cargo run -- show 3
| status | no | priority |       deadline      |    task    |    more   |
|--------|----|----------|---------------------|------------|-----------|
|        | 3  |    Low   | 2000-01-02 23:59:59 | 提交commit | Show desc |
-Description-
修复潜在的bug

$ cargo run -- change 2 -c "修复bug" -d 2000-1-1T12:00:00 -p medium
$ cargo run -- list
| status | no | priority |       deadline      |    task    |    more   |
|--------|----|----------|---------------------|------------|-----------|
|    ✓   | 1  |    Low   |          No         | 提交issue  |           |
|        | 2  |  Medium  | 2000-01-01 12:00:00 | 修复bug    |           |
|        | 3  |    Low   | 2000-01-02 23:59:59 | 提交commit | Show desc |
|        | 4  |   High   | 2000-01-02 12:30:00 | 审查代码   |           |

$ cargo run -- list p
| status | no | priority |       deadline      |    task    |    more   |
|--------|----|----------|---------------------|------------|-----------|
|        | 1  |   High   | 2000-01-02 12:30:00 | 审查代码   |           |
|        | 2  |  Medium  | 2000-01-01 12:00:00 | 修复bug    |           |
|    ✓   | 3  |    Low   |          No         | 提交issue  |           |
|        | 4  |    Low   | 2000-01-02 23:59:59 | 提交commit | Show desc |

$ cargo run -- list d
| status | no | priority |       deadline      |    task    |    more   |
|--------|----|----------|---------------------|------------|-----------|
|        | 1  |  Medium  | 2000-01-01 12:00:00 | 修复bug    |           |
|        | 2  |   High   | 2000-01-02 12:30:00 | 审查代码   |           |
|        | 3  |    Low   | 2000-01-02 23:59:59 | 提交commit | Show desc |
|    ✓   | 4  |    Low   |          No         | 提交issue  |           |

$ cargo run -- delete 1
$ cargo run -- list
| status | no | priority |       deadline      |    task    |    more   |
|--------|----|----------|---------------------|------------|-----------|
|        | 1  |   High   | 2000-01-02 12:30:00 | 审查代码   |           |
|        | 2  |    Low   | 2000-01-02 23:59:59 | 提交commit | Show desc |
|    ✓   | 3  |    Low   |          No         | 提交issue  |           |

```

## 命令说明
### 全局命令
所有命令都支持以下全局参数：
```
--file <FILE> 指定要操作的 JSON 文件
```
默认file地址：

|平台|路径|
|---|---|
|Linux|~/.local/share/rstodo/task.json|
|macOS|~/Library/Application Support/rstodo/task.json|
|Windows|C:\Users\<user>\AppData\Local\rstodo\task.json|

全局参数可以放在命令的任何位置：

```
# 放在命令前
rstodo --file ./test1_commands.json list

# 放在命令后
rstodo list --file ./test1_commands.json
```

### 子命令
#### 1. 添加 task 项目
```
rstodo add "{content}" -d {%Y-%m-%dT%h:%m:%s} -D "{description}" -p {priority}
```
可选参数（description，deadline）
```
-D "{description}"
-d {%Y-%m-%d}
-d {%Y-%m-%dT%h:%m:%s}
-p {priority}    # 注释：包括high, medium, low, 可以输入数字1,2,3 默认low
```
#### 2. 修改 task 条目
```
rstodo change {no} -c "{content}" -D "{description}" -d {%Y-%m-%dT%h:%m:%s} -p {priority}
```
可选参数（content，description，deadline）
```
-c "{content}"
-D    # 注释：-D 代表清空description
-D "{description}"
-d    # 注释：-d 代表清空deadline
-d {%Y-%m-%d}
-d {%Y-%m-%dT%h:%m:%s}
-p   # 注释：-p 代表清空priority, 默认为low
-p {priority}    # 注释：包括high, medium, low, 可以输入数字1,2,3
```
#### 3. 展示 todo 清单
```
rstodo list {SortBy}
```
可选参数（SortBy）
```
{SortBy}    # 注释：SortBy包括"d"（按deadline排序），"p"（按priority排序）
```
#### 4. 详细展示条目
```
rstodo show {no}
```
#### 5. 标记 task 完成
```
rstodo done {no}
```
#### 6. 标记 task 未完成
```
rstodo undone {no}
```
#### 7. 删除 task 条目
```
rstodo delete {no}
```

## 从源代码构建
```bash
git clone https://github.com/TiaoFeng/rust-todo-cli.git
cd rust-todo-cli
cargo build --release
```

## 项目结构

```
src/
├── main.rs           # CLI 命令解析与主程序入口
├── commands.rs       # add / list / complete / delete 等命令实现
├── task.rs           # Task 结构体与相关方法
├── time.rs           # 时间格式转换与时区处理
├── error.rs          # 项目中使用的错误类型
└── io/
    ├── storage.rs     # Todo list 的读取与保存
    └── cli_print.rs   # 基于 comfy_table 的终端表格输出
```

## License

本项目使用 [MIT License](LICENSE)。

## 声明与鸣谢

- Claude Sonnet 5、DeepSeek V4 Flash、DeepSeek V4 Pro、Mimo 2.5 Pro 和 KIMI K3 提供代码审查和技术指导。
- [opencode](https://github.com/anomalyco/opencode) 提供优秀、开源的工具。
