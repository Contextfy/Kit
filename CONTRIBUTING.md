# 贡献指南

感谢你有兴趣为 Contextfy/Kit 做出贡献！我们欢迎所有形式的贡献。

## 📋 目录

- [行为准则](#行为准则)
- [如何贡献](#如何贡献)
- [Issue 工作流程](#issue-工作流程)
- [开发流程](#开发流程)
- [代码规范](#代码规范)
- [提交规范](#提交规范)
- [Pull Request 流程](#pull-request-流程)

## 🤝 行为准则

- 尊重不同的观点和经验
- 接受建设性的批评
- 关注对社区最有利的事情
- 对其他社区成员表示同理心

## 🚀 如何贡献

### 方式一：接手现有 Issue

1. 访问 [Issues 页面](https://github.com/Contextfy/Kit/issues)
2. 筛选标记为 `status:ready` 的 Issue
3. 选择一个你感兴趣的 Issue
4. 在 Issue 中评论 `I'd like to work on this` 来认领任务
5. 等待维护者分配给你（assign）
6. 按照 Issue 中的要求开始开发

### 方式二：提出新的 Issue

如果你有新的功能想法或发现了 Bug：

1. 使用相应的 [Issue 模板](.github/ISSUE_TEMPLATE/) 创建 Issue
2. 清晰地描述问题或需求
3. 等待维护者审核和讨论

### 方式三：直接提交 PR

对于小的修复或改进：

1. Fork 本仓库
2. 创建你的特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交你的更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

## 📋 Issue 工作流程

详细的 Issue 管理指南，请参考 [ISSUE_WORKFLOW.md](./ISSUE_WORKFLOW.md)。

### Issue 生命周期

```
Open → In Review → Ready → In Progress → Done → Closed
```

### Issue 标签说明

- **Type**: `type:bug`, `type:feature`, `type:docs`, `type:refactor`
- **Priority**: `priority:critical`, `priority:high`, `priority:medium`, `priority:low`
- **Status**: `status:ready`, `status:in-progress`, `status:blocked`
- **Area**: `area:core`, `area:cli`, `area:server`, `area:web`
- **Complexity**: `complexity:small`, `complexity:medium`, `complexity:large`

## 💻 开发流程

### 开发前准备

1. Clone 仓库并添加 upstream
   ```bash
   git clone https://github.com/YOUR_USERNAME/Kit.git
   cd Kit
   git remote add upstream https://github.com/Contextfy/Kit.git
   ```

2. 创建特性分支
   ```bash
   git checkout -b feature/your-feature-name
   ```

3. 同步最新代码
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

### 代码要求

#### Rust 代码

- 遵循 [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- 使用 `cargo fmt` 格式化代码
- 通过 `cargo clippy` 检查
- 为公共 API 编写文档注释 (`///`)
- 使用 `anyhow::Result` 作为错误类型
- 单元测试覆盖率 >= 70%

#### JavaScript/TypeScript 代码

- 使用 2 空格缩进
- 使用分号
- 为函数添加 JSDoc 注释
- 遵循 ESLint 规则

### 测试要求

```bash
# 运行所有测试
cargo test

# 运行特定包的测试
cargo test -p contextfy-core

# 运行测试并显示输出
cargo test -- --nocapture

# 运行特定测试
cargo test test_name
```

### 构建检查

```bash
# 构建所有包
cargo build

# 构建 release 版本
cargo build --release

# 构建 Node.js bridge (需要 Node.js 环境)
cd packages/bridge
npm run build
```

## 📝 代码规范

### Rust

```rust
// ✅ 好的示例
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// 文档记录结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub content: String,
}

/// 解析文档
pub fn parse_document(input: &str) -> Result<Document> {
    // 实现...
}
```

### Commit 规范

使用 [Conventional Commits](https://www.conventionalcommits.org/) 格式：

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Type 类型:**
- `feat`: 新功能
- `fix`: Bug 修复
- `docs`: 文档更新
- `style`: 代码格式调整（不影响功能）
- `refactor`: 重构（不是新功能也不是修复）
- `perf`: 性能优化
- `test`: 添加或修改测试
- `chore`: 构建或辅助工具的变动

**示例:**

```bash
feat(core): add markdown parser with ast extraction
fix(cli): handle missing directory in build command
docs(readme): update installation instructions
refactor(storage): simplify file reading logic
```

## 🔀 Pull Request 流程

### PR 前检查清单

在提交 PR 前，确保：

- [ ] 代码通过所有测试 (`cargo test`)
- [ ] 代码格式化 (`cargo fmt`)
- [ ] 代码通过 clippy 检查 (`cargo clippy`)
- [ ] 更新了相关文档
- [ ] 添加了必要的测试
- [ ] PR 标题符合 commit 规范
- [ ] PR 描述清晰，包含相关 Issue 链接

### PR 描述模板

```markdown
## 变更说明
简要描述这个 PR 做了什么。

## 变更类型
- [ ] Bug 修复
- [ ] 新功能
- [ ] 重构
- [ ] 文档更新
- [ ] 性能优化

## 测试
描述如何测试这个变更：
- [ ] 手动测试通过
- [ ] 单元测试通过
- [ ] 集成测试通过

## 关联 Issue
Closes #(issue number)

## 截图
如果适用，添加截图展示变更效果。
```

### PR 审核流程

1. 提交 PR 后，自动 CI 会运行测试
2. 所有检查通过后，等待维护者 Code Review
3. 根据反馈修改代码
4. 审核通过后，合并到主分支

## 📚 获取帮助

如果你需要帮助：

- 查看 [开发文档](./DEVELOPMENT.md)
- 查看 [Issue](https://github.com/Contextfy/Kit/issues) 看是否有类似问题
- 在 Issue 中提问或讨论
- 加入 [QQ 群](https://jq.qq.com/): 1065806393

## 📄 许可

提交代码到本项目，表示你同意你的代码将根据 [MIT License](../LICENSE) 进行授权。
