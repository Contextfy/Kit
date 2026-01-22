# Contextfy Kit

AI 上下文编排引擎 - 基于 Rust 构建的高性能知识检索系统。

## 快速开始

### 1. 初始化项目

```bash
cargo run --bin contextfy init
```

此命令将创建：
- `contextfy.json` - 项目清单
- `docs/examples/` - 包含示例 markdown 文件的目录

### 2. 构建知识库

```bash
cargo run --bin contextfy build
```

解析 `docs/examples/` 中的所有 markdown 文件并将其存储到 `.contextfy/data/` 中。

### 3. 搜索知识库

```bash
# CLI 搜索
cargo run --bin contextfy scout "您的搜索查询"

# Web UI 搜索
cargo run --bin contextfy-server
# 在浏览器中打开 http://127.0.0.1:3000
```

## 架构

```
┌─────────────────────────────────────────┐
│         CLI                 │
│        ┌───┴───┐           │
│        │  Server │  │
│        └─────┬───┘           │
│              ↓              │
├──────────────┼───────────────┤
│    Core               │
│  (Parser, Storage,      │
│   Retriever)        │
└──────────────────────────┘
```

### 包

| 包 | 描述 | 位置 |
|---------|-------------|----------|
| `contextfy-core` | Rust 引擎，包含 markdown 解析器、JSON 存储和检索器 | `packages/core/` |
| `contextfy-bridge` | Node.js FFI 绑定 (napi-rs) | `packages/bridge/` |
| `contextfy-server` | 带 REST API 的 Web 服务器 | `packages/server/` |
| `contextfy-cli` | 命令行界面 | `packages/cli/` |

## 开发

### 构建所有包

```bash
cargo build
```

### 运行特定包

```bash
# CLI
cargo run --bin contextfy

# Server
cargo run --bin contextfy-server
```

## 项目结构

```
Kit/
├── Cargo.toml              # 工作区配置
├── packages/
│   ├── core/           # Rust 引擎
│   ├── bridge/         # Node.js 绑定
│   ├── server/          # Web 服务器
│   ├── cli/             # CLI 工具
│   └── web/
│       └── static/    # 静态 HTML 页面
├── contextfy.json          # 项目清单（由 init 创建）
├── docs/examples/         # 示例 markdown 文档
└── .contextfy/data/       # 存储目录（由 build 创建）
```

## Helloworld 流程

**完整测试工作流：**

```bash
# 1. 使用示例数据初始化项目
cargo run --bin contextfy init

# 2. 从示例文档构建知识库
cargo run --bin contextfy build

# 3. 通过 CLI 测试搜索
cargo run --bin contextfy scout "API"
cargo run --bin contextfy scout "Example"

# 4. 通过 Web UI 测试搜索
cargo run --bin contextfy-server
# 打开 http://127.0.0.1:3000
# 在 Web 界面中输入搜索查询
# 点击结果查看完整文档内容
```

**预期结果：**
- `scout "API"` → 返回 "API Reference" 文档
- `scout "Example"` → 返回 "Example Document 1" 和 "Example Document 2"
- `scout "Feature"` → 返回 "Example Document 1"（包含 "Feature"）
- Web UI 搜索与 API 集成正常工作
- 点击结果打开详细文档视图

## 组件

### 核心引擎 (`packages/core/`)

**解析器模块**
- 从 markdown 中提取 H1 标题
- 生成摘要（前 200 个字符）
- 返回完整内容和元数据

**存储模块**
- 基于 JSON 的存储（`.contextfy/data/` 目录）
- 简单的架构：`id`、`title`、`summary`、`content`
- 文件级持久化以确保可靠性

**检索器模块**
- 两阶段搜索 API：
  - `scout(query)` → 返回简要结果列表
  - `inspect(id)` → 返回完整文档内容
- 标题和摘要中的不区分大小写文本匹配

### CLI (`packages/cli/`)

命令：
- `contextfy init` - 初始化新项目
- `contextfy build` - 解析并索引 markdown 文件
- `contextfy scout <query>` - 搜索知识库
- `contextfy serve` - 启动 Web 服务器（注意仅作提醒，直接使用 server 二进制文件）

### Web 服务器 (`packages/server/`)

REST API：
- `GET /api/search?q=<query>` - 搜索文档
- `GET /api/document/:id` - 按 ID 获取文档
- `GET /health` - 健康检查
- 在 `/` 处提供静态文件服务

### Web UI (`packages/web/static/`)

页面：
- `/` - 带项目概览的登陆页面
- `/search.html` - 带结果显示的搜索界面
- 现代、响应式设计，带渐变背景

## 存储架构

```json
{
  "id": "uuid",
  "title": "string",
  "summary": "string (前 200 个字符)",
  "content": "string"
}
```

## 当前限制

- 简单文本匹配（不区分大小写的子字符串搜索）
- 尚无向量嵌入或语义搜索
- JSON 文件存储而非 LanceDB（Arrow 依赖冲突）
- Node.js FFI 不可用（链接器错误，结构保留）
- 尚无 Python FFI

## 未来增强

- 与 LanceDB 集成的向量嵌入
- 语义搜索功能
- 多文档内容提取
- 增量构建跟踪
- 更好的错误处理和恢复
- 用于 AI/ML 框架的 Python FFI 绑定
- 增强的 Web UI，带过滤器和高级搜索

## 许可证

MIT

## 贡献

欢迎贡献！请阅读代码并遵循现有模式。