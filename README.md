# Contextfy/Kit

> **高性能 AI 上下文编排引擎 (High-Performance Context Orchestration Engine)**

**"Context as Code."**

Contextfy/Kit 旨在解决 AI Agent 在垂直领域开发中面临的"知识断层"与"黑盒检索"问题。我们将非结构化的技术文档（Markdown, API Docs）编译为标准化的、可分发的、AI 原生的 Context Pack（上下文包），并提供一套高性能的运行时环境（Runtime）供上层应用（CLI, MCP Server）调用。

## 🚀 核心特性

### 混合检索架构 (Hybrid Retrieval Architecture)
- **原生双引擎**: 并行执行 BM25 (全文) + 向量 (语义) 搜索
- **RRF 融合**: 使用倒数排名融合算法 (k=60) 合并结果
- **优雅降级**: 单个后端失败时自动使用另一个
- **两阶段检索**:
  - **Scout（侦察）**: 仅返回摘要和评分，延迟 < 100ms
  - **Inspect（检视）**: 按需加载完整内容，避免 Token 浪费

### 三层存储设计
- **Facade 层**: `SearchEngine` 提供极简 API (`search`, `add`, `get`)
- **Orchestrator 层**: `HybridOrchestrator` 协调双存储并合并结果
- **Storage 层**: `LanceDbStore` (384维向量) + `TantivyBm25Store` (全文索引)

### 可观测性 (Observability)
- Web UI 仪表盘可视化检索过程
- X-Ray 面板展示向量匹配度、关键词命中率和热力图
- 完整的 Trace ID 和打分日志

### 统一编译管线
- 支持 Markdown、MDX、HTML 等异构数据源
- 标准化的中间表示 (IR)
- 自动语义切片和摘要生成

## 📦 项目结构

Contextfy/Kit 采用 Monorepo 结构，强制实现**核心逻辑与交互层分离**。

```
Contextfy/Kit
├── packages/core/          # 核心引擎 (Rust)
│   ├── facade/        # SearchEngine 对外 API
│   ├── slices/        # 存储切片（模块化架构）
│   │   ├── vector/    # LanceDB 向量存储
│   │   ├── bm25/      # Tantivy BM25 存储
│   │   └── hybrid/    # 混合检索编排 (RRF)
│   ├── parser/        # Markdown -> IR 编译管线
│   ├── embeddings/    # FastEmbed 向量化模型
│   └── kernel/        # 核心类型与错误定义
├── packages/cli/           # 命令行工具
│   └── commands/     # build, scout, serve 命令
├── packages/server/         # Web 服务器
│   └── main.rs        # Axum REST API
├── packages/web/           # 可视化 Dashboard
│   └── static/        # 静态前端资源
└── docs/              # 项目文档
    ├── PRD.md         # 产品需求文档
    ├── Architecture.md # 系统架构文档（三层架构图）
    └── QuickStart.md  # 快速入门指南
```

## 🎯 使用场景

### MVP 场景：Minecraft 基岩版 Addon 开发

Contextfy/Kit 首个验证场景是协助 AI 构建高质量的 Minecraft Bedrock Addon。

**用户需求**：

> "帮我做一个红色的'治疗石'方块，玩家站上去每秒回 2 点血。"

**系统自动完成**：

1. ✅ 工程创建：生成合规的 BP (Behavior Pack) 和 RP (Resource Pack) 目录结构
2. ✅ 资源注册：在 RP 中注册贴图和方块定义
3. ✅ 逻辑实现：准确检索 `@minecraft/server` API，编写 TypeScript 脚本
4. ✅ 无人工干预：生成的代码无需修改即可在游戏中运行

**核心能力**：

- **The Library (Contextfy)**: 提供准确的 API 文档和类型定义
- **The Instructor (Skills)**: 控制工程流程和最佳实践
- **零幻觉**: 所有 API 调用基于官方文档验证

## 🛠️ 技术栈

### Core (Rust)
- **Parsing**: `pulldown-cmark` - Markdown AST 解析
- **Storage**:
  - `LanceDB` - 向量数据库 + Arrow 格式（384维向量）
  - `Tantivy` - BM25 全文检索（支持中文分词）
- **Hybrid Search**: RRF (Reciprocal Rank Fusion) 倒数排名融合
- **Embedding**: `FastEmbed` - 本地 ONNX 模型（BGE-small-en-v1.5）
- **Async Runtime**: `tokio` - 异步执行引擎

### CLI (Rust)
- **Framework**: `clap` - 命令行参数解析
- **Commands**: `build`, `scout`, `serve`, `init`

### Server (Rust)
- **Framework**: `axum` - 异步 Web 框架
- **API**: RESTful endpoints (`/api/search`, `/api/document/:id`)
- **Logging**: `tracing` - 结构化日志

## 📖 快速开始

### 安装

```bash
# 克隆仓库
git clone https://github.com/Contextfy/Kit.git
cd Kit

# 构建核心引擎
cargo build --release
```

### 初始化知识库

```bash
# 初始化项目（以基岩版模板为例）
contextfy init --template bedrock-v1.21

# 构建 Context Pack
contextfy build
```

### 使用检索 API

```bash
# 混合搜索（BM25 + 向量）
kit scout "如何创建自定义剑?"

# 返回结果示例：
# [1] Score: 0.92 | ID: doc-123
#     Title: Item API
#     Summary: 创建自定义物品的完整文档...
```

```rust
// Rust API 示例
use contextfy_core::SearchEngine;

let engine = SearchEngine::new(
    Some(std::path::Path::new(".contextfy/data/bm25_index")),
    ".contextfy/data/lancedb",
    "knowledge"
).await?;

// 混合搜索（BM25 + 向量 + RRF 融合）
let hits = engine.search("自定义剑", 10).await?;

// 获取完整文档内容
let doc = engine.get_document("doc-123").await?;
```

### 启动 Dashboard

```bash
# 启动 Web UI
contextfy ui

# 浏览器打开 http://localhost:3000
```

## 🎬 演示流程

完整的演示剧本请参考 [docs/MVP.md](./docs/MVP.md)。

**Step 1**: 准备知识库

```bash
contextfy init --template bedrock-v1.21
contextfy build
```

**Step 2**: 调试检索效果
```bash
contextfy ui
# 在 Dashboard 中测试 Query，观察 X-Ray 面板
```

**Step 3**: 集成到 AI Agent
```bash
# 加载 Skills (通过 System Prompt 注入)
export CLAUDE_SYSTEM_PROMPT=$(cat bedrock-skills.xml)

# AI 现在可以调用 contextfy scout/inspect 来验证 API
```

## 📊 性能指标

- **混合检索延迟**: < 100ms (BM25 + 向量并行执行)
- **Top-3 召回率**: > 90% (混合检索优于单一方法)
- **冷启动时间**: < 2 分钟 (Embedding 模型首次加载)
- **后续启动**: < 5 秒 (模型已缓存)

## 🤝 贡献指南

欢迎贡献！我们欢迎任何形式的贡献，包括代码、文档、Bug 报告和功能建议。

### 📚 文档索引

#### 核心文档
- [CONTRIBUTING.md](./CONTRIBUTING.md) - 贡献指南与流程
- [DEVELOPMENT.md](./docs/DEVELOPMENT.md) - 开发指南与架构
- [ISSUE_WORKFLOW.md](./docs/ISSUE_WORKFLOW.md) - Issue 管理与协作流程

#### 产品与设计
- [PRD - 产品需求文档](./docs/PRD.md)
- [Architecture - 系统架构文档](./docs/Architecture.md)
- [MVP - MVP 规划](./docs/MVP.md)
- [QuickStart - 快速入门](./docs/QuickStart.md)

### 🚀 快速贡献

#### 方式一：接手开发任务

1. 访问 [Issues 页面](https://github.com/Contextfy/Kit/issues)
2. 筛选标记为 `status:ready` 的 Issue
3. 选择你感兴趣的任务并评论认领
4. 按照 Issue 中的要求开发并提交 PR

详见：[ISSUE_WORKFLOW.md](./docs/ISSUE_WORKFLOW.md)

#### 方式二：报告 Bug 或提建议

使用 [Issue 模板](.github/ISSUE_TEMPLATE/) 创建 Issue：
- 🐛 [Bug Report](.github/ISSUE_TEMPLATE/bug_report.md)
- 🚀 [Feature Request](.github/ISSUE_TEMPLATE/feature_request.md)
- 🤔 [Discussion](.github/ISSUE_TEMPLATE/discussion.md)
- 📚 [Documentation](.github/ISSUE_TEMPLATE/documentation.md)

### 💻 开发指南

详细的开发流程、代码规范、测试要求请参考：

- [DEVELOPMENT.md](./docs/DEVELOPMENT.md) - 开发环境搭建、架构说明、调试技巧
- [CONTRIBUTING.md](./CONTRIBUTING.md) - 代码规范、提交规范、PR 流程

## 📞 交流

加入我们的 QQ 群交流：**1065806393**

## 📜 许可证

MIT License - 详见 [LICENSE](./LICENSE) 文件

## 🗺️ 路线图

### ✅ Phase 1: Foundation (已完成)
- [x] Markdown 解析与语义切片
- [x] LanceDB 向量存储 (384维 BGE-small-en)
- [x] Tantivy BM25 全文检索
- [x] 混合检索架构 (RRF 融合)
- [x] CLI 命令 (`build`, `scout`, `serve`)
- [x] Web Server (Axum REST API)

### 🔄 Phase 2: Observability (进行中)
- [x] Web Dashboard 基础 UI
- [ ] Search Playground
- [ ] X-Ray 调试面板（可视化检索过程）
- [ ] 性能监控与指标收集

### 📋 Phase 3: Ecosystem (规划中)
- [ ] Node.js 和 Python FFI 绑定
- [ ] Context Pack 导入/导出
- [ ] 知识图谱可视化

---

**"Context as Knowledge, Prompt as Skill."**
