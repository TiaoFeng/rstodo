# rust-todo-cli
这是一个新手入门Rust程式语言的第一个小项目。演示项目实现了一个简单的 Todo CLI + 使用 json 文件持久化 Todo 清单。通过这个项目可以学习基本的程序设计、测试与错误处理。

## 构建
```
git clone https://github.com/TiaoFeng/rust-todo-cli.git
```
```
cd rust-todo-cli
```
```
cargo build --release
```

## 示例
```
$ cargo run -- add "提交issue"
$ cargo run -- add "修改代码" -d 2000-1-1
$ cargo run -- add "提交commit" -d 2000-1-2 -D "修复潜在的bug"
$ cargo run -- add "审查代码" -d 2000-1-2T12:30:00 -p high
$ cargo run -- list
| status | no | priority |       deadline      |    task    |    more   |
|--------|----|----------|---------------------|------------|-----------|
|        | 1  |    Low   |          No         | 提交issue  |           |
|        | 2  |    Low   | 2000-01-01 00:00:00 | 修改代码   |           |
|        | 3  |    Low   | 2000-01-02 00:00:00 | 提交commit | Show desc |
|        | 4  |   High   | 2000-01-02 12:30:00 | 审查代码   |           |

$ cargo run -- done 1
$ cargo run -- list
| status | no | priority |       deadline      |    task    |    more   |
|--------|----|----------|---------------------|------------|-----------|
|    ✓   | 1  |    Low   |          No         | 提交issue  |           |
|        | 2  |    Low   | 2000-01-01 00:00:00 | 修改代码   |           |
|        | 3  |    Low   | 2000-01-02 00:00:00 | 提交commit | Show desc |
|        | 4  |   High   | 2000-01-02 12:30:00 | 审查代码   |           |

$ cargo run -- show 3
| status | no | priority |       deadline      |    task    |    more   |
|--------|----|----------|---------------------|------------|-----------|
|        | 3  |    Low   | 2000-01-02 00:00:00 | 提交commit | Show desc |
-Description-
修复潜在的bug

$ cargo run -- change 2 -c "修复bug" -d 2000-1-1T12:00:00 -p medium
$ cargo run -- list
| status | no | priority |       deadline      |    task    |    more   |
|--------|----|----------|---------------------|------------|-----------|
|    ✓   | 1  |    Low   |          No         | 提交issue  |           |
|        | 2  |  Medium  | 2000-01-01 12:00:00 | 修复bug    |           |
|        | 3  |    Low   | 2000-01-02 00:00:00 | 提交commit | Show desc |
|        | 4  |   High   | 2000-01-02 12:30:00 | 审查代码   |           |

$ cargo run -- list p
| status | no | priority |       deadline      |    task    |    more   |
|--------|----|----------|---------------------|------------|-----------|
|        | 1  |   High   | 2000-01-02 12:30:00 | 审查代码   |           |
|        | 2  |  Medium  | 2000-01-01 12:00:00 | 修复bug    |           |
|    ✓   | 3  |    Low   |          No         | 提交issue  |           |
|        | 4  |    Low   | 2000-01-02 00:00:00 | 提交commit | Show desc |

$ cargo run -- list -s d
| status | no | priority |       deadline      |    task    |    more   |
|--------|----|----------|---------------------|------------|-----------|
|        | 1  |  Medium  | 2000-01-01 12:00:00 | 修复bug    |           |
|        | 2  |    Low   | 2000-01-02 00:00:00 | 提交commit | Show desc |
|        | 3  |   High   | 2000-01-02 12:30:00 | 审查代码   |           |
|    ✓   | 4  |    Low   |          No         | 提交issue  |           |

$ cargo run -- delete 1
$ cargo run -- list
| status | no | priority |       deadline      |    task    |    more   |
|--------|----|----------|---------------------|------------|-----------|
|        | 1  |    Low   | 2000-01-02 00:00:00 | 提交commit | Show desc |
|        | 2  |   High   | 2000-01-02 12:30:00 | 审查代码   |           |
|    ✓   | 3  |    Low   |          No         | 提交issue  |           |

```

## 指令
#### 1. 添加 task 项目
```
./target/release/rust-todo-cli -- add "{content}" -d {%Y-%m-%dT%h:%m:%s} -D "{description}" -p {priority}
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
./target/release/rust-todo-cli -- change {no} -c "{content}" -D "{description}" -d {%Y-%m-%dT%h:%m:%s} -p {priority}
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
```text
./target/release/rust-todo-cli -- list {SortBy}
```
可选参数（SortBy）
```
{SortBy}    #注释：SortBy包括"d"（按deadline排序），"p"（按priority排序）
```
#### 4. 详细展示条目
```text
./target/release/rust-todo-cli -- show {no}
```

#### 5. 标记 task 完成
```text
./target/release/rust-todo-cli -- done {no}
```

#### 6. 标记 task 未完成
```text
./target/release/rust-todo-cli -- undone {no}
```

#### 7. 删除 task 条目
```text
./target/release/rust-todo-cli -- delete {no}
```

## 项目结构
```
- src/main.rs           cli命令读取实现与主程序
- src/commands.rs       实现add,list,complete,delete四种命令
- src/io/storage.rs     实现todo list读取和保存
- src/io/cli_print.rs   调用comfy_table实现返回Table供打印
- src/task.rs           实现Task struct和Task的方法
- src/time.rs           实现时间时区转换与时间输入格式转换
- src/error.rs          项目中使用的错误类型
```

## License

本项目使用 [MIT License](LICENSE)。

## 鸣谢

- Claude Sonnet 5 、 DeepSeek V4 Flash 和 DeepSeek V4 Pro 提供技术指导。
- [opencode](https://github.com/anomalyco/opencode) 提供优秀、开源的工具。
