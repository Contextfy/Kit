# Contextfy/Kit

> **高性能 AI 上下文编排引擎 (High-Performance Context Orchestration Engine)**

**"Context as Code."**

Contextfy/Kit 旨在解决 AI Agent 在垂直领域开发中面临的"知识断层"与"黑盒检索"问题。我们将非结构化的技术文档（Markdown, API Docs）编译为标准化的、可分发的、AI 原生的 Context Pack（上下文包），并提供一套高性能的运行时环境（Runtime）供上层应用（CLI, MCP Server）调用。

## 🚀 核心特性

### 两阶段检索 (Two-Stage Retrieval)
- **Scout（侦察）**: 仅返回摘要和评分，延迟 < 20ms
- **Inspect（检视）**: 按需加载完整内容，避免 Token 浪费
- 混合检索策略：Vector Search + BM25

### Context Pack
- 类似 Docker Image 的版本控制机制
- 增量编译支持，基于文件 Hash 跳过未变更章节
- Namespace 隔离，支持多 Pack 并发加载

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
│   ├── compiler/     # Markdown -> IR 编译管线
│   ├── storage/       # LanceDB + KV 存储
│   └── retriever/     # 混合检索引擎
├── packages/bridge/        # FFI 胶水层
│   ├── ffi_node/      # Node.js Binding (NAPI-RS)
│   └── ffi_py/        # Python Binding (PyO3)
├── packages/web/           # 可视化 Dashboard
│   ├── dashboard/     # 知识库管理 UI
│   └── debugger/      # 检索调试器
└── docs/              # 项目文档
    ├── PRD.md         # 产品需求文档
    ├── Architecture.md # 架构设计文档
    └── MVP.md         # MVP 规划文档
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
- **Storage**: `LanceDB` - 向量数据库 + Arrow 格式
- **Search**: `Tantivy` - 全文检索 (BM25)
- **Embedding**: `FastEmbed` - 本地 ONNX 模型

### Bridge (FFI)
- **Node.js**: `napi-rs` - 高性能绑定
- **Python**: `pyo3` - 原生 Python 扩展

### Web (Dashboard)
- **Frontend**: Next.js + TypeScript
- **Backend**: Axum + Tokio
- **Visualization**: D3.js / Cytoscape.js

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

```javascript
// Node.js 示例
const { Kit } = require('@contextfy/kit');

const kit = new Kit();

// Stage 1: Scout - 快速侦察
const briefs = await kit.scout('如何创建自定义剑?', { limit: 10 });
// 返回: [{ id: '1', title: 'Item API', summary: '...', score: 0.92 }]

// Stage 2: Inspect - 获取详情
const details = await kit.inspect(['1']);
// 返回: 完整的 Markdown 文档片段和代码示例
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

- **Scout 延迟**: < 20ms (100MB 文本知识库)
- **Top-3 召回率**: > 90% (测试集)
- **冷启动时间**: < 5 分钟 (从 init 到 AI 可调用)

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

### Phase 1: Foundation (v0.1)
- [ ] Markdown 解析与 LanceDB 存储
- [ ] `scout` 和 `inspect` 接口实现
- [ ] CLI `build` 命令

### Phase 2: Observability (v0.5)
- [ ] Next.js + Tauri Dashboard
- [ ] Search Playground 和 X-Ray 面板
- [ ] BM25 混合检索

### Phase 3: Ecosystem (v1.0)
- [ ] 稳定的 Node.js 和 Python 绑定
- [ ] Context Pack 导入/导出
- [ ] 完整的知识图谱可视化

---

**"Context as Knowledge, Prompt as Skill."**
