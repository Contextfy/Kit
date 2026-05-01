# Change: Pragmatic Slice Architecture Refactoring

## Why

当前 KnowledgeStore 单体实现（1500+ 行）使检索、存储、桥接和编排逻辑高度耦合，导致改动风险高、AI 辅助开发上下文不稳定。

同时，LanceDB 基础设施细节外溢到业务路径，阻碍向量引擎替换与维度演进；Bridge 层存在异步生命周期造假、错误吞噬与 Option 假对象回填等历史问题，已成为系统稳定性与可维护性瓶颈。

## What Changes

- 建立 `kernel/` 共享内核，仅承载稳定全局类型（`Query`、`Hit`、`Score` 等），禁止任何底层引擎依赖渗透。
- 建立 `bridge/` FFI 护城河，拆分 `api.rs`、`runtime_guard.rs`、`dto.rs`、`error_map.rs`。
- 建立 `slices/vector/`，定义轻量 `VectorStoreTrait`，将 LanceDB 的 schema、connection、impl 物理隔离。
- 建立 `slices/bm25/` 与 `slices/hybrid/`，完成 BM25 检索与 RRF 融合编排切片。
- 引入 4 条强制架构红线：运行时策略统一、Hit 极小化、错误分层映射、Option 契约与反假对象测试。
- 按三期执行：先稳边界（Phase 1），再拆向量（Phase 2），最后拆检索融合并删除旧路径（Phase 3）。

## Impact

- Affected specs: `core-engine`, `bridge-layer`
- Affected code:
  - `packages/core/src/kernel/**`
  - `packages/core/src/bridge/**`
  - `packages/core/src/slices/vector/**`
  - `packages/core/src/slices/bm25/**`
  - `packages/core/src/slices/hybrid/**`
  - 历史 `KnowledgeStore` 与旧 bridge 相关实现
- **BREAKING**:
  - 公共 API 入口已迁移至新的门面模式：
    - 旧路径：直接使用 `KnowledgeStore` 已被删除
    - 新路径：必须通过 `SearchEngine` (packages/core/src/facade.rs) 或 `BridgeApi` (packages/core/src/bridge/api.rs)
  - Bridge 层 Node.js FFI 接口保持兼容（DTO 层隔离变更）
  - 内部模块结构重组，外部调用方需更新导入路径
