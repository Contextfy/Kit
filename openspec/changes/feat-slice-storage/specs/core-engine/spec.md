## MODIFIED Requirements

### Requirement: Granular Storage
The system SHALL store knowledge at the slice level rather than document level. 系统 SHALL 在切片级别而非文档级别存储知识。

#### Scenario: Ingesting Sliced Document (导入切片文档)
- **WHEN** a document containing semantic slices is added
- **当**包含语义切片的文档被添加时
- **THEN** the storage engine creates individual records for each slice
- **则**存储引擎为每个切片创建独立记录
- **AND** records the original file path in `source_path` field
- **并且**在 `source_path` 字段中记录原始文件路径
- **AND** assigns a unique ID to each slice record
- **并且**为每个切片记录分配唯一 ID

#### Scenario: Legacy Document Fallback (旧版文档回退)
- **WHEN** a document without semantic slices is added
- **当**没有语义切片的文档被添加时
- **THEN** the storage engine stores the entire document as a single record
- **则**存储引擎将整个文档作为单条记录存储
- **AND** records the original file path in `source_path` field
- **并且**在 `source_path` 字段中记录原始文件路径

## ADDED Requirements

### Requirement: Source Path Tracking
The system SHALL track the original file path for each stored slice record. 系统 SHALL 为每个存储的切片记录跟踪原始文件路径。

#### Scenario: Storing Slice with Source Path (使用源路径存储切片)
- **WHEN** a slice is stored in the knowledge base
- **当**切片被存储到知识库时
- **THEN** the record includes a `source_path` field containing the original file path
- **则**记录包含 `source_path` 字段，存储原始文件路径
- **AND** this field is searchable and retrievable
- **并且**该字段可搜索和检索
