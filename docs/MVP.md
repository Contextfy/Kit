# Contextfy/Kit MVP 规划文档: Project Bedrock

项目代号: Project Bedrock

版本目标: v0.1.0-MVP

核心定位: 验证 "Context Engine (Rust)" + "Skill Set (Prompt)" 协同开发垂直领域游戏的完整链路。

## 1. 核心愿景与 MVP 目标 (Vision & Objectives)

### 1.1 产品愿景

"Context as Knowledge, Prompt as Skill."

我们不仅要给 AI 一本“字典” (Contextfy)，还要给它一位“导师” (Skills)。MVP 将证明：通过 Contextfy 提供的准确知识快照，配合专门编写的工程化 Skills，AI 可以从零构建一个高质量的 Minecraft 基岩版 Addon，且零幻觉、符合最佳实践。

### 1.2 MVP 验收标准 (Definition of Done)

用户输入一句话需求：“帮我做一个红色的‘治疗石’方块，玩家站上去每秒回 2 点血。”

系统能够自动完成：

1. **工程创建**: 生成合规的 BP (Behavior Pack) 和 RP (Resource Pack) 目录结构及 manifest.json。
2. **资源注册**: 在 RP 中注册贴图和方块定义。
3. **逻辑实现**: 准确检索 `@minecraft/server` API，编写 Typescript 脚本实现回血逻辑。
4. **无人工干预**: 生成的代码无需用户修改即可在游戏中运行。

## 2. 系统架构: 双脑模型 (The Dual-Brain Architecture)

在 MVP 中，我们通过 CLI 将 Contextfy 的能力注入到 Claude Code/OpenCode 的对话上下文中。

### Layer 1: The Library (Contextfy/Kit - Rust Core)

- **职责**: 唯一的真理来源 (Source of Truth)。
- **数据源**: Microsoft Learn 官方文档 (Markdown) + `.d.ts` 类型定义。
- **产物**: `bedrock-std-v1.21.ctxpack`
- **核心动作**: `scout` (查摘要), `inspect` (看细节)。

### Layer 2: The Instructor (Claude Code Skills - XML/Prompt)

- **职责**: 工程与流程控制。
- **形式**: Skill。
- **核心动作**: 规划文件结构、功能实现、生成 UUID、强制检查版本等。

## 3. 功能规范 (Functional Specifications)

### 3.1 核心引擎侧 (Contextfy/Kit)

针对基岩版文档的特殊优化：

- **Ingestion (摄入)**:
  - 使用 `pulldown-cmark` 解析 Markdown。
  - **特殊处理**: 将 `.d.ts` 文件视为纯文本代码块进行索引，确保 `inspect` 能返回准确的 TS 类型定义。
- **Retrieval (检索)**:
  - **Alias 注入**: 在 `manifest.json` 中配置同义词。例如：用户搜 "回血" -> 自动匹配 "Heal", "Health", "Regeneration"。

### 3.2 技能侧 (Claude Skills Definition)

这是 MVP 的重头戏。我们需要编写一套 `bedrock-skills.xml`，包含以下三个核心 Tool/Skill：

#### Skill A: `Bedrock_Scaffolder` (脚手架专家)

- **解决痛点**: AI 经常搞不清 BP 和 RP 的依赖关系，或者忘记生成 UUID。
- **逻辑**:
  1. 创建 `BP/` 和 `RP/` 两个根目录。
  2. 生成两个 `manifest.json`，确保 BP 的 `dependencies` 指向 RP 的 UUID。
  3. **强制规则**: 所有的 `uuid` 字段必须是新生成的 UUID v4。

#### Skill B: `Contextfy_Bridge` (查阅专家)

- **解决痛点**: AI 瞎编 API。
- **逻辑**:
  1. 当用户要求实现具体功能（如“生成粒子”、“播放声音”）时，**禁止**凭记忆写代码。
  2. 必须先调用 `contextfy scout "关键词"`。
  3. 必须阅读返回的文档，确认 API 在 `1.21.50` 版本可用。

#### Skill C: `Component_Registry` (组件注册流)

- **解决痛点**: 忘记方块需要在两边（BP/RP）同时注册。
- **逻辑**:
  1. **Block Definition**: 在 `BP/blocks` 定义组件（如 `minecraft:light_emission`）。
  2. **Texture Mapping**: 在 `RP/textures/terrain_texture.json` 添加映射。
  3. **Client Definition**: 在 `RP/blocks.json` 关联贴图。

## 4. MVP 演示剧本 (The Script)

**环境**: MacOS Terminal, Contextfy CLI installed, Cursor/Claude Code.

Step 1: 准备 (Setup)

用户在终端运行：

Bash

```
# 1. 初始化 Contextfy 引擎并下载基岩版知识包
contextfy init --template bedrock-v1.21
contextfy build

# 2. 加载 Skills (假设通过系统 Prompt 注入)
export CLAUDE_SYSTEM_PROMPT=$(cat bedrock-skills.xml)
```

Step 2: 交互 (Interaction)

用户输入：

> "初始化一个项目，做一个叫 'Healing Stone' 的方块，玩家站上去每秒回血。"

**Step 3: AI 执行流 (AI Execution Flow)**

1. **AI (Thinking)**: "检测到初始化需求，调用 `Bedrock_Scaffolder`。"
   - *Action*: 创建文件结构，生成 UUIDs。
2. **AI (Thinking)**: "需要实现‘站上去回血’。我不确定现在的 Script API 怎么获取方块上的实体。调用 `Contextfy_Bridge`。"
   - *Action*: `contextfy scout "get entities on block script api"`
   - *Result*: Contextfy 返回 `Dimension.getEntitiesAtBlockLocation` 的文档摘要。
   - *Action*: `contextfy inspect <doc_id>`
   - *Result*: 获得完整的 TS 类型定义和示例。
3. **AI (Thinking)**: "需要实现方块的基础属性。调用 `Component_Registry`。"
   - *Action*: 生成 JSON 定义，设置 `minecraft:geometry` 和贴图引用。
4. **AI (Coding)**: 编写 `main.ts`，利用查到的 API 实现逻辑。

## 5. 关键产物：Bedrock Skills 模板 (Draft)

这是你需要集成到 MVP 中的 `bedrock-skills.xml` 的核心片段。

XML

```
<contextfy_skills>
    <instruction>
        You are an expert Minecraft Bedrock Addon Developer paired with the 'Contextfy' engine.
        Verify every API call using Contextfy. Do NOT guess APIs.
    </instruction>

    <skill name="scaffold_project">
        <description>Sets up the BP/RP folder structure with valid manifests.</description>
        <steps>
            <step>Create folders: 'BP', 'RP'.</step>
            <step>Generate unique UUIDs for header and modules.</step>
            <step>Ensure BP depends on '@minecraft/server' version '1.13.0-beta'.</step>
        </steps>
    </skill>

    <skill name="implement_script_logic">
        <description>Writes TypeScript logic using Contextfy to verify APIs.</description>
        <workflow>
            <step>Analyze the user requirement (e.g., "Heal player").</step>
            <step>Run command: `contextfy scout "[keywords]"`</step>
            <step>Read results. If specific syntax is needed, run: `contextfy inspect [id]`</step>
            <step>Write code ONLY using the APIs found in the documentation.</step>
        </workflow>
    </skill>
</contextfy_skills>
```

## 6. 为什么这个 MVP 能成？

1. **避开了 Tree-sitter**: 我们主要索引的是 Microsoft Learn 的 Markdown 文档，文本解析难度低，正好符合你 `pulldown-cmark` 的技术栈。
2. **验证了核心价值**: 基岩版开发最恶心的就是 UUID 管理和 API 经常变动。Skills 解决了 UUID，Contextfy 解决了 API 变动。
3. **可演示性极强**: 最终产出一个能玩的方块，比单纯展示“搜索结果”要直观得多。

------

### 下一步行动建议

1. **数据源**: 去 clone [MicrosoftDocs/minecraft-creator](https://github.com/MicrosoftDocs/minecraft-creator) 仓库，这是官方文档的 Markdown 源。
2. **Contextfy 配置**: 编写一个针对该仓库的 `contextfy.json`，配置 include 规则，只索引 Script API 和 JSON Reference 章节。
3. **Skill 调试**: 手动扮演 AI，测试上面的 Prompt 逻辑，确保它能引导出正确的文件结构。
