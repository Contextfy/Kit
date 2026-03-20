# Design: Pragmatic Slice Architecture Refactoring

## Context

系统当前存在三类架构债务：

1. 单体存储类膨胀导致语义边界模糊。
2. LanceDB 细节侵入业务编排导致引擎不可替换。
3. Bridge 层异步与错误模型不稳定导致跨语言调用风险上升。

本设计以“实用主义切片”为原则，优先收敛跨切片契约与边界，再渐进拆分能力。

## Target Architecture

```text
packages/core/src/
├── kernel/
│   ├── mod.rs
│   └── types.rs
├── bridge/
│   ├── mod.rs
│   ├── api.rs
│   ├── runtime_guard.rs
│   ├── dto.rs
│   └── error_map.rs
└── slices/
    ├── vector/
    │   ├── mod.rs
    │   ├── trait_.rs
    │   ├── schema.rs
    │   ├── connection.rs
    │   └── lancedb_impl.rs
    ├── bm25/
    │   ├── mod.rs
    │   └── service.rs
    └── hybrid/
        ├── mod.rs
        └── orchestrator.rs
```

### Responsibility Boundaries

- `kernel/`: 共享稳定类型，禁止引入 Arrow、Tantivy、LanceDB。
- `bridge/`: FFI 护城河，封装运行时策略、DTO 隔离与错误映射。
- `slices/vector/`: 向量存储抽象与 LanceDB 实现隔离点。
- `slices/bm25/` + `slices/hybrid/`: 检索与融合编排切片。

## Mandatory Constraints (Architecture Red Lines)

**规则一：Runtime 调用策略必须统一。**

- 区分同步 FFI 与异步 FFI。
- 禁止在已有 Tokio 上下文中再次 `block_on`。
- 同步 FFI 使用 runtime guard 阻塞接口；异步 FFI 直接 `await`。

**规则二：Hit 对象必须极致最小化。**

- `kernel::types::Hit` 仅包含稳定字段（如 `id`, `score`）。
- 禁止塞入向量原文、Arrow 批次对象、Tantivy 文档对象。

**规则三：错误模型必须分层映射，禁止吞噬。**

- 定义并区分 `DomainError`、`InfraError`、`BridgeError`。
- Bridge 层仅映射错误，不重写语义。
- 保留根因链路，禁止统一 `.to_string()` 丢失上下文。

**规则四：Option 契约与反假对象测试。**

- 未命中数据必须返回 `Ok(None)`。
- Bridge 层必须测试 `Option::None` 的端到端传递。
- 增加负向测试，防止“字段全空字符串的假对象”被返回。

## Execution Plan

### Phase 1: Stabilize Kernel and Bridge

- 建立 `kernel/types`。
- 建立 `bridge` 三件套（`runtime_guard`、`dto`、`error_map`）。
- 统一 FFI 调用策略与错误映射。
- 补全 Option/None 与反假对象测试。

### Phase 2: Extract Vector Slice

- 建立 `slices/vector/` 与 `VectorStoreTrait`。
- 迁移 LanceDB schema/connection/impl 到切片内。
- 上层编排仅依赖 trait。

### Phase 3: Extract BM25 + Hybrid and Remove Legacy

- 建立 `slices/bm25/` 和 `slices/hybrid/`。
- 迁移 BM25 检索与 RRF 编排。
- 删除旧 KnowledgeStore 耦合路径与废弃 bridge 分支。

## Risks and Mitigations

- 风险：迁移期双路径并存导致行为漂移。
  - 缓解：阶段化切流与回归测试基线对比。
- 风险：错误模型改造导致对外错误码变化。
  - 缓解：Bridge 层保持映射稳定，增加兼容测试。
- 风险：异步边界调整引入死锁或 panic。
  - 缓解：runtime guard 覆盖单元测试与并发场景测试。
