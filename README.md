# rust-todo-cli-demo
这是一个新手入门Rust程式语言的第一个小项目。演示项目实现了一个简单的 Todo CLI + 使用 json 文件持久化 Todo 清单。通过这个项目可以学习基本的程序设计、测试与错误处理。

## 构建
```text
git clone https://github.com/TiaoFeng/rust-todo-cli-demo.git

cd rust-todo-cli-demo

cargo build --release
```

## 示例
```
$ cargo run -- add "提交issue"
$ cargo run -- add "修改代码"
$ cargo run -- list
status| id | task
  [ ]   1    提交issue
  [ ]   2    修改代码
$ cargo run -- done 1
$ cargo run -- list
status| id | task
  [✓]   1    提交issue
  [ ]   2    修改代码
```

## 指令
添加 todo 项目
```text
./target/release/rust-todo-cli-demo -- add "{content}"
```

展示 todo 清单
```text
./target/release/rust-todo-cli-demo -- list
```

标记 todo 完成
```text
./target/release/rust-todo-cli-demo -- done {id}
```

删除 todo 条目
```text
./target/release/rust-todo-cli-demo -- delete {id}
```

## 项目结构
```
- src/main.rs       cli命令读取实现与主程序
- src/commands.rs   实现add,list,complete,delete四种命令
- src/storage.rs    实现todo list读取和保存
- src/task.rs       实现Task struct和Task的方法
```

## License

本项目使用 [MIT License](LICENSE)。

## 鸣谢

- Claude Sonnet 5 和 DeepSeek V4 Flash 提供技术指导。
