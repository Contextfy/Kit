# Pragmatic Slice Architecture Refactoring

## Title & Meta

- Proposal Name: Pragmatic Slice Architecture Refactoring
- Status: Proposed
- Date: 2026-03-18
- Scope: KnowledgeStore decomposition, Bridge hardening, LanceDB isolation

## Motivation

当前架构在可维护性、可演进性与跨语言边界安全性上已出现系统性阻塞，主要体现在以下三类问题：

1. 巨型存储类导致上下文失焦

- 现有 KnowledgeStore 已超过 1500 行，检索、存储、编排、桥接语义交织在单体实现中。
- 人工维护与 AI 辅助编程均难以建立稳定局部上下文，导致改动风险和回归成本持续上升。

2. LanceDB 强依赖污染业务编排

- 向量基础设施细节（schema、连接、驱动行为）泄漏到业务路径，破坏模块边界。
- 业务流程与底层引擎耦合，直接阻碍向量维度调整、驱动替换和故障隔离。

3. Bridge 层存在历史遗留缺陷

- N-API 桥接路径中存在异步生命周期造假，运行时调用策略不一致。
- 错误处理中出现吞噬与重写，根因链路不可追踪。
- Option 语义在跨层传递中被“假对象回填”，制造逻辑歧义与数据污染。

结论：需要以“实用主义切片（Pragmatic Slice）”方式进行分层重构，先稳定边界，再逐步切出能力，最终清除历史耦合与技术债。

## Proposed Architecture

目标是将系统拆分为“稳定内核 + 桥接护城河 + 能力切片”，并建立可替换的基础设施边界。

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
    │   ├── trait.rs
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

职责边界定义如下：

1. kernel/：共享内核

- 仅包含跨切片稳定的全局类型与契约（如 Query, Hit, Score）。
- 严禁引入任何外部引擎依赖（无 Arrow、无 Tantivy、无 LanceDB）。
- 作为编排层与桥接层的唯一公共语义基座。

2. bridge/：FFI 护城河

- api.rs：对外门面，统一暴露跨语言调用入口。
- runtime_guard.rs：异步运行时守护，统一同步/异步 FFI 调用策略。
- dto.rs：桥接 DTO 层，与内部类型物理隔离，避免泄漏实现细节。
- error_map.rs：分层错误映射，保留根因链路，输出桥接友好错误。

3. slices/vector/：向量切片

- 定义轻量 VectorStoreTrait，向上仅暴露稳定接口。
- LanceDB 相关 schema、connection、impl 全部物理隔离在该切片内。
- 保证向量维度、索引参数、底层引擎可替换，不影响业务编排层。

4. slices/bm25/ 与 slices/hybrid/：检索与融合切片

- bm25/：负责倒排检索能力与查询执行。
- hybrid/：负责 RRF 融合、召回编排与排序策略。
- 两者仅依赖 kernel 契约，不反向依赖向量实现细节。

## Mandatory Constraints

**以下 4 条为强制架构红线，是后续所有代码生成与评审的铁律。任何违反都视为架构级缺陷。**

**规则一：Runtime 调用策略必须统一。**

- 必须明确区分同步 FFI 与异步 FFI。
- **绝对禁止**在已有 Tokio 上下文时二次调用 block_on。
- 同步 FFI 必须走 runtime guard 的阻塞接口；异步 FFI 必须直接 await。

**规则二：Hit 对象必须极致最小化。**

- kernel::types::Hit 只能存放跨切片稳定字段（如 id, score）。
- **绝对禁止**将底层向量原文、Arrow 批次对象或 Tantivy 文档对象塞入 Hit。
- 内核类型不得被底层库反向污染。

**规则三：错误模型必须分层映射，禁止吞噬。**

- 必须定义并区分 DomainError、InfraError、BridgeError。
- 桥接层只做错误映射，不做语义重写。
- 必须保留根因链路（source chain）；**严禁**将所有错误统一 .to_string() 导致上下文丢失。

**规则四：Option 契约与“反假对象”测试。**

- 底层查不到数据时必须返回 Ok(None)。
- Bridge 层必须补充测试，断言 Option::None 的正确传递。
- 必须包含负向测试，防范“返回全部字段为空字符串的假对象”。

## Execution Order

采用三期安全落地策略，遵循“前一期不稳，后一期不动”的执行原则。

### Phase 1: 稳定内核与边界

目标：先锁定语义与跨语言边界，避免重构过程继续扩散耦合。

- 建立 kernel/types（Query, Hit, Score 等稳定类型）。
- 建立 bridge 三件套：runtime_guard、dto、error_map。
- 统一 FFI 调用路径与错误映射路径。
- 增补 Bridge 层 Option/None 与反假对象测试。

验收门槛：

- Runtime 调用策略通过测试验证（无嵌套 block_on）。
- Bridge 错误链路可追踪到根因。
- Option::None 端到端可验证，无假对象回填。

### Phase 2: 切出向量切片

目标：从业务编排中剥离 LanceDB 细节，实现基础设施可替换。

- 建立 slices/vector/ 目录与 VectorStoreTrait。
- 将 LanceDB schema、connection、impl 全量迁入 vector 切片。
- 编排层改为依赖 trait，不直接引用 LanceDB 具体实现。

验收门槛：

- 业务层不再直接依赖 LanceDB 类型。
- 向量维度与引擎替换影响面限定在 vector 切片内部。

### Phase 3: 切出 BM25 与 Hybrid 切片并清理遗留

目标：完成搜索能力模块化与最终调度闭环，删除旧单体路径。

- 建立 slices/bm25/ 与 slices/hybrid/，迁移检索与 RRF 融合逻辑。
- 完成统一搜索调度编排与跨切片结果聚合。
- 删除旧 KnowledgeStore 巨型耦合实现与废弃桥接分支。

验收门槛：

- 检索链路仅通过切片编排协作。
- 历史耦合路径可追溯删除，架构边界清晰且可测试。
- 回归测试覆盖核心查询、错误映射、Option 契约与融合结果稳定性。
