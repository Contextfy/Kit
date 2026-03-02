## 1. 环境准备与脚本开发

- [ ] 1.1 创建 `scripts/` 目录（如不存在）
- [ ] 1.2 编写 `scripts/fetch_bedrock_docs.sh` 脚本：
  - 浅克隆镜像仓库到临时目录（`git clone --depth 1 --filter=blob:none --sparse`）
  - 配置 sparse-checkout 仅拉取 `creator/ScriptAPI/` 目录
  - 精准筛选并复制以下主题相关的 Markdown 文件到 `docs/minecraft-bedrock/`：
    - **Block 相关**: `*.md` 匹配 `Block`、`CustomBlock` 关键字
    - **Player 相关**: `*.md` 匹配 `Player`、`EntityHealth` 关键字
    - **Entity 相关**: `*.md` 匹配 `Entity`、`Spawn` 关键字
    - **Dimension 相关**: `*.md` 匹配 `Dimension` 关键字
    - **Item 相关**: `*.md` 匹配 `Item`、`ItemStack`、`ItemComponent` 关键字
  - 清理临时克隆目录
  - 输出提取的文件数量和路径列表
- [ ] 1.3 测试脚本执行，确认提取约 22-25 篇文档

## 2. 构建配置增强

- [ ] 2.1 扩展 `contextfy.json` 结构，添加 `docs_path` 配置项：
  ```json
  {
    "name": "contextfy-project",
    "version": "0.1.0",
    "description": "A Contextfy knowledge base project",
    "docs_path": "docs/minecraft-bedrock"
  }
  ```
- [ ] 2.2 修改 `packages/cli/src/commands/init.rs`：生成包含 `docs_path` 的 contextfy.json 模板
- [ ] 2.3 修改 `packages/cli/src/commands/build.rs`：
  - 读取 contextfy.json 获取 `docs_path`（如不存在则回退到 `docs/examples`）
  - 更新错误提示信息，反映新的配置路径

## 3. Git 忽略配置

- [ ] 3.1 更新 `.gitignore`，添加构建产物目录：
  ```
  # Contextfy build artifacts
  .contextfy/
  ```

## 4. 文档导入与验证

- [ ] 4.1 执行 `scripts/fetch_bedrock_docs.sh` 拉取文档
- [ ] 4.2 验证 `docs/minecraft-bedrock/` 目录结构正确
- [ ] 4.3 手动检查若干文档，确认 Markdown 格式正常且为中文内容

## 5. 构建管线测试

- [ ] 5.1 运行 `cargo run -p contextfy-cli -- build`
- [ ] 5.2 验证 pulldown-cmark 成功解析中文 Markdown 和微软复杂标签（无 panic）
- [ ] 5.3 确认 `.contextfy/` 产物目录生成且不被 Git 追踪
- [ ] 5.4 运行 `contextfy scout` 测试检索功能（可选验证）

## 6. 质量门禁（The Trinity）

- [ ] 6.1 运行 `cargo fmt` 格式化代码
- [ ] 6.2 运行 `cargo clippy` 修复所有 lint 警告
- [ ] 6.3 运行 `cargo test` 确保所有单元测试通过
- [ ] 6.4 运行 `cargo build --release` 验证编译成功

## 7. 提交前检查

- [ ] 7.1 执行 `git status` 确认变更集：
  - 新增：`scripts/fetch_bedrock_docs.sh`
  - 新增：`docs/minecraft-bedrock/*.md`（约 22-25 个文件）
  - 修改：`packages/cli/src/commands/init.rs`
  - 修改：`packages/cli/src/commands/build.rs`
  - 修改：`.gitignore`
  - 无：`.contextfy/` 目录或其他构建垃圾
- [ ] 7.2 使用 `git diff` 审查代码变更，确认符合 Conventional Commits 风格
- [ ] 7.3 准备 commit message：`feat(docs): import bedrock script api core docs for mvp`
