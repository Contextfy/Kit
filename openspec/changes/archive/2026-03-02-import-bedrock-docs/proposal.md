# Change: Import Minecraft Bedrock Script API 核心文档

## Why

Issue #6 要求导入 Minecraft 基岩版官方文档作为测试数据源，以验证 Contextfy 引擎解析真实技术文档的能力，并为 Issue #7 的查询场景提供高质量的垂直领域知识基础。

直接全量导入会导致 Token 消耗过大且包含大量不相关内容。采用精准提取策略，仅筛选与核心游戏开发场景（方块、玩家、实体、维度、物品）强相关的 ~22-25 篇文档，既能覆盖 MVP 验证需求，又保持数据集轻量化。

## What Changes

- **新增文档源**：从 `https://github.com/Contextfy/minecraft-creator-zh-cn` 浅克隆并提取 Script API 核心文档
- **精准提取策略**：编写脚本从 `creator/ScriptAPI` 目录筛选以下 5 个主题相关的文档（约 22-25 篇）：
  1. Block / CustomBlock（方块定义与自定义）
  2. Player / EntityHealth（玩家实体与血量系统）
  3. Entity Spawn（实体生成机制）
  4. Dimension（维度 API）
  5. Item / ItemStack（物品注册）
- **存储路径规范化**：提取的文档集放入项目 `docs/minecraft-bedrock/` 目录（符合项目文档目录约定）
- **构建配置更新**：更新根目录 `contextfy.json`，配置文档源指向 `docs/minecraft-bedrock/`
- **构建产物隔离**：将生成的 `.contextfy/` 产物目录添加到 `.gitignore`，防止构建垃圾污染 Git 历史

## Impact

- **Affected specs**:
  - `cli`: build 命令需要支持可配置的文档源目录（通过 contextfy.json 或命令行参数）
  - `core-engine`: 需要验证 pulldown-cmark 对中文 Markdown 和微软复杂标签的解析稳定性
- **Affected code**:
  - `packages/cli/src/commands/build.rs`: 修改硬编码的 `docs/examples` 路径为可配置路径
  - `packages/cli/src/commands/init.rs`: init 命令生成的 contextfy.json 应包含 docs_path 配置项
  - `.gitignore`: 添加 `.contextfy/` 目录
- **新增脚本**：`scripts/fetch_bedrock_docs.sh` 用于自动化提取文档
- **测试数据**：`docs/minecraft-bedrock/` 目录包含 ~22-25 篇精选 Script API Markdown 文档

## Non-Goals

- 不实现完整的模板系统（`contextfy init --template` 留待后续）
- 不实现复杂的文档过滤规则（本次采用硬编码的文件名匹配）
- 不修改核心解析逻辑（仅验证现有 parser 对真实文档的兼容性）
