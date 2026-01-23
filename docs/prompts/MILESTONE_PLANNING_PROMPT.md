# Milestone 和 Issue 规划 Prompt

这个 prompt 用于让 AI 深度思考项目规划，分解里程碑和任务，并调用 GitHub MCP 创建。

## 🎯 使用说明

1. 将本 prompt 的内容复制给 AI
2. AI 会深度阅读项目文档、分析现状、规划里程碑
3. AI 会调用 GitHub MCP 工具创建 milestone 和 issue

---

## 📋 完整 Prompt 内容

```
你是一个经验丰富的项目规划师和 GitHub 项目管理专家。你的任务是为 Contextfy/Kit 项目规划里程碑和 issue。

### 📚 第一步：深度理解项目

**阅读以下文档（按优先级）：**

1. **必读 - 项目现状**
   - `docs/PRD.md` - 产品需求文档
   - `docs/Architecture.md` - 系统架构
   - `docs/MVP.md` - MVP 规划
   - `README.md` - 项目概述
   - `openspec/project.md` - OpenSpec 项目信息
   - `openspec/specs/` - 所有已实现的需求规格

2. **必读 - 工作流程**
   - `docs/ISSUE_WORKFLOW.md` - Issue 管理流程
   - `docs/DEVELOPMENT.md` - 开发指南
   - `CONTRIBUTING.md` - 贡献指南
   - `.github/labels.yml` - 标签体系

3. **可选 - 已完成的工作**
   - `openspec/changes/archive/` - 查看已归档的变更

### 🔍 第二步：分析项目状态

**回答以下问题：**

1. **当前完成度**：
   - 哪些核心功能已实现？
   - 哪些模块可以工作？
   - 技术债务有哪些？
   - 已知 Bug 有哪些？

2. **技术约束**：
   - Rust 版本要求？
   - 依赖项的当前状态？
   - 性能指标目标？
   - 集成要求（Node.js、Python）？

3. **团队能力假设**：
   - 假设开发者熟悉 Rust？
   - 假设有测试经验？
   - 假设熟悉领域知识？

### 🎯 第三步：定义规划目标

**根据用户提供的规划目标，明确：**

```
用户目标: [用户在本次规划中提供的具体目标]

规划范围:
- 是否包含新功能开发？具体是哪些？
- 是否包含重构或优化？
- 是否包含文档更新？
- 时间范围: [如: 2周、1个月]
```

### 📊 第四步：设计里程碑

**设计 2-4 个里程碑，每个里程碑 1-2 周周期。**

**里程碑设计原则：**
1. **优先核心功能**：先完成必要的、阻塞其他功能的功能
2. **依赖关系清晰**：后面的里程碑依赖前面的
3. **可交付成果**：每个里程碑结束时有可用的功能
4. **任务平衡**：每个里程碑包含不同类型的任务（开发、测试、文档）

**里程碑模板：**

```
## Milestone [名称] (v0.x.0)

**目标**: 一句话描述这个里程碑要达成的目标

**交付成果**:
- [ ] 成果 1
- [ ] 成果 2
- [ ] 成果 3

**关键功能**:
- 功能 1: 描述
- 功能 2: 描述

**预计周期**: 2 周
**依赖**: 前置里程碑（如果有）
```

**示例里程碑分解（参考）：**

### Milestone v0.2.0 - 存储优化
**目标**: 优化存储性能，支持更大数据量

**交付成果**:
- [ ] BM25 排序算法实现
- [ ] 文件缓存机制
- [ ] 性能基准测试

**关键功能**:
- BM25: 提升检索相关性
- 缓存: 减少 I/O 开销
- 基准: 建立性能基线

**预计周期**: 2 周

### Milestone v0.3.0 - 混合检索
**目标**: 实现向量 + 全文混合检索

**交付成果**:
- [ ] 向量嵌入模块
- [ ] 混合检索算法
- [ ] 检索结果融合

**关键功能**:
- 向量: 语义搜索能力
- 混合: 结合关键词和语义
- 融合: 智能结果排序

**预计周期**: 2 周
**依赖**: v0.2.0

### 📝 第五步：分解 Issue

**为每个里程碑分解为具体的 Issue。**

**Issue 分解原则：**

1. **原子性**: 每个 Issue 只做一件事
2. **可测试**: 每个 Issue 有明确的验收标准
3. **小而专注**: 预计 1-3 天完成
4. **模块清晰**: 明确属于哪个模块
5. **依赖明确**: 如果有前置依赖，在 Issue 中说明

**Issue 创建顺序：**

1. **基础设施**（如果有）：工具链、CI、构建脚本
2. **核心功能**: 最重要的功能优先
3. **辅助功能**: 依赖核心的功能
4. **优化和重构**: 功能完成后再做
5. **文档**: 随功能一起或最后

**Issue 创建前思考清单：**

- [ ] 这个 Issue 是否足够小？（如果不是，继续拆分）
- [ ] 验收标准是否清晰？（开发者能判断是否完成）
- [ ] 是否有合适的技术方案？（如果复杂，需要 Discussion）
- [ ] 标签是否正确？（type + priority + area + complexity）
- [ ] 优先级是否合理？（依赖关系考虑）

**Issue 内容模板（基于 dev_task.md）：**

```markdown
[TASK] 模块名称 - 任务描述

## 🎯 任务目标
一句话描述这个任务的目标。

## 📦 涉及模块
- [ ] core-engine
- [ ] cli
- [ ] server
- [ ] bridge
- [ ] web-dashboard
- [ ] docs

## 📝 需求详情

### 功能描述
详细描述需要实现的功能或修复的问题。

### 接受标准
- [ ] 功能实现完成
- [ ] 添加了必要的单元测试
- [ ] 代码通过 `cargo test`
- [ ] 代码通过 `cargo fmt` 和 `cargo clippy`
- [ ] 更新了相关文档

### 技术要求
- Rust 版本要求: `>= 1.75.0`
- 遵循现有的代码风格和架构模式
- 使用 `anyhow::Result` 作为错误类型
- 添加适当的注释

## 📚 相关资源
- 相关 OpenSpec 变更: (如果有)
- 相关 PR: #
- 参考实现: (链接到文档或类似代码)

## 🚫 不需要做的事
明确说明这个任务不需要做的事情，避免范围蔓延。

---
## ✅ 完成清单（接手者填写）
（接手开发后再填写）

### 开发完成
- [ ] 代码实现
- [ ] 单元测试
- [ ] 文档更新
- [ ] 本地测试通过

### PR 提交前
- [ ] 代码格式化: `cargo fmt`
- [ ] 代码检查: `cargo clippy`
- [ ] 所有测试通过: `cargo test`
- [ ] PR 描述清晰，包含相关 Issue 链接
```

### 🏷️ Issue 标签策略

**每个 Issue 必须包含以下标签：**

1. **Type（类型）**: `type:bug` / `type:feature` / `type:enhancement` / `type:refactor`
2. **Priority（优先级）**: `priority:critical` / `priority:high` / `priority:medium` / `priority:low`
3. **Status（状态）**: `status:ready`（创建时默认为 ready）
4. **Area（模块）**: `area:core` / `area:cli` / `area:server` / `area:bridge` / `area:web` / `area:docs`
5. **Complexity（复杂度）**: `complexity:small` / `complexity:medium` / `complexity:large`

**优先级分配规则：**
- `critical`: 阻塞其他功能，安全漏洞，严重 bug
- `high`: 核心功能，性能关键
- `medium`: 普通功能，增强
- `low`: 错误处理，文档，优化

**复杂度分配规则：**
- `small`: 1-2 小时，独立单元，修改单个文件
- `medium`: 3-5 小时，涉及 2-3 个文件，需要一些设计
- `large`: 1-2 天，跨多个模块，需要架构设计

### 🚀 第六步：调用 GitHub MCP 创建

**使用 GitHub MCP 工具按以下顺序创建：**

#### 1. 创建 Milestones

```javascript
// 创建里程碑
const milestone = await create_milestone({
  owner: "Contextfy",
  repo: "Kit",
  title: "v0.2.0 - 存储优化",
  description: "优化存储性能，支持更大数据量",
  state: "open",
  due_on: "2026-02-05T00:00:00Z" // 2周后
});
```

#### 2. 创建 Issues

```javascript
// 为每个 milestone 创建 issue
for (const issue of issues) {
  const created = await create_issue({
    owner: "Contextfy",
    repo: "Kit",
    title: issue.title,
    body: issue.body,
    labels: issue.labels,
    milestone: issue.milestone_number // 使用创建后的 milestone number
  });
}
```

**创建顺序建议：**
1. 先创建所有 Milestones，获取它们的 numbers
2. 按依赖顺序创建 Issues
3. 先创建基础设施类 Issue
4. 再创建核心功能类 Issue

### 📊 第七步：验证和总结

**创建完成后，输出总结：**

```
## 📊 规划总结

### 创建的 Milestones (3个)
1. v0.2.0 - 存储优化 (Due: 2026-02-05)
   - 8 个 Issue
2. v0.3.0 - 混合检索 (Due: 2026-02-19)
   - 10 个 Issue
3. v0.4.0 - Web Dashboard 增强 (Due: 2026-03-04)
   - 12 个 Issue

### 总 Issue 数量
- 总计: 30 个
- 优先级: critical(2), high(8), medium(15), low(5)
- 模块: core(12), cli(5), server(4), web(5), docs(4)
- 复杂度: small(10), medium(15), large(5)

### 规划建议
1. 建议先从 milestone v0.2.0 开始
2. 优先完成 critical 和 high 优先级的 Issue
3. 新手可以从 small 复杂度、label "good first issue" 的任务开始
```

### ⚠️ 注意事项

1. **不要一次性创建太多 Issue**: 建议 15-30 个为宜，避免 overwhelme
2. **保持灵活性**: 规划是动态的，实际开发中可以调整
3. **沟通优先**: 不确定的技术点先创建 Discussion Issue
4. **文档同步**: 每个里程碑完成后，更新相关文档

### 📝 最终输出格式

**请按以下格式输出你的规划过程和结果：**

```markdown
## 📚 项目理解

### 当前状态
[项目当前完成情况总结]

### 技术约束
[技术约束和依赖情况]

### 规划目标
[用户提供的规划目标]

## 🎯 里程碑规划

### Milestone 1: v0.2.0 - [名称]
**目标**: [一句话描述]
**周期**: [2周]
**Issue 数量**: [X个]
**关键功能**:
- [功能1]
- [功能2]

### Milestone 2: v0.3.0 - [名称]
...

## 📋 Issue 列表（按 Milestone 分组）

### v0.2.0 (8个 Issue)

1. **[TASK] Core - BM25 ranking algorithm**
   - Priority: high
   - Complexity: medium
   - Labels: type:enhancement, priority:high, area:core, complexity:medium

2. **[TASK] Core - File caching mechanism**
   - Priority: medium
   - Complexity: small
   - ...

### v0.3.0 (10个 Issue)
...

## 🚀 GitHub 操作计划

**第一步：创建 Milestones**
- v0.2.0 - [描述]
- v0.3.0 - [描述]

**第二步：创建 Issues（按依赖顺序）**
1. [创建 issue 1]
2. [创建 issue 2]
...

## 📊 规划总结
[总览统计和建议]
```

---

## 💡 使用示例

### 用户请求示例

```
"请为 Contextfy/Kit 规划接下来一个月的开发路线，重点是：
1. 实现混合检索（向量 + 全文）
2. 优化性能（BM25、缓存）
3. 改进 Web UI（添加可视化面板）
时间范围：4 周"
```

### AI 执行流程

1. **阅读文档** (10-15分钟)
   - 读取 PRD, Architecture, MVP
   - 理解 Issue 工作流程

2. **分析状态** (5-10分钟)
   - 评估当前完成度
   - 识别技术约束

3. **规划里程碑** (15-20分钟)
   - 设计 2-3 个里程碑
   - 确保依赖关系清晰

4. **分解 Issue** (20-30分钟)
   - 为每个里程碑分解任务
   - 确保每个 Issue 可测试、可交付

5. **调用 GitHub MCP** (10-15分钟)
   - 创建 Milestones
   - 创建 Issues（按顺序）

6. **输出总结** (5分钟)
   - 提供总览和建议

## ⚙️ GitHub MCP 配置

**确保你的环境已配置 GitHub MCP:**

参考 Claude MCP 服务器配置文档：
- 安装 GitHub MCP 服务器
- 配置 GITHUB_TOKEN 权限
- 确保 token 有创建 milestone 和 issue 的权限

**需要权限:**
- `repo` (完整仓库权限）
- `milestones` (创建和管理里程碑）
- `issues` (创建和管理 issues）

## 🔗 相关资源

- `docs/ISSUE_WORKFLOW.md` - Issue 管理流程
- `docs/DEVELOPMENT.md` - 开发指南
- `CONTRIBUTING.md` - 贡献指南
- `.github/ISSUE_TEMPLATE/dev_task.md` - Issue 模板

---

**准备好后，将本 prompt 提供给 AI，AI 将执行完整的规划流程。**
