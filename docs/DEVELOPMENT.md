# 开发指南

本文档提供 Contextfy/Kit 项目开发的详细指南，包括架构、模块划分、开发流程和调试技巧。

## 📋 目录

- [项目架构](#项目架构)
- [模块说明](#模块说明)
- [本地开发](#本地开发)
- [测试](#测试)
- [调试](#调试)
- [发布流程](#发布流程)

## 🏗️ 项目架构

### 整体架构

```
Contextfy/Kit
├── packages/
│   ├── core/           # Rust 核心引擎
│   ├── bridge/         # FFI 桥接层 (Node.js/Python)
│   ├── cli/            # 命令行工具
│   └── server/         # Web 服务器
├── docs/              # 项目文档
└── openspec/          # OpenSpec 规格管理
```

### 架构分层

```
┌─────────────────────────────────────────────────┐
│         User Interfaces                      │
│  ┌──────┐  ┌──────┐  ┌──────────┐   │
│  │ CLI  │  │ Web  │  │ FFI SDK  │   │
│  └───┬──┘  └───┬──┘  └────┬─────┘   │
└──────┼──────────┼─────────┼─────────────┘
       │          │         │
┌──────┼──────────┼─────────┼─────────────┐
│      │   HTTP  │    FFI   │             │
│      └─────────┴─────────┘             │
│           Core API                      │
│  ┌──────────────────────────┐            │
│  │   Contextfy Core       │            │
│  │  ┌──────────────────┐  │            │
│  │  │ Retriever        │  │            │
│  │  │ ┌────┬────────┐ │  │            │
│  │  │ │Scout│Inspect│ │  │            │
│  │  │ └────┴────────┘ │  │            │
│  │  │ Storage          │  │            │
│  │  │  ┌─────────┐   │  │            │
│  │  │  │ Parser  │   │  │            │
│  │  │  └─────────┘   │  │            │
│  │  └──────────────────┘  │            │
│  └──────────────────────────┘            │
└─────────────────────────────────────────────┘
```

## 📦 模块说明

### Core Engine (`packages/core/`)

核心 Rust 引擎，提供文档解析、存储和检索功能。

**目录结构:**

```
packages/core/
├── Cargo.toml
└── src/
    ├── lib.rs              # 公共 API 导出
    ├── parser/            # Markdown 解析模块
    │   └── mod.rs
    ├── storage/           # 存储模块
    │   └── mod.rs
    └── retriever/        # 检索模块
        └── mod.rs
```

**核心类型:**

```rust
// 解析结果
pub struct ParsedDoc {
    pub path: String,
    pub title: String,
    pub summary: String,
    pub content: String,
}

// 存储记录
pub struct KnowledgeRecord {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub content: String,
}

// 检索结果（摘要）
pub struct Brief {
    pub id: String,
    pub title: String,
    pub summary: String,
}

// 检索结果（详情）
pub struct Details {
    pub id: String,
    pub title: String,
    pub content: String,
}
```

**开发规范:**

- 使用 `anyhow::Result` 作为错误类型
- 使用 `serde` 进行序列化/反序列化
- 所有公共 API 必须有文档注释
- 单元测试放在模块文件末尾

### CLI (`packages/cli/`)

命令行工具，提供 `init`, `build`, `scout`, `serve` 等命令。

**使用 `clap` 定义命令:**

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init { template: Option<String> },
    Build,
    Scout { query: String },
}
```

### Server (`packages/server/`)

使用 `axum` 提供的 Web 服务器，暴露 REST API。

**API 端点:**

```
GET  /api/search?q=query    # 搜索文档
GET  /api/document/:id      # 获取文档详情
GET  /health                # 健康检查
GET  /                     # 静态页面
```

**状态管理:**

```rust
use std::sync::Arc;
use tokio::sync::Mutex;

type AppState = Arc<Mutex<KnowledgeStore>>;
```

### Bridge (`packages/bridge/`)

使用 `napi-rs` 提供的 Node.js FFI 绑定。

**构建流程:**

```bash
# 需要使用 napi-rs CLI 构建
napi build --platform

# 或使用 npm scripts
npm run build
```

**注意**: 不能用 `cargo build` 构建此包，因为需要 Node.js 符号链接。

## 🚀 本地开发

### 环境要求

- Rust >= 1.75.0
- Node.js >= 20.0.0 (用于 bridge 构建)
- Git

### 初始化开发环境

```bash
# 1. Clone 仓库
git clone https://github.com/Contextfy/Kit.git
cd Kit

# 2. 安装 Rust 工具链
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 3. 构建 Rust 项目
cargo build

# 4. 运行测试
cargo test
```

### 常用开发命令

```bash
# 格式化代码
cargo fmt

# 检查代码
cargo clippy

# 运行测试
cargo test

# 运行特定包的测试
cargo test -p contextfy-core

# 运行 CLI
cargo run --bin contextfy init
cargo run --bin contextfy build

# 运行 Server
cargo run --bin contextfy-server

# 构建 Bridge (需要 Node.js)
cd packages/bridge
npm install
npm run build
```

### 添加新依赖

```bash
# 添加依赖
cargo add serde

# 添加开发依赖
cargo add --dev tokio-test

# 指定版本
cargo add anyhow --version 1.0.0
```

## 🧪 测试

### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_title() {
        let input = "# Test Document\nContent...";
        let doc = parse_markdown(input).unwrap();
        assert_eq!(doc.title, "Test Document");
    }

    #[tokio::test]
    async fn test_async_search() {
        let store = KnowledgeStore::new("/tmp/test")?;
        let results = store.search("test").await?;
        assert!(!results.is_empty());
    }
}
```

### 集成测试

在 `tests/` 目录下创建：

```rust
// tests/integration_test.rs
use contextfy_core::*;

#[tokio::test]
async fn test_e2e_flow() {
    // 1. 解析文档
    let doc = parse_markdown("test.md")?;

    // 2. 存储文档
    let store = KnowledgeStore::new("/tmp/test")?;
    let id = store.add(&doc).await?;

    // 3. 检索文档
    let retriever = Retriever::new(&store);
    let briefs = retriever.scout("test").await?;
    assert!(!briefs.is_empty());
}
```

### 测试覆盖

```bash
# 生成覆盖率报告
cargo install cargo-tarpaulin
cargo tarpaulin --out Html

# 或使用
cargo install cargo-llvm-cov
cargo llvm-cov --html
```

## 🐛 调试

### 使用 `dbg!` 宏

```rust
let result = parse_markdown(input)?;
dbg!(&result); // 打印到 stderr
```

### 使用 VSCode 调试

创建 `.vscode/launch.json`:

```json
{
    "version": "0.2.0",
    "configurations": [
        {
            "type": "lldb",
            "request": "launch",
            "name": "Debug contextfy-core",
            "cargo": {
                "args": [
                    "build",
                    "--package=contextfy-core",
                    "--bin=contextfy-core"
                ],
                "filter": {
                    "name": "contextfy-core",
                    "kind": "bin"
                }
            },
            "args": [],
            "cwd": "${workspaceFolder}"
        }
    ]
}
```

### 日志调试

使用 `env_logger`:

```rust
use env_logger;

fn main() {
    env_logger::init();
    // ...
}

// 运行时设置日志级别
RUST_LOG=debug cargo run
```

### 常见问题

**Q: 编译错误 `error[E0432]: unresolved imports`**

A: 检查模块导入路径，确保在 `lib.rs` 中导出。

**Q: 生命周期错误**

A: Rust 生命周期复杂，参考 [Rust Book - Lifetimes](https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html)

**Q: Bridge 链接错误**

A: 必须使用 `napi build` 或 `npm run build`，不能用 `cargo build`

## 🚢 发布流程

### 版本号更新

1. 更新 `Cargo.toml` 中的版本号
2. 运行 `cargo publish` (如果发布到 crates.io)
3. 创建 Git tag: `git tag v0.1.0`
4. Push tag: `git push origin v0.1.0`

### 里程碑管理

使用 GitHub Milestones 追踪版本计划：

1. 创建新的 Milestone
2. 将相关 Issue 添加到 Milestone
3. 完成所有 Issue 后关闭 Milestone
4. 发布新版本

## 📚 参考资源

- [Rust 官方文档](https://doc.rust-lang.org/)
- [Cargo Book](https://doc.rust-lang.org/cargo/)
- [Tokio 文档](https://docs.rs/tokio/)
- [Axum 文档](https://docs.rs/axum/)
- [napi-rs 文档](https://napi.rs/)

## 💡 最佳实践

1. **保持模块小而专注**: 每个模块只做一件事
2. **优先使用标准库**: 避免不必要的依赖
3. **编写测试先行**: 测试驱动开发
4. **文档注释**: 为公共 API 编写清晰的文档
5. **错误处理**: 使用 `anyhow` 和 `?` 操作符优雅处理错误
6. **异步优先**: 使用 `tokio` 处理 I/O 操作

## 🤔 有问题？

查看 [CONTRIBUTING.md](./CONTRIBUTING.md) 了解如何贡献，或在 GitHub Issues 中提问。
