# Issue 工作流程

本文档详细说明 Contextfy/Kit 项目使用 Issue 管理开发任务的工作流程，适用于多人协作的渐进式开发场景。

## 📋 目录

- [Issue 策略](#issue-策略)
- [Issue 生命周期](#issue-生命周期)
- [创建 Issue](#创建-issue)
- [认领 Issue](#认领-issue)
- [开发流程](#开发流程)
- [Issue 分类与标签](#issue-分类与标签)
- [Issue 模板](#issue-模板)
- [里程碑管理](#里程碑管理)

## 🎯 Issue 策略

### 核心原则

1. **任务拆分**: 大型功能拆解为多个小的、独立的 Issue
2. **模块化**: 每个 Issue 专注于一个模块或一个功能点
3. **可测试**: 每个 Issue 有明确的验收标准
4. **优先级清晰**: 使用 priority 标签明确任务优先级

### Issue 类型

| 类型 | 用途 | 模板 |
|------|------|------|
| **Feature Request** | 新功能建议 | `feature_request.md` |
| **Bug Report** | 报告 Bug | `bug_report.md` |
| **Development Task** | 具体开发任务（维护者创建）| `dev_task.md` |
| **Discussion** | 技术讨论 | `discussion.md` |
| **Documentation** | 文档改进 | `documentation.md` |

## 🔄 Issue 生命周期

```
Created → In Review → Ready → In Progress → Done → Closed
    ↑                                      ↓
    └────────────── Reopen ←──────────────────┘
```

### 各阶段说明

#### 1. Created (新建)

Issue 刚创建，标签为 `status:in-review`

**维护者操作:**
- 审核 Issue 内容是否清晰
- 确认是否接受该 Issue
- 添加合适的标签（priority, area）
- 更新为 `status:ready` 或关闭

#### 2. In Review (审核中)

Issue 正在被维护者审核

**审核标准:**
- ✅ 需求描述清晰
- ✅ 有明确的验收标准
- ✅ 技术上可行
- ✅ 优先级合理

#### 3. Ready (待认领)

Issue 已审核通过，等待开发者认领

**标签:** `status:ready`

**开发者操作:**
- 在 Issue 中评论 `I'd like to work on this` 认领任务

#### 4. In Progress (进行中)

Issue 已被认领，正在开发

**标签:** `status:in-progress`, `assigned: @username`

**开发者操作:**
- 创建分支: `git checkout -b issue-123-feature-name`
- 按照需求开发
- 定期更新进度

#### 5. Done (已完成)

PR 已提交并合并

**标签:** `status:done` (或移除 status 标签)

**操作:**
- 关闭 Issue
- 评论 `Closed via PR #456`

#### 6. Closed (已关闭)

Issue 已完成，标记为关闭

## 📝 创建 Issue

### 维护者创建开发任务

使用 `dev_task.md` 模板创建任务，确保包含：

1. **任务目标**: 一句话描述
2. **涉及模块**: 清晰标记受影响的模块
3. **需求详情**: 详细的功能描述
4. **接受标准**: 明确的完成条件
5. **技术要求**: 版本要求、风格要求
6. **相关资源**: OpenSpec、参考文档
7. **不需要做的事**: 明确排除范围，防止范围蔓延

**示例 Issue:**

```markdown
[TASK] Core Engine - Add BM25 ranking to search results

## 🎯 任务目标
为搜索结果添加 BM25 相关性排序，提升检索质量。

## 📦 涉及模块
- [x] core-engine
- [ ] cli
- [ ] server

## 📝 需求详情
- 实现 BM25 算法
- 在 `scout()` 方法中应用排序
- 保持现有 API 不变
```

### 社区提交 Issue

1. 使用合适的模板创建 Issue
2. 清晰描述问题或需求
3. 等待维护者审核

## 👥 认领 Issue

### 认领流程

1. 筛选 `status:ready` 的 Issue
2. 选择合适的 Issue（考虑优先级和复杂度）
3. 在 Issue 中评论

**评论格式:**
```
I'd like to work on this.
- 擅长: Rust, Tokio
- 预计完成时间: 2-3 天
```

4. 维护者会分配 Issue 给你

### 一次认领一个 Issue

为了避免超载，建议一次只认领 1-2 个 Issue，完成后再认领新的。

## 💻 开发流程

### 1. 准备工作

```bash
# 确保本地是最新的
git fetch upstream
git rebase upstream/main

# 创建分支（Issue #123）
git checkout -b issue-123-feature-name
```

### 2. 开发与测试

```bash
# 开发
# ... 编写代码 ...

# 格式化
cargo fmt

# 检查
cargo clippy

# 测试
cargo test
```

### 3. 提交 PR

```bash
# 提交
git add .
git commit -m "feat(core): add bm25 ranking"

# 推送
git push origin issue-123-feature-name

# 在 GitHub 创建 PR，关联 Issue
```

**PR 标题:**
```
feat(core): add bm25 ranking to search results (#123)
```

**PR 描述:**
```markdown
## 变更说明
实现了 BM25 排序算法，提升搜索结果相关性。

## 变更类型
- [x] 新功能

## 测试
- [x] 添加了单元测试
- [x] 所有测试通过

## 关联 Issue
Closes #123
```

### 4. 更新 Issue

PR 创建后，在 Issue 中评论：
```
PR created: #456
```

维护者会将 Issue 标签更新为 `status:in-progress`

## 🏷️ Issue 分类与标签

### Type 标签

| 标签 | 颜色 | 用途 |
|------|------|------|
| `type:bug` | 🔴 红色 | Bug 报告 |
| `type:feature` | 🔵 蓝色 | 新功能 |
| `type:enhancement` | 🟡 黄色 | 功能增强 |
| `type:docs` | 🟢 绿色 | 文档相关 |
| `type:discussion` | 🟣 紫色 | 讨议题 |
| `type:refactor` | 🟠 橙色 | 重构 |

### Priority 标签

| 标签 | 颜色 | 响应时间 |
|------|------|----------|
| `priority:critical` | 🔴 红色 | 24小时内 |
| `priority:high` | 🟠 橙色 | 3天内 |
| `priority:medium` | 🟡 黄色 | 1周内 |
| `priority:low` | 🟢 绿色 | 按需处理 |

### Status 标签

| 标签 | 用途 |
|------|------|
| `status:ready` | 已审核，等待认领 |
| `status:in-progress` | 正在开发 |
| `status:blocked` | 被阻塞 |
| `status:needs-review` | 需要审核 |

### Area 标签

标记影响的模块：
- `area:core` - 核心 Rust 引擎
- `area:cli` - 命令行工具
- `area:server` - Web 服务器
- `area:bridge` - FFI 桥接
- `area:web` - Web 界面
- `area:docs` - 文档
- `area:infra` - 基础设施

### Complexity 标签

预估工作量：
- `complexity:small` - 1-2 小时
- `complexity:medium` - 3-5 小时
- `complexity:large` - 1-2 天

### Special 标签

- `good first issue` - 适合新手

## 📚 Issue 模板使用

### Feature Request

**何时使用:**
- 提出新功能想法
- 建议改进现有功能

**包含内容:**
- 功能描述
- 使用场景
- 建议方案（可选）

### Bug Report

**何时使用:**
- 发现代码错误
- 功能不按预期工作

**包含内容:**
- Bug 描述
- 复现步骤
- 预期 vs 实际行为
- 环境信息
- 日志/截图

### Development Task

**何时使用:**
- 维护者分解大的功能为小任务
- 明确的开发需求

**包含内容:**
- 任务目标
- 涉及模块
- 需求详情
- 验收标准
- 技术要求

### Discussion

**何时使用:**
- 技术方案讨论
- 架构设计决策
- 不确定的技术问题

**包含内容:**
- 讨论主题
- 背景
- 问题列表
- 可能的方案

### Documentation

**何时使用:**
- 文档错误
- 需要补充文档
- 文档改进建议

**包含内容:**
- 文档类型
- 问题描述
- 建议的改进

## 🎯 里程碑管理

### 创建 Milestone

1. 为每个版本创建 Milestone (如 `v0.2.0`)
2. 设置截止日期（可选）
3. 描述里程碑目标

### 关联 Issue

- 将相关 Issue 添加到 Milestone
- 按优先级排序

### 跟踪进度

- 定期检查 Milestone 进度
- 调整 Issue 优先级
- 必要时拆分 Issue

### 完成 Milestone

当所有 Issue 完成后：

1. 关闭 Milestone
2. 发布新版本
3. 创建下一个 Milestone

## 🔍 Issue 筛选

### 常用筛选

```
# 待认领的 Issue
is:issue is:open label:"status:ready"

# 高优先级 Issue
is:issue is:open label:"priority:high"

# 特定模块 Issue
is:issue is:open label:"area:core"

# 适合新手的 Issue
is:issue is:open label:"good first issue"

# 我的 Issue
is:issue is:open assignee:@yourname
```

### GitHub Actions 自动化

可以配置 GitHub Actions 自动：

- 自动添加标签
- 自动回复 Issue
- 自动创建分支
- 自动检查 PR 状态

## 📊 统计与报告

### 每周 Issue 处理报告

维护者可以定期发布：

```markdown
# 本周 Issue 处理报告 (2026-01-20 ~ 2026-01-26)

## 新增
- 5 个新 Issue (2 feature, 2 bug, 1 discussion)

## 已完成
- 8 个 Issue 已关闭

## 进行中
- 3 个 Issue 正在开发

## 待认领
- 12 个 Issue 等待认领
  - 3 个高优先级
  - 5 个中优先级
  - 4 个低优先级
```

## 💡 最佳实践

### 对于维护者

1. **快速审核**: Issue 应在 24-48 小时内审核
2. **明确标签**: 为每个 Issue 打上正确的标签
3. **及时分配**: 认领后及时 assign
4. **定期清理**: 关闭已解决的 Issue
5. **积极反馈**: 在开发过程中提供帮助

### 对于开发者

1. **先沟通**: 不确定的地方先在 Issue 中讨论
2. **小步迭代**: 大任务拆解为多个 PR
3. **及时更新**: 开发受阻时及时更新 Issue
4. **质量优先**: 不要为了速度牺牲代码质量
5. **测试覆盖**: 新代码必须有测试

### 对于 Issue 创建者

1. **详细描述**: 越详细越好，包括复现步骤
2. **提供上下文**: 说明为什么需要这个功能
3. **参与讨论**: 维护者可能会提出问题
4. **耐心等待**: 维护者可能很忙

## 🔗 相关文档

- [贡献指南](../CONTRIBUTING.md)
- [开发指南](./DEVELOPMENT.md)
- [OpenSpec 指南](../openspec/AGENTS.md)

## ❓ 常见问题

**Q: 如何找到适合自己的 Issue？**

A: 筛选 `status:ready` 和 `good first issue`，查看 complexity 标签选择合适的任务。

**Q: 可以同时认领多个 Issue 吗？**

A: 可以，但建议不超过 2-3 个，确保能按时完成。

**Q: 开发受阻怎么办？**

A: 在 Issue 中评论说明问题，维护者会提供帮助。必要时可以暂时释放该 Issue。

**Q: PR 被拒绝怎么办？**

A: 不要灰心，仔细阅读 reviewer 的反馈，修改后重新提交。这是正常的协作过程。

**Q: 如何更改 Issue 优先级？**

A: 在 Issue 中评论说明理由，维护者会评估并调整。

---

有问题？在 GitHub Issues 中提问或加入 [QQ 群](https://jq.qq.com/): 1065806393
