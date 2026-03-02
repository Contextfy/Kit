## MODIFIED Requirements

### Requirement: Knowledge Storage
The core engine SHALL store parsed documents and their semantic chunks in a file-system based JSON storage with atomic write guarantees and support for slice-level storage. 核心引擎 SHALL 使用基于文件系统的 JSON 存储来存储解析的文档和语义块，具有原子写入保证并支持切片级别存储。

#### Scenario: 搜索存储的切片（基于分词的匹配优化）

- **当**用户使用查询字符串搜索文档时
- **则**系统执行分词匹配（按空格分割查询为多个 token）
- **并且**计算每个记录的匹配分数：
  - title 中每个命中 token 计 2 分
  - summary 中每个命中 token 计 1 分
  - title 完全匹配所有 tokens 时额外 +3 分
  - title 部分匹配（至少 1 个且 ≥ 一半）时额外 +1 分
- **并且**按匹配分数降序排序结果
- **并且**跳过临时文件和目录（防御性检查）
- **并且**返回匹配的切片记录列表（按相关性排序）
- **并且**空查询时直接返回空结果（边界保护）

#### Scenario: 创建新的知识存储

- **当**用户使用目录路径初始化新的 `KnowledgeStore` 时
- **则**系统：
  - 在指定目录中创建存储文件夹
  - 自动清理上次崩溃遗留的 `.temp-*` 临时目录
  - 准备接受文档

#### Scenario: 存储切片文档（切片模式）

- **当**用户将包含多个切片的 `ParsedDoc` 添加到知识存储时
- **则**系统为每个切片创建独立的 `KnowledgeRecord`，每个记录包含：
  - `id`：切片的唯一 UUID
  - `title`：切片的标题（H2 标题）
  - `parent_doc_title`：父文档的标题（H1 标题或文件名）
  - `summary`：切片内容的前 200 个字符
  - `content`：切片的完整内容
  - `source_path`：原始文档文件路径
- **并且**使用原子性写入（临时文件 → 原子移动模式）确保数据完整性
- **并且**返回所有切片的 UUID 列表

#### Scenario: 存储无切片文档（回退模式）

- **当**用户将不包含切片的 `ParsedDoc` 添加到知识存储时
- **则**系统将整个文档作为单条记录存储（向后兼容）
- **并且**记录包含：
  - `title`：文档的完整标题
  - `parent_doc_title`：与 `title` 相同（保持一致性）
  - `source_path` 字段用于追溯原始文件

#### Scenario: 原子性写入失败回滚

- **当**写入过程中发生错误（磁盘满、权限问题等）
- **则**系统：
  - 删除所有已提交的文件（如果进入提交阶段）
  - 清理临时目录
  - 返回详细错误信息
- **确保**不会留下孤儿数据或不完整的文件

#### Scenario: 崩溃恢复

- **当**系统在写入过程中崩溃并重启
- **则**系统启动时自动扫描并清理所有 `.temp-*` 目录
- **确保**不会积累垃圾数据
