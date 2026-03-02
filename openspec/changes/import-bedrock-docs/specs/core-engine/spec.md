## ADDED Requirements

### Requirement: 中文 Markdown 兼容性验证
核心引擎 SHALL 验证其对中文技术文档和微软复杂 Markdown 标签的解析稳定性，确保在生产环境中不会因解析器 panic 导致服务中断。

#### Scenario: 解析包含中文字符的 Markdown 文档
- **当**系统解析包含 UTF-8 编码中文字符的 Markdown 文件时
- **则**pulldown-cmark parser 应成功解析文档结构
- **并且**正确提取标题、段落、代码块等元素
- **并且**不产生解析错误或 panic

#### Scenario: 处理微软复杂标签结构
- **当**系统解析包含复杂嵌套结构的 Markdown 文档时（例如多层级的列表、表格、代码块嵌套）
- **则**parser 应优雅处理这些结构
- **并且**提取的语义块保持结构完整性
- **并且**不会因标签嵌套深度导致栈溢出或性能问题

#### Scenario: 构建基岩版文档知识库
- **当**系统批量处理约 22-25 篇 Minecraft Bedrock Script API 文档时
- **则**所有文档应成功解析并存储到知识库
- **并且**`contextfy build` 命令完成时显示成功处理的文档数量
- **并且**生成的切片可通过 `contextfy scout` 正常检索
