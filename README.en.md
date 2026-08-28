# rstodo

[简体中文](README.md) | **English**

[![Latest Release](https://img.shields.io/github/v/release/TiaoFeng/rstodo)](https://github.com/TiaoFeng/rstodo/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)

A command-line to-do tool written in Rust. It supports adding, deleting, editing, and querying tasks, as well as managing priorities and due dates. Data is stored in JSON files. Although this project began as a practice project for the author while learning Rust, it is still being further optimized and updated. It already offers a fairly comprehensive set of features and can be used as a lightweight tool for everyday tasks.

## Features
- Supports adding, editing, deleting, and marking tasks as completed or uncompleted
- Supports batch operations (delete, mark as complete/unmark as complete, delete all completed items)
- Supports setting task priorities (High / Medium / Low) and due dates
- Supports sorting and displaying tasks by due date or priority
- Supports viewing task details (including descriptions)
- Supports reverting to the previous action
- Supports viewing task status statistics
- Automatically flags overdue and uncompleted items
- Supports searching and filtering tasks by task content, date, priority, completion status, and whether they are overdue
- Implements local persistence using JSON files
- Outputs formatted text in a Markdown-like style in the terminal

## Installation
Download the latest precompiled version:
[![Latest Release](https://img.shields.io/github/v/release/TiaoFeng/rstodo)](https://github.com/TiaoFeng/rstodo/releases/latest)
or build from source (see the “Building” section below).

## Quick Start (Example)
```
$ cargo run -- add "提交issue"
$ cargo run -- add "修改代码" -d 2000-1-1
$ cargo run -- add "提交commit" -d 2000-1-2 -D "修复潜在的bug"
$ cargo run -- add "审查代码" -d 2000-1-2T12:30:00 -p high
$ cargo run -- list

| status | no | priority |        deadline       |    task    |    more   |
|--------|----|----------|-----------------------|------------|-----------|
|        | 1  |    Low   |           No          | 提交issue  |           |
|        | 2  |    Low   | 2000-01-01 23:59:59 ! | 修改代码   |           |
|        | 3  |    Low   | 2000-01-02 23:59:59 ! | 提交commit | Show desc |
|        | 4  |   High   | 2000-01-02 12:30:00 ! | 审查代码   |           |

$ cargo run -- done 1 3
$ cargo run -- list

| status | no | priority |        deadline       |    task    |    more   |
|--------|----|----------|-----------------------|------------|-----------|
|    ✓   | 1  |    Low   |           No          | 提交issue  |           |
|        | 2  |    Low   | 2000-01-01 23:59:59 ! | 修改代码   |           |
|    ✓   | 3  |    Low   |  2000-01-02 23:59:59  | 提交commit | Show desc |
|        | 4  |   High   | 2000-01-02 12:30:00 ! | 审查代码   |           |

$ cargo run -- show 3

| status | no | priority |       deadline      |    task    |    more   |
|--------|----|----------|---------------------|------------|-----------|
|    ✓   | 3  |    Low   | 2000-01-02 23:59:59 | 提交commit | Show desc |
-Description-
修复潜在的bug

$ cargo run -- change 2 -c "修复bug" -d 2000-1-1T12:00:00 -p medium
$ cargo run -- list

| status | no | priority |        deadline       |    task    |    more   |
|--------|----|----------|-----------------------|------------|-----------|
|    ✓   | 1  |    Low   |           No          | 提交issue  |           |
|        | 2  |  Medium  | 2000-01-01 12:00:00 ! | 修复bug    |           |
|    ✓   | 3  |    Low   |  2000-01-02 23:59:59  | 提交commit | Show desc |
|        | 4  |   High   | 2000-01-02 12:30:00 ! | 审查代码   |           |

$ cargo run -- list p

| status | no | priority |        deadline       |    task    |    more   |
|--------|----|----------|-----------------------|------------|-----------|
|        | 1  |   High   | 2000-01-02 12:30:00 ! | 审查代码   |           |
|        | 2  |  Medium  | 2000-01-01 12:00:00 ! | 修复bug    |           |
|    ✓   | 3  |    Low   |           No          | 提交issue  |           |
|    ✓   | 4  |    Low   |  2000-01-02 23:59:59  | 提交commit | Show desc |

$ cargo run -- list d

| status | no | priority |        deadline       |    task    |    more   |
|--------|----|----------|-----------------------|------------|-----------|
|        | 1  |  Medium  | 2000-01-01 12:00:00 ! | 修复bug    |           |
|        | 2  |   High   | 2000-01-02 12:30:00 ! | 审查代码   |           |
|    ✓   | 3  |    Low   |  2000-01-02 23:59:59  | 提交commit | Show desc |
|    ✓   | 4  |    Low   |           No          | 提交issue  |           |

$ cargo run -- status

─────────────────
  Items    Count 
═════════════════
  Total      4   
─────────────────
   Done      2   
─────────────────
  Undone     2   
─────────────────
 Overdue     2   
─────────────────

$ cargo run -- delete 1 2
$ cargo run -- list

| status | no | priority |       deadline      |    task    |    more   |
|--------|----|----------|---------------------|------------|-----------|
|    ✓   | 1  |    Low   | 2000-01-02 23:59:59 | 提交commit | Show desc |
|    ✓   | 2  |    Low   |          No         | 提交issue  |           |

$ cargo run -- delete --alldone -y
$ cargo run -- list

+_+ No tasks
```
## Command Description
### Global Commands
All commands support the following global parameters:
```
--file <FILE> Specify the JSON file to be processed
```
Default file path:

|Platform|Path|
|---|---|
|Linux|~/.local/share/rstodo/task.json|
|macOS|~/Library/Application Support/rstodo/task.json|
|Windows|C:\Users\<user>\AppData\Local\rstodo\task.json|

Global parameters can be placed anywhere in a command:
```
# Place it before the command
rstodo --file ./test1_commands.json list

# Place it after the command
rstodo list --file ./test1_commands.json
```
### Subcommand
#### 1. Add a task
```
rstodo add "{content}" -d {%Y-%m-%dT%H:%M:%S} -D "{description}" -p {priority}
```
Optional Parameters（description，deadline）
```
-D "{description}"
-d {%Y-%m-%d}
-d {%Y-%m-%dT%H:%M:%S}
-p {priority}    # Note: Options include “high,” “medium,”
                 # and “low.” You can enter the numbers 1, 2, or 3. The default is “low.”
```
Example
```
rstodo add "task1" -d 2000-1-1T12:00:00 -D "desc1" -p high
rstodo add "task2" -D "desc2" -p 2
```
#### 2. Edit a task
```
rstodo change {no} -c "{content}" -D "{description}" -d {%Y-%m-%dT%H:%M:%S} -p {priority}
```
Optional Parameters（content，description，deadline）
```
-c "{content}"
-D    # Note: -D clears the description
-D "{description}"
-d    # Note: -d clears the deadline
-d {%Y-%m-%d}
-d {%Y-%m-%dT%H:%M:%S}
-p   # Note: -p clears the priority; the default is “low.”
-p {priority}    # Note: Includes “high,” “medium,” and “low”;
                 # you can enter the numbers 1, 2, or 3.
```
Example
```
rstodo change 1 -c "change content" -d 2000-1-1 -D "change desc" -p high
rstodo change 1 -c "change content" -d -p high
rstodo change 1 -c "change content" -D
```
#### 3. Display task list
```
rstodo list {SortBy} {-f [keyword]}
```
Optional Parameters（SortBy, -f [keyword]）
```
{SortBy}    # Note: SortBy includes “d” (sort by deadline) and “p” (sort by priority).
-f [keyword]  # Note: Search and filter
              # Keywords include: done (completed), undone or todo (unfinished),
              # overdue (overdue), other fields that appear in the task
```
Example
```
rstodo list p          # Sort by priority
rstodo list d          # Sort by Due Date
rstodo list -f done    # Search and filter all completed tasks
rstodo list -f example_content  # Search and filter for tasks
                                # where this keyword appears in the content, description, etc.
rstodo list -f 2000    # Search for and filter tasks from the year 2000
rstodo list -f 01-01   # Search for and filter tasks from January 1
rstodo list p -f todo  # While filtering, sort the results by priority.
```
#### 4. View task details
```
rstodo show {no}
```
#### 5. Done a task
```
rstodo done {nos}
```
Example
```
rstodo done 1 2 3
```
#### 6. Undone a task
```
rstodo undone {nos}
```
Example
```
rstodo undone 1 2 3
```
#### 7. Delete a task
```
rstodo delete {nos} {--alldone [-y]}
```
"Choose One of Two" Parameter{nos} {--alldone [-y]}
```
{nos}       # Note: Delete the task with the specified number
{--alldone} # Note: Delete all completed tasks
[-y]        # Note: Confirm execution without a second prompt
```
Example
```
rstodo delete 1 2 3
rstodo delete --alldone
```
#### 8. Restore from the last operation
```
rstodo undo [-y]
```
Optional Parameters[-y]
```
[-y]   # Note: Confirm execution without a second prompt
```
#### 9. Display Task Status
```
rstodo status
```

## Build from Source Code
Rust ≥ 1.89.0

```bash
git clone https://github.com/TiaoFeng/rstodo.git
cd rstodo
cargo build --release
```

## Project Structure
```
src/
├── main.rs           # CLI Command Parsing and Main Program Entry Point
├── commands.rs       # Interaction Glue Layer for Terminal Interfaces
├── todo.rs           # Core business code that provides business interfaces
├── task.rs           # The `Task` Structure and Related Methods
├── time.rs           # Time Format Conversion and Time Zone Handling
├── error.rs          # Error Types Used in the Project
├── test_helpers.rs   # Helper Functions for Unit Testing
└── io/
    ├── storage.rs     # Reading and Saving a To-Do List
    └── cli_print.rs   # Terminal table output based on comfy_table
```

## License
This project is licensed under the [MIT License](LICENSE).

## Statement and Acknowledgments

- Claude Sonnet 5, DeepSeek V4 Flash, DeepSeek V4 Pro, Mimo 2.5 Pro, and KIMI K3 provide code reviews and technical guidance.
- [opencode](https://github.com/anomalyco/opencode) offers excellent, open-source tools.
- Translated with DeepL.com (free version)
