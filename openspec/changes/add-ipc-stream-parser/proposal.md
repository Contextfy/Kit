# Change: IPC 流式解析器集成

## Why

当前 Contextfy/Kit 编译管线需要处理 Python 生态的代码库（如 Minecraft Bedrock Script API），但 Rust 生态缺乏成熟的 Python AST 解析器（如 `cocoindex`）的等价实现。为了复用现有的 Python Sidecar 工具，我们需要建立一个高性能的进程间通信（IPC）管道。

当前存在的问题：
1. **语言生态鸿沟**：Python 代码库的 AST 解析在 Rust 中无成熟方案，需要借助现有 Python 工具
2. **一次性内存加载风险**：如果直接读取完整 stdout，大型代码库的 JSONL 输出会导致 OOM
3. **错误处理缺失**：子进程崩溃时 stderr 信息丢失，难以诊断问题

本次变更将建立 Rust → Python Sidecar 的标准化 IPC 通信模式，为未来多语言工具集成奠定基础。

## What Changes

### 1. 新增 IPC 通信模块

在 `packages/core/src/parser/ipc.rs` 中实现：
- `SidecarIPC` 结构体：封装子进程启动与流式读取逻辑
- 使用 `std::process::Command` 启动 `cocoindex` 二进制（开发态支持 `uv run` 跨 worktree 调用）
- 使用 `BufReader` 逐行读取 stdout，避免内存爆炸
- 使用 `serde_json::from_str` 将每行 JSONL 反序列化为 `AstChunk`

### 2. 数据结构定义

在 `packages/core/src/kernel/types.rs` 中新增：

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AstChunk {
    pub file_path: String,
    pub symbol_name: String,
    pub node_type: String,  // "function", "class", "method", etc.
    pub ast_content: String,
    pub dependencies: Vec<String>,
}
```

### 3. 错误处理增强

- 捕获子进程的 stderr，在失败时包含完整错误消息
- 使用 `anyhow::Error` 提供上下文信息
- 禁止 `unwrap()` 或 `expect()`，确保 Panic Safe

### 4. 单元测试策略

- 使用 `Command::new("echo")` 模拟子进程输出
- 测试 `BufReader` 流式读取逻辑
- 测试 `serde_json` 反序列化不会 panic
- **不依赖真实的 Python 二进制**（确保 CI 环境可运行）

**范围限制**：
- 不修改现有的 Markdown 解析逻辑
- 不集成到编译管线主流程（本次仅实现基础设施）
- 不替换现有存储层

**BREAKING**: None（纯新增功能，不修改现有 API）

## Impact

- Affected specs: `core-engine`
- Affected code:
  - `packages/core/src/kernel/types.rs`（新增 `AstChunk` 结构体）
  - `packages/core/src/parser/ipc.rs`（新增 IPC 模块）
  - `packages/core/src/parser/mod.rs`（导出 `ipc` 模块）
  - `packages/core/Cargo.toml`（无需新增依赖，`serde_json` 已存在）
- **BREAKING**: None（纯新增功能）
