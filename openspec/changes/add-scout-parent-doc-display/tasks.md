## 1. Implementation
- [ ] 1.1 Add `parent_doc_title` field to `KnowledgeRecord` struct
- [ ] 1.2 Update `KnowledgeStore::add()` to store parent document title when creating slices
- [ ] 1.3 Update `KnowledgeStore::add()` fallback logic to set parent_doc_title for unsliced docs
- [ ] 1.4 Update `Brief` struct to include `parent_doc_title` field
- [ ] 1.5 Update `Retriever::scout()` to map parent_doc_title to Brief
- [ ] 1.6 Update CLI scout command to display "[parent_doc] section_title" format
- [ ] 1.7 Run cargo test to verify changes
- [ ] 1.8 Run cargo clippy and fix any warnings
