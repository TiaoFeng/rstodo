# rust-todo-cli-demo
这是一个新手入门Rust程式语言的第一个小项目。演示项目实现了一个简单的 Todo CLI + 使用 json 文件持久化 Todo 清单。通过这个项目可以学习基本的程序设计、测试与错误处理。

## 构建
```
git clone https://github.com/TiaoFeng/rust-todo-cli-demo.git
```
```
cd rust-todo-cli-demo
```
```
cargo build --release
```

## 示例
```
$ cargo run -- add "提交issue"
$ cargo run -- add "修改代码" -d 2000-1-1
$ cargo run -- list
status| id |         deadline         | task
  [ ]   1             No                提交issue
  [ ]   2      2000-01-01 00:00:00      修改代码
$ cargo run -- done 1
$ cargo run -- list
status| id |         deadline         | task
  [✓]   1             No                提交issue
  [ ]   2      2000-01-01 00:00:00      修改代码
$ cargo run -- change 2 -c "修复bug" -d 2000-1-1T12:00:00
$ cargo run -- list
status| id |         deadline         | task
  [✓]   1             No                提交issue
  [ ]   2      2000-01-01 12:00:00      修复bug
```

## 指令
#### 1. 添加 task 项目
```
./target/release/rust-todo-cli-demo -- add "{content}" -d {%Y-%m-%dT%h:%m:%s}
```
可选参数（deadline）
```
-d {%Y-%m-%d}
-d {%Y-%m-%dT%h:%m:%s}
```
#### 2. 修改 task 条目
```
./target/release/rust-todo-cli-demo -- change {id} -c "{content}" -d {%Y-%m-%dT%h:%m:%s}
```
可选参数（content，deadline）
```
-c {"content"}
-d    # 注释：-d 代表清空deadline
-d {%Y-%m-%d}
-d {%Y-%m-%dT%h:%m:%s}
```
#### 3. 展示 todo 清单
```text
./target/release/rust-todo-cli-demo -- list
```

#### 4. 标记 task 完成
```text
./target/release/rust-todo-cli-demo -- done {id}
```

#### 5. 标记 task 未完成
```text
./target/release/rust-todo-cli-demo -- undone {id}
```

#### 6. 删除 task 条目
```text
./target/release/rust-todo-cli-demo -- delete {id}
```

## 项目结构
```
- src/main.rs       cli命令读取实现与主程序
- src/commands.rs   实现add,list,complete,delete四种命令
- src/storage.rs    实现todo list读取和保存
- src/task.rs       实现Task struct和Task的方法
- src/time.rs       实现时间时区转换与时间输入格式转换
```

## License

本项目使用 [MIT License](LICENSE)。

## 鸣谢

- Claude Sonnet 5 和 DeepSeek V4 Flash 提供技术指导。
- [opencode](https://github.com/anomalyco/opencode) 提供优秀、开源的工具。
