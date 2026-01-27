# core-engine 规范变更

## ADDED Requirements

### Requirement: 智能摘要提取
The core engine SHALL extract summaries using the first semantic paragraph with special handling for code blocks, ensuring complete code signatures are preserved. 核心引擎 SHALL 使用首个语义段落提取摘要，并对代码块进行特殊处理，确保完整的代码签名被保留。

#### Scenario: 从普通 markdown 内容提取摘要
- **Given** 一个具有标准段落结构的 markdown 章节或文档
- **When** 提取摘要时
- **Then** 系统返回完整的第一个段落（直到第一个 `\n\n` 双换行符的所有文本）
- **And** 摘要保留完整的句子和代码块
- **And** 摘要去除首尾空白字符

#### Scenario: 保留以代码块开始的完整签名
- **Given** 一个以代码块开始的内容（例如函数签名）
- **When** 提取摘要时
- **Then** 系统包含**整个代码块**，从开始的 ``` 到关闭的 ```
- **And** 即使代码块内包含换行符，也不会在代码块中间截断
- **And** 摘要包含完整的函数/类签名及其返回类型
- **Example**: 输入 `"```rust\npub fn foo() -> Bar\n```\n\n说明..."` → 摘要包含完整的三行代码块

#### Scenario: 处理代码块内的双换行符
- **Given** 一个以代码块开始，且代码块内部包含双换行符的内容
- **When** 提取摘要时
- **Then** 系统不会在代码块内部的 `\n\n` 处截断
- **And** 摘要持续直到找到代码块的关闭标记 ```
- **And** 摘要包含完整的代码块内容

#### Scenario: 处理无段落分隔的内容
- **Given** 没有双换行符的 markdown 内容（例如单一长段落或代码片段）
- **When** 提取摘要时
- **Then** 系统回退到现有行为（截取前 200 字符）
- **And** 尝试在最后一个句子结束标点（`.`、`!`、`?`）处截断（如果存在）
- **And** 如果不存在句子结束标点，则在 200 字符处截断
- **And** 摘要去除首尾空白字符

#### Scenario: 处理超长段落（Wall of Text 保护）
- **Given** 一个超过 1000 字符的单一段落（例如粘贴的日志或从不换行的文本）
- **When** 提取摘要时
- **Then** 系统在 1000 字符处强制截断
- **And** 尝试在截断点附近的最后一个完整句子（`.`、`!`、`?`）处断开
- **And** 如果找不到句子结束符，直接在 1000 字符处截断并添加 `...` 后缀
- **And** 摘要不会超过 1000 字符（防止撑爆 UI 或数据库字段溢出）

#### Scenario: 处理空内容或极短内容
- **Given** 空内容或少于 50 字符的内容
- **When** 提取摘要时
- **Then** 系统按原样返回内容，不进行截断
- **And** 去除首尾空白字符
- **And** 不发生错误

## MODIFIED Requirements

### Requirement: Semantic Summarization
The system SHALL generate summaries based on content structure. 系统 SHALL 基于内容结构生成摘要。

#### Scenario: Summarizing Markdown
- **WHEN** the system generates a summary for a slice or document
- **THEN** it extracts the first complete paragraph (delimited by `\n\n`)
- **OR** it extracts the complete first code block if the section starts with one
- **OTHERWISE** it falls back to a 200-character truncation
