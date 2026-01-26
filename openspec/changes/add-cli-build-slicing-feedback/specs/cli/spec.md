# CLI 规格

## 目的
CLI 规格定义了 Contextfy Kit 的命令行界面需求，包括 build、scout 和其他面向用户的命令。

## MODIFIED Requirements

### Requirement: Build 命令切片进度反馈
CLI build 命令 SHALL 在构建过程中显示每个切片的处理进度和汇总统计。

#### Scenario: 显示每个切片的 ID
- **当**系统处理每个切片时
- **则**显示格式化进度信息：
  - "[1] Slice ID: <uuid>"
  - "[2] Slice ID: <uuid>"
  - 以此类推
- **并且**使用从 1 开始的连续编号

#### Scenario: 显示每个文档的切片数量
- **当**文档被成功存储且包含切片时
- **则**显示："Stored: <title> (<n> slices)"
- **其中** <n> 是该文档中的切片数量

#### Scenario: 显示构建摘要
- **当**所有文档处理完成时
- **则**系统显示总结信息："找到 <doc_count> 个文档，<section_count> 个切片"
- **其中** <doc_count> 是处理的文档总数
- **并且** <section_count> 是所有文档中切片的总数

#### Scenario: 优雅处理解析错误
- **当**某个文档解析失败时
- **则**系统：
  - 显示错误信息："✗ Failed to parse <file_path>: <error>"
  - 继续处理其他文档
  - 在摘要中仅包含成功处理的文档计数
