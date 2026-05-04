## ADDED Requirements

### Requirement: IPC 流式解析器

The core engine SHALL provide an inter-process communication (IPC) parser that spawns a Python sidecar process and reads AST chunks from stdout in a streaming fashion using JSON Lines format. 核心引擎 SHALL 提供进程间通信（IPC）解析器，该解析器生成 Python 侧车进程并使用 JSON Lines 格式以流式方式从 stdout 读取 AST 块。

#### Scenario: 成功启动子进程并读取 AST 块

- **当**用户调用 `SidecarIPC::spawn(["cocoindex", "parse", "file.py"])` 时
- **则**系统使用 `std::process::Command` 启动子进程
- **并且**系统配置 `stdout(Stdio::piped())` 和 `stderr(Stdio::piped())`
- **并且**系统返回 `SidecarIPC` 实例，包含子进程句柄和 BufReader
- **并且**子进程成功启动，返回码在未来可用 `wait()` 获取

#### Scenario: 流式读取并反序列化 JSONL 输出

- **当**用户调用 `sidecar_ipc.next_chunk()` 方法时
- **则**系统使用 `BufReader::read_line()` 读取一行 stdout
- **并且**系统对每行调用 `serde_json::from_str::<AstChunk>()`
- **并且**系统在反序列化成功时返回 `Ok(Some(AstChunk))`
- **并且**系统在反序列化失败时返回 `Err(IpcError::JsonParseFailed)`
- **并且**系统在到达 EOF 时返回 `Ok(None)`
- **并且**系统不一次性将所有 stdout 加载到内存（流式处理）

#### Scenario: 检测子进程异常退出并捕获错误

- **当**子进程以非零退出码终止时
- **并且**用户已读取完所有 stdout（`next_chunk()` 返回 `Ok(None)`）后调用 `ipc.wait()` 时
- **则**系统调用 `child.wait()` 获取退出码
- **并且**系统读取 `stderr` 的完整内容
- **并且**系统返回 `Err(IpcError::ChildExitedAbnormally)` 包含退出码和 stderr 消息
- **并且**系统不 panic 或崩溃（Panic Safe）

#### Scenario: 开发模式使用 uv run 调用本地工具

- **当**环境变量 `COCOINDEX_MODE=dev` 被设置时
- **并且**用户调用 `SidecarIPC::spawn()` 时
- **则**系统使用 `Command::new("uv")` 并传递参数 `["run", "cocoindex", ...]`
- **并且**系统支持跨 worktree 调用本地开发的 Python 工具
- **当**环境变量未设置或为 `prod` 时
- **则**系统直接调用 `Command::new("cocoindex")`（生产态）

#### Scenario: 处理空的 stdout 输出

- **当**子进程成功启动但 stdout 无任何输出时
- **则**`next_chunk()` 立即返回 `Ok(None)`
- **并且**系统不产生错误或 panic
- **并且**系统检查子进程退出码（如果为 0 则视为成功）

#### Scenario: 处理格式错误的 JSON 行

- **当**stdout 包含非 JSON 格式的行时
- **则**`serde_json::from_str` 返回错误
- **并且**系统将错误包装为 `IpcError::JsonParseFailed`
- **并且**错误信息包含：
  - 行号（`line_number`）
  - 原始行内容（`raw_line`）
  - serde_json 的原始错误消息（`cause`）
- **并且**`next_chunk()` 立即返回 `Err(...)`，由调用方决定是否继续

### Requirement: AST 块数据结构

The core engine SHALL define an `AstChunk` structure that represents a parsed code symbol with metadata for storage and indexing. 核心引擎 SHALL 定义 `AstChunk` 结构体，表示已解析的代码符号及其元数据，用于存储和索引。

#### Scenario: 反序列化有效的 JSONL 为 AstChunk

- **当**系统接收到符合以下格式的 JSON 行时：

  ```json
  {
    "file_path": "/path/to/file.py",
    "symbol_name": "MyClass",
    "node_type": "class",
    "ast_content": "class MyClass:\n    pass",
    "dependencies": ["OtherClass"]
  }
  ```
- **则**`serde_json::from_str::<AstChunk>()` 成功反序列化
- **并且**返回的 `AstChunk` 包含所有字段
- **并且**`dependencies` 字段为字符串数组，长度为 1

#### Scenario: 处理缺少可选字段的 JSON

- **当**系统接收到缺少 `dependencies` 字段的 JSON 时
- **则**系统使用 `#[serde(default)]` 将 `dependencies` 设为空数组 `Vec::new()`
- **并且**反序列化成功，不返回错误

#### Scenario: AstChunk 支持序列化和网络传输

- **当**系统需要将 `AstChunk` 写入磁盘或通过网络发送时
- **则**`AstChunk` 实现 `Serialize` trait
- **并且**调用 `serde_json::to_string(&chunk)` 返回有效的 JSON 字符串
- **并且**序列化后的 JSON 与原始输入格式兼容（往返对称性）

### Requirement: IPC 错误类型定义

The core engine SHALL define structured error types for IPC operations that provide contextual information for debugging. 核心引擎 SHALL 为 IPC 操作定义结构化错误类型，提供上下文信息以支持调试。

#### Scenario: 子进程启动失败错误

- **当**系统尝试启动不存在的命令时（如 `cocoindex` 未安装）
- **则**`Command::spawn()` 返回 `std::io::Error`
- **并且**系统将其包装为 `IpcError::ChildStartFailed`
- **并且**错误信息包含：
  - 尝试执行的完整命令字符串（`command`）
  - 原始错误的 cause（`cause`）
- **并且**错误消息人类可读，指示"未找到命令"或"权限不足"

#### Scenario: JSON 解析失败错误包含调试信息

- **当**`serde_json::from_str::<AstChunk>()` 失败时
- **则**系统构造 `IpcError::JsonParseFailed`
- **并且**使用 `.context()` 添加以下信息：
  - 失败的行号（从 1 开始计数）
  - 原始行内容的前 100 字符（避免日志爆炸）
  - serde_json 的详细错误（如 "missing field `file_path` at line 1 column 5"）
- **并且**错误可通过 `anyhow::Error` 向上传播

#### Scenario: 子进程异常退出错误包含 stderr

- **当**子进程以退出码 1 终止时
- **并且**stderr 输出 "SyntaxError: invalid syntax"
- **则**系统构造 `IpcError::ChildExitedAbnormally`
- **并且**错误信息包含：
  - 退出码：`Some(1)`
  - stderr 完整内容：`"SyntaxError: invalid syntax"`
- **并且**调用方可通过 `match error` 提取这些字段用于诊断

### Requirement: IPC 解析器单元测试覆盖

The core engine SHALL provide comprehensive unit tests for the IPC parser using mock child processes (e.g., `echo`) without requiring Python dependencies. 核心引擎 SHALL 为 IPC 解析器提供全面的单元测试，使用模拟子进程（如 `echo`）而不需要 Python 依赖。

#### Scenario: 使用 echo 模拟有效 JSONL 输出

- **当**测试使用 `Command::new("echo").arg("{\"file_path\":\"test.py\",\"symbol_name\":\"foo\",\"node_type\":\"function\",\"ast_content\":\"pass\",\"dependencies\":[]}")` 时
- **则**`SidecarIPC::spawn()` 成功启动
- **并且**`next_chunk()` 第一次调用返回 `Ok(Some(AstChunk))`
- **并且**`next_chunk()` 第二次调用返回 `Ok(None)`（EOF）
- **并且**`AstChunk` 字段值与 JSON 输入匹配
- **并且**测试可在 CI 环境运行（无需 Python）

#### Scenario: 使用 echo 模拟无效 JSON 输出

- **当**测试使用 `echo` 输出 "not a json" 时
- **则**`next_chunk()` 返回 `Err(IpcError::JsonParseFailed)`
- **并且**错误类型可通过 `match` 模式匹配
- **并且**测试断言错误信息包含 "expected value" 或类似关键词

#### Scenario: 使用 sh -c "exit 1" 模拟子进程崩溃

- **当**测试启动 `sh -c "exit 1"` 作为子进程时
- **则**`next_chunk()` 立即返回 `Ok(None)`（EOF）
- **并且**调用 `ipc.wait()` 后系统检测到非零退出码
- **并且**返回 `Err(IpcError::ChildExitedAbnormally)`
- **并且**`exit_code` 字段为 `Some(1)`
- **并且**测试不 panic

#### Scenario: 使用空 echo 模拟空输出

- **当**测试使用 `echo -n ""` 启动子进程时（无换行符）
- **则**`next_chunk()` 立即返回 `Ok(None)`
- **并且**`child.wait()` 返回 `Ok(ExitStatus::exit_code(0))`
- **并且**测试断言没有 `AstChunk` 被生成
