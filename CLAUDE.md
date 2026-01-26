<!-- OPENSPEC:START -->
# OpenSpec Instructions

These instructions are for AI assistants working in this project.

Always open `@/openspec/AGENTS.md` when the request:
- Mentions planning or proposals (words like proposal, spec, change, plan)
- Introduces new capabilities, breaking changes, architecture shifts, or big performance/security work
- Sounds ambiguous and you need the authoritative spec before coding

Use `@/openspec/AGENTS.md` to learn:
- How to create and apply change proposals
- Spec format and conventions
- Project structure and guidelines

Keep this managed block so 'openspec update' can refresh the instructions.

<!-- OPENSPEC:END -->

# AI 开发工作流指南

本指南为 AI 助手在 Contextfy/Kit 项目中开发的快速参考。

## 核心工作流

```
Issue 认领 → 创建分支 → 项目理解 → OpenSpec Proposal → 实现变更 → 测试 → 提交 PR → Review → 归档 → Merge
```

## 关键步骤

1. **创建分支**
   ```bash
   git checkout -b issue-<number>-<short-desc>
   ```

2. **项目理解**
   - 阅读 Issue，明确任务目标
   - 快速了解项目：README.md, docs/PRD.md, docs/Architecture.md
   - 检查 OpenSpec 状态：`openspec list` 和 `openspec list --specs`

3. **创建 Proposal**
   - 使用 `/openspec:proposal` 命令，它会引导你完成整个流程

4. **实现变更**
   - 使用 `/openspec:apply` 开始实现
   - 遵循代码规范：`cargo fmt` + `cargo clippy`
   - 编写单元测试（覆盖率 >= 70%）

5. **测试与提交**
   - 运行测试：`cargo test`
   - 使用 `/commit` 提交代码

6. **提交 PR**
   - 推送代码：`git push origin issue-xxx`
   - 创建 PR，在描述中链接 issue：`Closes #<issue-number>`

7. **修复与归档**
   - 根据 Review 反馈修改，`/commit` + `git push` 更新 PR
   - PR 通过后，`/openspec:archive <change-id> --yes` 归档
   - 在 Issue 中评论：`Closed via PR #<pr-number>`

## 文档索引

| 文档 | 用途 |
|------|------|
| README.md | 项目概况、核心特性、快速开始 |
| docs/PRD.md | 产品需求、目标用户 |
| docs/Architecture.md | 系统架构、模块划分 |
| CONTRIBUTING.md | 代码规范、测试要求 |
| docs/ISSUE_WORKFLOW.md | Issue 生命周期、标签说明 |

## 常见问题

**Q: 开发受阻怎么办？**
- 在 Issue 中详细说明问题：`Blocked on: [具体问题]`
- 等待维护者指导

**Q: 如何处理 merge 冲突？**
```bash
git fetch upstream
git rebase upstream/main
# 解决冲突后
git rebase --continue
git push origin issue-xxx --force-with-lease
```
