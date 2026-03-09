# Tasks: 向量存储与余弦相似度计算

## 任务列表

### 1. 数据模型升级 ✅
- [x] 在 `KnowledgeRecord` 结构体中添加 `embedding: Option<Vec<f32>>` 字段
- [x] 为 `embedding` 字段添加 `#[serde(default)]` 属性以兼容旧版 JSON
- [x] 验证现有测试仍然通过（向后兼容性）

### 2. 余弦相似度计算实现 ✅
- [x] 创建 `packages/core/src/embeddings/math.rs` 模块
- [x] 实现 `pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32` 函数
- [x] 实现归一化映射：`(raw_cosine + 1.0) / 2.0`
- [x] 实现除零保护：分母为 0 时返回 0.0
- [x] **额外稳定性防线**：
  - 极小分母保护（`denominator.abs() <= 1e-12`）
  - 非有限值检查（`!raw_cosine.is_finite()`）
  - 归一化结果 clamp（`.clamp(0.0, 1.0)`）
- [x] 在 `embeddings/mod.rs` 中导出 `math` 模块

### 3. 批量向量生成实现 ✅
- [x] 在 `EmbeddingModel` 中新增 `embed_batch(&self, texts: Vec<&str>) -> Result<Vec<Vec<f32>>>` 方法
- [x] 利用 FastEmbed 原生的批处理能力（直接调用 `inner.embed(texts, None)`）
- [x] 验证批量生成的向量维度均为 384
- [x] **性能优化**：预分配容量（`Vec::with_capacity()`）

### 4. 摄入管道集成 ✅
- [x] 修改 `KnowledgeStore::new()` 接受 `Option<Arc<EmbeddingModel>>` 参数
- [x] 修改 `KnowledgeStore::add()` 方法
- [x] 在添加文档前，收集所有切片的 `title + " " + summary` 拼接文本
- [x] 调用 `embed_batch()` 一次性生成所有向量
- [x] 将向量赋值给对应 `KnowledgeRecord` 的 `embedding` 字段
- [x] **鲁棒性增强**：使用 `get(idx)` 防止越界 panic，添加不匹配警告
- [x] 更新所有调用者（CLI build/scout、Server、测试）

### 5. 单元测试编写 ✅
- [x] 余弦相似度数学测试（12 个场景）
  - [x] 测试相同向量相似度为 1.0
  - [x] 测试正交向量相似度为 0.5（归一化后）
  - [x] 测试相反向量相似度为 0.0
  - [x] 测试零向量除零保护
  - [x] 测试空向量
  - [x] 测试不同长度向量
  - [x] 测试嵌入类向量（384 维）
  - [x] 测试负值向量
  - [x] **额外测试**：极小分母保护
  - [x] **额外测试**：结果严格在 [0,1] 范围（含极值案例）
  - [x] **额外测试**：clamp 防止浮点误差越界
  - [x] **额外测试**：部分相关向量
- [x] 批量向量生成测试（5 个场景）
  - [x] 测试空输入返回空结果
  - [x] 测试多条文本批量生成
  - [x] 验证返回向量数量与输入一致
  - [x] 测试单条文本与 embed_text 结果一致
  - [x] 测试 Unicode 支持
- [x] JSON 序列化测试
  - [x] 验证 `KnowledgeRecord` 序列化包含 `embedding` 数组
  - [x] 验证反序列化时旧版 JSON（无 embedding）可正常加载（`#[serde(default)]`）

### 6. 质量门禁验证 ✅
- [x] 运行 `cargo fmt` 格式化代码
- [x] 运行 `cargo clippy` 修复所有警告（仅 2 个预存在警告）
- [x] 运行 `cargo test` 确保所有测试通过（71 tests passed）
- [x] 验证测试覆盖率 >= 70%

### 7. 并发安全修复 ✅ **（关键修复）**
- [x] 将 `UnsafeCell<TextEmbedding>` 改为 `Mutex<TextEmbedding>`
- [x] 删除 `unsafe impl Send/Sync for EmbeddingModel`
- [x] 更新 `embed_text()` 和 `embed_batch()` 使用 `lock()`
- [x] 添加 mutex 获取失败错误处理

### 8. 代码质量优化 ✅ **（PR Review 修复）**
- [x] 修复循环内重复告警问题（storage/mod.rs:472-479）
- [x] 在循环前一次性检查向量数量与切片数量是否匹配
- [x] 长度不匹配时打印单次警告并将所有切片 `embedding` 设为 `None`
- [x] 使用 `get(idx)` 防止越界 panic
- [x] 在 spec.md 和 tasks.md 中明确优雅降级策略

## 架构决策

### 优雅降级策略 (Graceful Degradation)

**决策**：向量生成失败时系统应继续运行，文档仍可通过 BM25 被检索。

**实现**：
- `embedding: Option<Vec<f32>>` 字段允许为 `None`
- 当 `EmbeddingModel` 未注入时，所有切片的 `embedding` 为 `None`
- 当向量生成失败时，记录警告并将 `embedding` 设为 `None`
- 当向量数量与切片数量不匹配时，打印警告并将所有切片的 `embedding` 设为 `None`

**原因**：
- 系统的高可用性高于向量完整性
- BM25 全文检索仍然可以工作
- 避免因模型问题导致整个存储流程失败

## 实际结果

- ✅ **假批处理防线**：通过 - 直接调用底层 API，先收集后一次批调
- ✅ **数学严谨性与除零爆炸**：通过 - 公式正确 + 极小分母保护 + 非有限值检查 + clamp
- ✅ **向后兼容防线**：通过 - `#[serde(default)]` 确保旧版 JSON 可加载
- ✅ **依赖注入涟漪效应**：通过 - CLI/Server/测试已全部更新
- ✅ **维度与测试完整性**：通过 - 数学测试 12 个场景，存储测试 5 个场景

## 测试统计

- **单元测试总数**: 71 个（+3 数学稳定性测试）
- **测试通过率**: 100%
- **文档测试**: 8 个全部通过
- **集成测试**: 1 个（BM25 评估）通过
