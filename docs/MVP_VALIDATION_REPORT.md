# MVP 基准验证报告

**生成时间**:
**测试查询数量**: 5
**验收标准**: Top-1 准确率 ≥ 3/5

---

## 测试结果

### 查询 1: "create custom block"

**返回结果数**: 16


**Top-3 结果**:

1. **author: jakeshirley ms.author: jashir ms.service: minecraft-bedrock-edition ms.date: 02/10/2025 title: minecraft/server.BlockCustomComponent Interface description: Contents of the @minecraft/server.BlockCustomComponent class.**
   - **ID**: ``
   - **Summary**: # BlockCustomComponent Interface

2. **Classes that extend BlockComponent**
   - **ID**: ``
   - **Summary**: - [*BlockCustomComponentInstance*](BlockCustomComponentInstance.md)

3. **author: jakeshirley ms.author: jashir ms.service: minecraft-bedrock-edition ms.date: 02/10/2025 title: minecraft/server.BlockTypes Class description: Contents of the @minecraft/server.BlockTypes class.**
   - **ID**: ``
   - **Summary**: # BlockTypes Class


---

### 查询 2: "player health"

**返回结果数**: 10


**Top-3 结果**:

1. **author: jakeshirley ms.author: jashir ms.service: minecraft-bedrock-edition ms.date: 02/10/2025 title: minecraft/server.Player Class description: Contents of the @minecraft/server.Player class.**
   - **ID**: ``
   - **Summary**: # Player Class

2. **author: jakeshirley ms.author: jashir ms.service: minecraft-bedrock-edition ms.date: 02/10/2025 title: minecraft/server.EntityHealthComponent Class description: Contents of the @minecraft/server.EntityHealthComponent class.**
   - **ID**: ``
   - **Summary**: # EntityHealthComponent Class

3. **Classes that extend Player**
   - **ID**: ``
   - **Summary**: - [*@minecraft/server-gametest.SimulatedPlayer*](../../../scriptapi/minecraft/server-gametest/Simula...


---

### 查询 3: "spawn entity"

**返回结果数**: 19


**Top-3 结果**:

1. **author: jakeshirley ms.author: jashir ms.service: minecraft-bedrock-edition ms.date: 02/10/2025 title: minecraft/server.EntitySpawnAfterEvent Class description: Contents of the @minecraft/server.EntitySpawnAfterEvent class.**
   - **ID**: ``
   - **Summary**: # EntitySpawnAfterEvent Class

2. **author: jakeshirley ms.author: jashir ms.service: minecraft-bedrock-edition ms.date: 02/10/2025 title: minecraft/server.EntitySpawnAfterEventSignal Class description: Contents of the @minecraft/server.EntitySpawnAfterEventSignal class.**
   - **ID**: ``
   - **Summary**: # EntitySpawnAfterEventSignal Class

3. **author: jakeshirley ms.author: jashir ms.service: minecraft-bedrock-edition ms.date: 02/10/2025 title: minecraft/server.SpawnEntityOptions Interface description: Contents of the @minecraft/server.SpawnEntityOptions class.**
   - **ID**: ``
   - **Summary**: # SpawnEntityOptions Interface


---

### 查询 4: "dimension API"

**返回结果数**: 6


**Top-3 结果**:

1. **author: jakeshirley ms.author: jashir ms.service: minecraft-bedrock-edition ms.date: 02/10/2025 title: minecraft/server.DimensionType Class description: Contents of the @minecraft/server.DimensionType class.**
   - **ID**: ``
   - **Summary**: # DimensionType Class

2. **author: jakeshirley ms.author: jashir ms.service: minecraft-bedrock-edition ms.date: 02/10/2025 title: minecraft/server.Dimension Class description: Contents of the @minecraft/server.Dimension class.**
   - **ID**: ``
   - **Summary**: # Dimension Class

3. **author: jakeshirley ms.author: jashir ms.service: minecraft-bedrock-edition ms.date: 02/10/2025 title: minecraft/server.DimensionTypes Class description: Contents of the @minecraft/server.DimensionTypes class.**
   - **ID**: ``
   - **Summary**: # DimensionTypes Class


---

### 查询 5: "item registration"

**返回结果数**: 11


**Top-3 结果**:

1. **Classes that extend ItemComponent**
   - **ID**: ``
   - **Summary**: - [*ItemCompostableComponent*](ItemCompostableComponent.md)

2. **author: jakeshirley ms.author: jashir ms.service: minecraft-bedrock-edition ms.date: 02/10/2025 title: minecraft/server.ItemStack Class description: Contents of the @minecraft/server.ItemStack class.**
   - **ID**: ``
   - **Summary**: # ItemStack Class

3. **author: jakeshirley ms.author: jashir ms.service: minecraft-bedrock-edition ms.date: 02/10/2025 title: minecraft/server.ItemComponentTypeMap Type Alias description: Contents of the @minecraft/server.ItemComponentTypeMap type alias.**
   - **ID**: ``
   - **Summary**: # ItemComponentTypeMap Type Alias


---


## 验收标准对齐

| 指标 | 结果 | 状态 |
|------|------|------|
| Top-1 准确率 | 4/5 (80%) | ✅ 通过 |
| Top-3 准确率 | 5/5 (100%) | - |

**验收标准**: Top-1 准确率 ≥ 3/5
**实际结果**: 4/5
**最终状态**: ✅ **达标**

---

## 结论

### 当前算法分析

本次测试使用**基于分词的加权匹配算法**：
- 将查询字符串按空格分割为多个 tokens
- 计算加权分数：`title` 命中每个 token +2 分，`summary` 命中每个 token +1 分
- 额外奖励：`title` 完全匹配所有 tokens +3 分，部分匹配（至少 1 个且 ≥ 一半）+1 分
- 按匹配分数降序排序结果，分数相同时使用 ID 作为确定性 tie-breaker

### 算法局限性

1. **缺乏语义理解**：仅进行字面匹配，无法理解查询语义
2. **停用词干扰**：常见词（如 "API", "create"）匹配度高但区分度低
3. **无法处理同义词**：无法识别 "spawn" = "create" 或 "health" = "hp"
4. **排序权重单一**：仅基于 token 命中数，不考虑词频、位置、重要性等因素
5. **长尾查询脆弱**：多词查询中任何一个词不匹配都会导致分数下降

### 迫切需要改进

**当前算法极其脆弱，无法满足生产环境检索质量要求。迫切需要引入：**

1. **Tantivy 全文检索引擎**：提供高性能的倒排索引和 BM25 排序算法
2. **BM25 排序算法**：考虑词频（TF）和文档频率（IDF），提升相关性排序质量
3. **停用词过滤**：排除无意义的常见词，提升匹配精度
4. **词干提取和词形归一化**：处理词汇变形（如 "spawn" = "spawning" = "spawned"）
5. **向量语义搜索**：结合嵌入模型实现语义级检索，理解查询意图

### 建议下一步

- [ ] 集成 Tantivy 作为全文检索引擎
- [ ] 实现 BM25 排序算法
- [ ] 添加停用词过滤和词干提取
- [ ] 评估并集成向量嵌入模型（如 BGE-small-en）
- [ ] 实现混合检索（BM25 + 向量相似度）

---

**报告生成时间**: 2026-03-02 16:30:26 UTC
**测试环境**: Contextfy/Kit MVP
**算法版本**: 基于分词的字符串匹配 v1.0
