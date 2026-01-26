## 1. Implementation
- [x] 1.1 Add `parent_doc_title` field to `KnowledgeRecord` struct
- [x] 1.2 Update `KnowledgeStore::add()` to store parent document title when creating slices
- [x] 1.3 Update `KnowledgeStore::add()` fallback logic to set parent_doc_title for unsliced docs
- [x] 1.4 Update `Brief` struct to include `parent_doc_title` field
- [x] 1.5 Update `Retriever::scout()` to map parent_doc_title to Brief
- [x] 1.6 Update CLI scout command to display "[parent_doc] section_title" format
- [x] 1.7 Run cargo test to verify changes
- [x] 1.8 Run cargo clippy and fix any warnings
