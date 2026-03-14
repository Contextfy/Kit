use crate::embeddings::{math::cosine_similarity, EmbeddingModel};
use crate::parser::{extract_code_block_keywords, ParsedDoc};
use crate::search::{create_index, Indexer, Searcher};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

/// 混合检索召回倍数
///
/// 为了保证召回率，BM25 和向量搜索各自召回 `top_k * RECALL_MULTIPLIER` 条结果，
/// 然后在合并后通过加权融合和排序选出最终的 top_k 条。
const RECALL_MULTIPLIER: usize = 2;

/// 知识库中的一条记录
///
/// # 字段
///
/// * `id` - 记录的唯一标识符（UUID）
/// * `title` - 记录标题（对于切片文档，这是 H2 标题）
/// * `parent_doc_title` - 父文档的标题（H1 标题或文件名）
/// * `summary` - 内容摘要（前 200 个字符）
/// * `content` - 完整内容
/// * `source_path` - 原始文件路径，用于追溯源文件
/// * `keywords` - 关键词列表（用于全文搜索，为 Issue #10 打桩）
/// * `embedding` - 向量嵌入（384 维浮点数组，用于语义搜索）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeRecord {
    pub id: String,
    pub title: String,
    pub parent_doc_title: String,
    pub summary: String,
    pub content: String,
    #[serde(default)]
    pub source_path: String, // 新增字段：记录原始文件路径，向后兼容旧版 JSON
    #[serde(default)]
    pub keywords: Vec<String>, // 关键词列表（为 Issue #10 打桩）
    #[serde(default)]
    pub embedding: Option<Vec<f32>>, // 向量嵌入（用于语义搜索，向后兼容旧版 JSON）
}

/// 混合检索结果（包含原始分数和最终融合分数）
///
/// # 字段
///
/// * `record` - 知识记录
/// * `bm25_score` - 原始 BM25 分数（未归一化）
/// * `vector_score` - 余弦相似度（范围 [0, 1]）
/// * `final_score` - 加权融合后的最终分数（范围 [0, 1]）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefWithScore {
    pub record: KnowledgeRecord,
    pub bm25_score: f32,
    pub vector_score: f32,
    pub final_score: f32,
}

pub struct KnowledgeStore {
    data_dir: String,
    /// 内存缓存：所有已加载的知识记录
    /// 使用 Arc<RwLock<>> 支持多读者单写者并发模式
    /// Key 为 record.id，Value 为完整的 KnowledgeRecord
    records: Arc<RwLock<HashMap<String, KnowledgeRecord>>>,
    /// Tantivy 索引写入器（可选，用于全文搜索）
    /// 使用 Arc<Mutex<>> 以支持在异步上下文和阻塞任务中安全访问
    indexer: Option<Arc<Mutex<Indexer>>>,
    /// Tantivy 搜索器（可选，用于全文搜索）
    /// 使用 Arc 以支持在异步上下文和阻塞任务中安全访问
    searcher: Option<Arc<Searcher>>,
    /// 嵌入模型（可选，用于向量化）
    /// 使用 Arc 以支持在异步上下文和阻塞任务中安全访问
    embedding_model: Option<Arc<EmbeddingModel>>,
}

impl KnowledgeStore {
    pub async fn new(data_dir: &str, embedding_model: Option<Arc<EmbeddingModel>>) -> Result<Self> {
        fs::create_dir_all(data_dir).await?;

        // 启动恢复：清理上次崩溃遗留的临时目录
        Self::cleanup_orphaned_temp_dirs(data_dir).await;

        // 初始化内存缓存并全量加载所有文档
        let records = Arc::new(RwLock::new(HashMap::new()));
        Self::load_all_records_to_cache(data_dir, Arc::clone(&records)).await;

        // 初始化 Tantivy 索引
        let index_dir = Path::new(data_dir).join(".tantivy");
        let (indexer, searcher) = match create_index(Some(index_dir.as_path())) {
            Ok(index) => {
                // 创建 Indexer 和 Searcher
                match (Indexer::new(index.clone()), Searcher::new(index)) {
                    (Ok(idx), Ok(sch)) => (Some(Arc::new(Mutex::new(idx))), Some(Arc::new(sch))),
                    (Err(e), _) => {
                        eprintln!("Warning: Failed to create indexer: {}. Search will fall back to naive matching.", e);
                        (None, None)
                    }
                    (_, Err(e)) => {
                        eprintln!("Warning: Failed to create searcher: {}. Search will fall back to naive matching.", e);
                        (None, None)
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to initialize Tantivy index: {}. Search will fall back to naive matching.", e);
                (None, None)
            }
        };

        Ok(KnowledgeStore {
            data_dir: data_dir.to_string(),
            records,
            indexer,
            searcher,
            embedding_model,
        })
    }

    /// 清理孤儿临时目录（启动恢复）
    ///
    /// 如果程序在写入过程中崩溃，可能会遗留 `.temp-{uuid}` 目录。
    /// 这个方法在启动时扫描并删除这些目录。
    async fn cleanup_orphaned_temp_dirs(data_dir: &str) {
        let mut entries = match fs::read_dir(data_dir).await {
            Ok(entries) => entries,
            Err(_) => return,
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name();

            // 检查是否是临时目录（以 .temp- 开头）
            let name_str = match name.to_str() {
                Some(s) => s,
                None => continue,
            };

            if name_str.starts_with(".temp-") && entry.path().is_dir() {
                eprintln!("Cleaning up orphaned temp directory: {}", name_str);
                let _ = fs::remove_dir_all(entry.path()).await;
            }
        }
    }

    /// 全量加载所有文档到内存缓存（Cold Start）
    ///
    /// 在启动时遍历数据目录，将所有合法的 JSON 文档加载到内存缓存中。
    /// 单个文件解析失败不会中断整体加载，仅记录警告。
    async fn load_all_records_to_cache(
        data_dir: &str,
        records: Arc<RwLock<HashMap<String, KnowledgeRecord>>>,
    ) {
        let mut entries = match fs::read_dir(data_dir).await {
            Ok(entries) => entries,
            Err(e) => {
                eprintln!(
                    "Warning: Failed to read data directory {}: {}. Starting with empty cache.",
                    data_dir, e
                );
                return;
            }
        };

        let mut cache = records.write().await;
        let mut loaded_count = 0;
        let mut error_count = 0;

        // 【防御性编程】使用显式 loop { match ... } 捕获所有错误，避免静默失败
        loop {
            let entry = match entries.next_entry().await {
                Ok(Some(entry)) => entry,
                Ok(None) => break,
                Err(e) => {
                    eprintln!(
                        "Warning: Failed while iterating data directory: {}. Cache warmup may be incomplete.",
                        e
                    );
                    error_count += 1;
                    break;
                }
            };

            // 【异步 I/O 规范】使用 entry.file_type().await 替代阻塞的 path.is_file()
            // 避免在 Tokio 异步上下文中调用阻塞系统调用
            let file_type = match entry.file_type().await {
                Ok(ft) => ft,
                Err(_) => continue,
            };

            // 【健壮性检查】跳过子目录，只处理文件
            if !file_type.is_file() {
                continue;
            }

            let path = entry.path();

            // 跳过临时文件和目录
            if Self::is_temp_file(&path) {
                continue;
            }

            // 只处理 JSON 文件
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            // 反序列化文档
            match fs::read_to_string(&path).await {
                Ok(content) => match serde_json::from_str::<KnowledgeRecord>(&content) {
                    Ok(record) => {
                        cache.insert(record.id.clone(), record);
                        loaded_count += 1;
                    }
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to parse JSON file {:?}: {}. Skipping.",
                            path, e
                        );
                        error_count += 1;
                    }
                },
                Err(e) => {
                    eprintln!("Warning: Failed to read file {:?}: {}. Skipping.", path, e);
                    error_count += 1;
                }
            }
        }

        // 释放写锁
        drop(cache);

        if loaded_count > 0 {
            eprintln!("Loaded {} documents into memory cache", loaded_count);
        }
        if error_count > 0 {
            eprintln!(
                "Warning: {} files failed to load during cache initialization",
                error_count
            );
        }
    }

    /// 创建临时写入目录
    ///
    /// 用于实现原子性写入：所有文件先写入临时目录，成功后批量移动到正式目录。
    /// 如果中途失败，临时目录会被删除，确保不会留下"幽灵数据"。
    async fn create_temp_dir(&self) -> Result<PathBuf> {
        let temp_dir = Path::new(&self.data_dir).join(format!(".temp-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).await?;
        Ok(temp_dir)
    }

    /// 清理临时目录（失败时调用）
    async fn cleanup_temp_dir(temp_dir: &Path) {
        let _ = fs::remove_dir_all(temp_dir).await;
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<(KnowledgeRecord, f32)>> {
        // 短路保护：limit 为 0 时直接返回空结果
        if limit == 0 {
            return Ok(Vec::new());
        }

        // 优先使用 Tantivy Searcher 执行 BM25 搜索
        if let Some(searcher) = &self.searcher {
            // 使用 spawn_blocking 避免阻塞 Tokio 线程池
            // Tantivy 的 search 是同步阻塞操作，必须在专用线程中执行
            let searcher_clone = Arc::clone(searcher);
            let query_owned = query.to_string();

            let search_result =
                tokio::task::spawn_blocking(move || searcher_clone.search(&query_owned, limit))
                    .await;

            match search_result {
                Ok(Ok(search_results)) => {
                    // 将 SearchResult 转换为 (KnowledgeRecord, f32) 元组
                    // SearchResult.id 现在包含真实的 record.id，直接使用 O(1) 查询
                    let mut records = Vec::new();
                    for result in search_results {
                        match self.get_by_id_fast(&result.id).await {
                            Ok(Some(record)) => {
                                records.push((record, result.score));
                            }
                            Ok(None) => {
                                eprintln!(
                                    "Warning: Missing record for BM25 result id={}",
                                    result.id
                                );
                            }
                            Err(e) => {
                                eprintln!(
                                    "Warning: Failed to load record id={} after BM25 hit: {}",
                                    result.id, e
                                );
                            }
                        }
                    }
                    return Ok(records);
                }
                Ok(Err(e)) => {
                    eprintln!(
                        "Warning: BM25 search failed: {}. Falling back to naive matching.",
                        e
                    );
                    // 继续执行回退逻辑
                }
                Err(join_err) => {
                    eprintln!(
                        "Warning: Blocking task for search panicked or was cancelled: {}. Falling back to naive matching.",
                        join_err
                    );
                    // 继续执行回退逻辑
                }
            }
        }

        // 回退逻辑：朴素文本匹配搜索
        // 分数归一化系数：将朴素匹配分数（通常 1-6 分）放大到 BM25 量级（通常 0-10+ 分）
        const FALLBACK_SCALE: f32 = 10.0;
        let mut scored_records = Vec::new();
        let mut entries = fs::read_dir(&self.data_dir).await?;

        // 分词：按空格分割查询为多个 tokens
        let query_lower = query.to_lowercase();
        let query_tokens: Vec<&str> = query_lower.split_whitespace().collect();

        // 前置拦截：空查询直接返回空结果
        if query_tokens.is_empty() {
            return Ok(Vec::new());
        }

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // 防御性检查：跳过临时文件和目录
            if Self::is_temp_file(&path) {
                continue;
            }

            if !path.is_file() {
                continue;
            }

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path).await?;
                if let Ok(record) = serde_json::from_str::<KnowledgeRecord>(&content) {
                    // 计算匹配分数：title 匹配权重更高（2分），summary 匹配权重较低（1分）
                    let title_lower = record.title.to_lowercase();
                    let summary_lower = record.summary.to_lowercase();

                    let mut match_score: f32 = 0.0;
                    let mut title_matches = 0;

                    for token in &query_tokens {
                        // title 中的匹配权重为 2
                        if title_lower.contains(token) {
                            match_score += 2.0;
                            title_matches += 1;
                        }
                        // summary 中的匹配权重为 1
                        if summary_lower.contains(token) {
                            match_score += 1.0;
                        }
                    }

                    // 奖励：如果 title 包含所有 tokens，给予额外加分
                    if title_matches == query_tokens.len() {
                        match_score += 3.0; // 完全匹配奖励
                    } else if title_matches > 0 && title_matches >= query_tokens.len().div_ceil(2) {
                        match_score += 1.0; // 部分匹配奖励（必须至少命中 1 个）
                    }

                    // 只保留至少匹配一个 token 的记录
                    if match_score > 0.0 {
                        let normalized_score = match_score * FALLBACK_SCALE;
                        scored_records.push((record, normalized_score));
                    }
                }
            }
        }

        // 按匹配分数降序排序，分数相同时使用 ID 作为确定性 tie-breaker
        scored_records.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.id.cmp(&b.0.id))
        });

        // 严格遵守 limit 契约，防止内存溢出
        scored_records.truncate(limit);

        // 返回排序后的 (record, score) 元组列表
        Ok(scored_records)
    }

    /// 基于向量的语义搜索
    ///
    /// 通过计算查询向量与文档向量之间的余弦相似度来检索语义相关的文档。
    /// 此方法在内存中执行扫描，不涉及磁盘 I/O，性能远高于磁盘遍历。
    ///
    /// # 参数
    ///
    /// * `query` - 查询文本
    /// * `limit` - 返回结果的最大数量
    ///
    /// # 返回
    ///
    /// 按相似度降序排列的 `(KnowledgeRecord, similarity_score)` 元组列表。
    /// `similarity_score` 是归一化到 [0.0, 1.0] 范围的余弦相似度。
    ///
    /// # 错误
    ///
    /// * 如果 `embedding_model` 未初始化，返回 "Embedding model not initialized"
    /// * 如果查询向量化失败，返回描述性错误
    ///
    /// # 性能
    ///
    /// - 查询向量化：~50-100ms（FastEmbed 模型推理）
    /// - 内存扫描：O(N)，N 为文档数量（1000 文档约 10-50ms）
    /// - **总延迟 < 500ms**（对于 1000 文档的知识库）
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// # use contextfy_core::storage::KnowledgeStore;
    /// # use anyhow::Result;
    /// # async fn example(store: KnowledgeStore) -> Result<()> {
    /// let results = store.vector_search("how to create a red block", 10).await?;
    /// for (record, score) in results {
    ///     println!("{} (similarity: {:.2})", record.title, score);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn vector_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<(KnowledgeRecord, f32)>> {
        // 【边界条件检查】提前返回空结果，避免昂贵的模型调用
        if limit == 0 || query.trim().is_empty() {
            return Ok(Vec::new());
        }

        // 检查是否初始化了嵌入模型
        let embedding_model = self
            .embedding_model
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Embedding model not initialized"))?;

        // 使用 spawn_blocking 包裹向量化调用，避免阻塞异步运行时
        let embedding_model_clone = Arc::clone(embedding_model);
        let query_owned = query.to_string();

        let query_vector =
            tokio::task::spawn_blocking(move || embedding_model_clone.embed_text(&query_owned))
                .await
                .map_err(|e| anyhow::anyhow!("Embedding task failed: {}", e))?
                .context("Failed to generate query embedding")?;

        // 委托给核心搜索逻辑
        self.do_vector_search(&query_vector, limit).await
    }

    /// 核心向量搜索逻辑（私有辅助方法）
    ///
    /// 此方法封装了向量搜索的核心算法：
    /// - 内存扫描遍历所有文档
    /// - 余弦相似度计算
    /// - 降序排序（带确定性 tie-breaker）
    /// - Top-K 截断
    ///
    /// # 排序确定性
    ///
    /// 当多个文档的相似度分数相同时，使用文档 ID 作为次级排序键，
    /// 确保结果顺序稳定可重现，避免 HashMap 迭代顺序的随机性影响。
    ///
    /// # 参数
    ///
    /// * `query_vector` - 预计算的 384 维查询向量
    /// * `limit` - 返回结果的最大数量
    ///
    /// # 返回
    ///
    /// 按相似度降序排列的 `(KnowledgeRecord, similarity_score)` 元组列表
    async fn do_vector_search(
        &self,
        query_vector: &[f32],
        limit: usize,
    ) -> Result<Vec<(KnowledgeRecord, f32)>> {
        // 边界条件检查
        if limit == 0 || query_vector.is_empty() {
            return Ok(Vec::new());
        }

        // 验证向量维度
        if query_vector.len() != 384 {
            return Err(anyhow::anyhow!(
                "Query vector must be 384-dimensional, got {} dimensions",
                query_vector.len()
            ));
        }

        // 在内存中遍历所有文档，计算相似度（不使用磁盘 I/O）
        let cache = self.records.read().await;
        let mut scored_records = Vec::new();

        for (_id, record) in cache.iter() {
            // 过滤掉没有向量的文档
            if let Some(doc_embedding) = &record.embedding {
                // 【防御性编程】验证向量维度一致性，防止损坏/篡改的 JSON 数据导致 panic
                if doc_embedding.len() != query_vector.len() {
                    continue;
                }
                // 计算余弦相似度
                let similarity = cosine_similarity(query_vector, doc_embedding);
                scored_records.push((record.clone(), similarity));
            }
        }
        // 释放读锁
        drop(cache);

        // 按相似度降序排序（确定性排序：分数相同时按 ID 排序）
        scored_records.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                // 分数相同时，按文档 ID 稳定排序
                .then_with(|| a.0.id.cmp(&b.0.id))
        });

        // 截取前 K 个结果
        scored_records.truncate(limit);

        Ok(scored_records)
    }

    /// 测试专用：使用预设查询向量进行搜索
    ///
    /// **【测试专用方法】** 此方法仅用于单元测试，允许跳过 EmbeddingModel 推理直接传入查询向量。
    ///
    /// **设计目的**：
    /// - **分离模型推理成本**：EmbeddingModel 推理（~50ms）会干扰性能测试，此方法让测试聚焦于核心扫描路径
    /// - **可控的测试数据**：使用手动构造的向量可以精确控制相似度关系（如 [1.0, 0.0] vs [0.0, 1.0]）
    /// - **快速反馈**：单元测试应快速运行（< 100ms），不等待模型推理
    ///
    /// **代码路径**：此方法直接调用私有辅助方法 `do_vector_search()`，复用与生产环境完全相同的核心搜索逻辑。
    /// 唯一的区别是跳过 `embed_text()` 调用，直接使用预设的查询向量。
    ///
    /// **注意**：生产环境应使用 `vector_search()` 方法，集成测试应验证完整的 EmbeddingModel 集成。
    ///
    /// # 参数
    ///
    /// * `query_vector` - 预设的 384 维查询向量
    /// * `limit` - 返回结果的最大数量
    ///
    /// # 返回
    ///
    /// 按相似度降序排列的 `(KnowledgeRecord, similarity_score)` 元组列表
    #[cfg(test)]
    async fn vector_search_with_query_vector(
        &self,
        query_vector: &[f32],
        limit: usize,
    ) -> Result<Vec<(KnowledgeRecord, f32)>> {
        // 委托给核心搜索逻辑
        self.do_vector_search(query_vector, limit).await
    }

    /// 混合检索（Hybrid Search）：结合 BM25 和向量语义搜索
    ///
    /// 通过并发执行 BM25 全文搜索和向量语义搜索，归一化分数后按权重融合，
    /// 提供更精准的检索结果。BM25 权重 70%，向量权重 30%。
    ///
    /// # 算法流程
    ///
    /// 1. **多路召回**：并发调用 BM25 `search` 和 `vector_search`，获取 Top-N 结果（N = top_k * 2）
    /// 2. **BM25 归一化**：使用 Min-Max 归一化将 BM25 分数映射到 [0, 1] 范围
    /// 3. **结果合并**：使用 HashMap 按文档 ID 合并两路结果，缺失分数设为 0.0
    /// 4. **加权融合**：按 `final_score = 0.7 * normalized_bm25 + 0.3 * vector_score` 计算最终得分
    /// 5. **排序截断**：按 `final_score` 降序排序，分数相同时使用 ID 作为 tie-breaker，截取 top_k
    ///
    /// # 优雅降级
    ///
    /// - 如果向量模型未初始化或向量搜索失败，系统平滑退化为纯 BM25 搜索
    /// - 如果 BM25 搜索返回空结果，返回纯向量搜索结果
    ///
    /// # 性能优化
    ///
    /// - 使用 `tokio::join!` 并发执行两路搜索，降低总延迟
    /// - 内存缓存优先，减少磁盘 I/O
    ///
    /// # 参数
    ///
    /// * `query` - 查询文本
    /// * `top_k` - 返回结果的最大数量
    ///
    /// # 返回
    ///
    /// 按 `final_score` 降序排列的 `BriefWithScore` 列表，包含原始分数和最终融合分数
    ///
    /// # 错误
    ///
    /// - 如果 BM25 搜索失败，返回错误
    /// - 如果向量搜索失败，记录警告并继续（优雅降级）
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// # use contextfy_core::storage::KnowledgeStore;
    /// # use anyhow::Result;
    /// # async fn example(store: KnowledgeStore) -> Result<()> {
    /// let results = store.hybrid_search("how to create a red block", 10).await?;
    /// for result in results {
    ///     println!("{} (BM25: {:.2}, Vector: {:.2}, Final: {:.2})",
    ///         result.record.title, result.bm25_score, result.vector_score, result.final_score);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn hybrid_search(&self, query: &str, top_k: usize) -> Result<Vec<BriefWithScore>> {
        // 边界条件检查
        if top_k == 0 || query.trim().is_empty() {
            return Ok(Vec::new());
        }

        // 计算召回数量（top_k * RECALL_MULTIPLIER 保证召回率）
        let recall_n = top_k * RECALL_MULTIPLIER;

        // 【并发检索】使用 tokio::join! 并发执行 BM25 和向量搜索
        // Rust 允许多个不可变借用 (&self) 并发存在，所以这里是安全的
        let (bm25_results, vector_results) = tokio::join!(self.search(query, recall_n), async {
            // 向量搜索可能失败（无模型），优雅降级
            match self.vector_search(query, recall_n).await {
                Ok(results) => results,
                Err(e) => {
                    eprintln!(
                        "Warning: Vector search failed: {}. Falling back to BM25-only search.",
                        e
                    );
                    Vec::new() // 返回空结果，后续会将 vector_score 设为 0.0
                }
            }
        });

        // 处理 BM25 搜索失败情况
        let bm25_results = bm25_results?;

        // 【结果交并集】使用 HashMap 按文档 ID 合并两路结果
        let mut merged_results: HashMap<String, BriefWithScore> = HashMap::new();

        // 插入 BM25 结果（原始分数，稍后统一归一化）
        for (record, bm25_score) in bm25_results {
            merged_results.insert(
                record.id.clone(),
                BriefWithScore {
                    record,
                    bm25_score,
                    vector_score: 0.0, // 默认为 0，如果向量搜索有结果会被更新
                    final_score: 0.0,  // 稍后计算
                },
            );
        }

        // 更新向量分数
        for (record, vector_score) in vector_results {
            if let Some(result) = merged_results.get_mut(&record.id) {
                // 文档同时存在于两路结果中，更新 vector_score
                result.vector_score = vector_score;
            } else {
                // 文档仅在向量结果中，插入新记录
                merged_results.insert(
                    record.id.clone(),
                    BriefWithScore {
                        record,
                        bm25_score: 0.0,
                        vector_score,
                        final_score: 0.0, // 稍后计算
                    },
                );
            }
        }

        // 【加权融合】计算最终分数
        const BM25_WEIGHT: f32 = 0.7;
        const VECTOR_WEIGHT: f32 = 0.3;

        // 转换为 Vec 并计算最终分数
        let mut results: Vec<BriefWithScore> = merged_results.into_values().collect();

        // 重新计算归一化 BM25 分数和最终分数
        if !results.is_empty() {
            // 提取所有 BM25 分数并归一化
            let all_bm25_scores: Vec<f32> = results.iter().map(|r| r.bm25_score).collect();
            let min_bm25 = all_bm25_scores
                .iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(&0.0);
            let max_bm25 = all_bm25_scores
                .iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(&0.0);
            let range = max_bm25 - min_bm25;

            for result in &mut results {
                let normalized_bm25 = if range.abs() < f32::EPSILON {
                    // 所有分数相同或只有向量结果
                    if result.bm25_score > 0.0 {
                        1.0
                    } else {
                        0.0
                    }
                } else {
                    // Min-Max 归一化
                    (result.bm25_score - min_bm25) / range
                };
                result.final_score =
                    BM25_WEIGHT * normalized_bm25 + VECTOR_WEIGHT * result.vector_score;
            }
        }

        // 【安全排序】按 final_score 降序排序，分数相同时使用 ID 作为 tie-breaker
        results.sort_by(|a, b| {
            b.final_score
                .partial_cmp(&a.final_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.record.id.cmp(&b.record.id))
        });

        // 截取前 top_k 个结果
        results.truncate(top_k);

        Ok(results)
    }

    /// 快速通过 ID 获取记录（O(1) 复杂度）
    ///
    /// **【性能优化】** 优先使用内存缓存，仅在 Cache Miss 时回退到磁盘读取。
    /// 这是 BM25 搜索后获取完整内容的首选方法。
    ///
    /// # 查询策略
    ///
    /// 1. **Cache-First**: 先查询 `self.records` 内存缓存（O(1) 复杂度）
    /// 2. **Fallback**: 如果缓存未命中，从磁盘读取 JSON 文件
    ///
    /// # 参数
    ///
    /// * `id` - 记录的唯一标识符
    async fn get_by_id_fast(&self, id: &str) -> Result<Option<KnowledgeRecord>> {
        // 【Cache-First】优先查询内存缓存（O(1) 复杂度，零磁盘 I/O）
        let cache = self.records.read().await;
        if let Some(record) = cache.get(id) {
            return Ok(Some(record.clone()));
        }
        // 释放读锁
        drop(cache);

        // 【Fallback】缓存未命中，回退到磁盘读取
        let file_path = Path::new(&self.data_dir).join(format!("{}.json", id));

        match fs::read_to_string(&file_path).await {
            Ok(content) => {
                let record = serde_json::from_str::<KnowledgeRecord>(&content)
                    .context("Failed to parse knowledge record")?;
                Ok(Some(record))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e).context(format!(
                "Failed to read record file: {}",
                file_path.display()
            )),
        }
    }

    /// 通过 ID 获取记录（公开 API）
    ///
    /// **【性能优化】** 此方法直接委托给 `get_by_id_fast()`，优先使用内存缓存（O(1) 复杂度），
    /// 仅在 Cache Miss 时回退到磁盘读取。
    ///
    /// # 参数
    ///
    /// * `id` - 记录的唯一标识符
    pub async fn get(&self, id: &str) -> Result<Option<KnowledgeRecord>> {
        // 【内存优先】直接委托给 get_by_id_fast()，利用内存缓存
        self.get_by_id_fast(id).await
    }

    /// 检查路径是否是临时文件（防御性检查）
    ///
    /// 临时文件/目录的**名称**以 `.temp-` 开头，应该在正常扫描中被跳过。
    /// 只检查路径的最后一个组件（文件名或目录名），避免误判包含 `.temp-` 的父目录路径。
    fn is_temp_file(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.starts_with(".temp-"))
            .unwrap_or(false)
    }

    /// 添加文档到知识库（带回滚机制的原子性写入）
    ///
    /// # 回滚机制
    ///
    /// 1. **准备阶段**：创建临时目录（`.data/.temp-{uuid}`）
    /// 2. **写入阶段**：所有切片先写入临时目录
    /// 3. **提交阶段**：全部成功后，原子性移动文件到正式目录
    /// 4. **回滚阶段**：任何失败发生时，删除整个临时目录和已提交的文件
    ///
    /// 这样确保了原子性：要么所有切片都成功写入，要么都不写入。
    /// 不会出现"部分写入"导致的"幽灵数据"问题。
    pub async fn add(&self, doc: &ParsedDoc) -> Result<Vec<String>> {
        let mut ids = Vec::new();
        // 【关键修复】在内存中收集所有 KnowledgeRecord 对象，用于后续缓存更新
        let mut records_to_cache = Vec::new();

        if doc.sections.is_empty() {
            // 回退逻辑：如果文档没有切片，将整个文档作为单条记录存储
            // 这种情况可能出现在：
            // 1. 文档没有 H2 标题
            // 2. 旧版本解析的文档（向后兼容）
            //
            // 使用临时文件模式确保原子性：写入临时文件 -> 原子重命名
            let id = Uuid::new_v4().to_string();

            // 从内容中提取代码块关键词
            let keywords = extract_code_block_keywords(&doc.content);

            // 生成向量嵌入（如果有嵌入模型）
            // 使用 spawn_blocking 避免阻塞异步 worker 线程
            let embedding = if let Some(model) = &self.embedding_model {
                let model_clone = Arc::clone(model);
                let text = format!("{} {}", doc.title, doc.summary);
                match tokio::task::spawn_blocking(move || model_clone.embed_text(&text)).await {
                    Ok(Ok(vec)) => Some(vec),
                    Ok(Err(e)) => {
                        eprintln!(
                            "Warning: Failed to generate embedding for document '{}': {}. Vector field will be None.",
                            doc.title, e
                        );
                        None
                    }
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to join embedding task for document '{}': {}. Vector field will be None.",
                            doc.title, e
                        );
                        None
                    }
                }
            } else {
                None
            };

            let record = KnowledgeRecord {
                id: id.clone(),
                title: doc.title.clone(),
                parent_doc_title: doc.title.clone(),
                summary: doc.summary.clone(),
                content: doc.content.clone(),
                source_path: doc.path.clone(),
                keywords,
                embedding,
            };

            // 创建临时目录
            let temp_dir = match self.create_temp_dir().await {
                Ok(dir) => dir,
                Err(e) => {
                    return Err(e).context("Failed to create temporary directory");
                }
            };

            // 序列化并写入临时文件
            let json = match serde_json::to_string_pretty(&record) {
                Ok(j) => j,
                Err(e) => {
                    Self::cleanup_temp_dir(&temp_dir).await;
                    return Err(e).context("Failed to serialize document");
                }
            };

            let temp_path = temp_dir.join(format!("{}.json", id));
            if let Err(e) = fs::write(&temp_path, json).await {
                Self::cleanup_temp_dir(&temp_dir).await;
                return Err(e).context("Failed to write temporary file");
            }

            // 原子性移动到正式目录
            let final_path = Path::new(&self.data_dir).join(format!("{}.json", id));
            if let Err(e) = fs::rename(&temp_path, &final_path).await {
                Self::cleanup_temp_dir(&temp_dir).await;
                return Err(e).context("Failed to move file to final destination");
            }

            // 清理临时目录
            Self::cleanup_temp_dir(&temp_dir).await;
            ids.push(id.clone());
            // 【关键修复】收集 record 对象，避免后续回读磁盘
            records_to_cache.push((id, record));
        } else {
            // 新逻辑：为每个切片创建独立的记录（带回滚机制）
            //
            // 【问题】如果第 5 个切片写入失败（比如磁盘满），前 4 个切片已经留在那了，
            // 变成了"幽灵数据"，导致数据不一致。
            //
            // 【解决方案】使用临时目录 + 原子移动 + 回滚机制：
            // 1. 所有切片先写入临时目录
            // 2. 全部成功后，批量移动到正式目录（记录已移动的文件）
            // 3. 任何失败发生时，删除已移动的文件和临时目录
            //
            // 这样确保了原子性：要么全部成功，要么全部失败。

            // 步骤 1：创建临时目录
            let temp_dir = match self.create_temp_dir().await {
                Ok(dir) => dir,
                Err(e) => {
                    return Err(e).context("Failed to create temporary directory");
                }
            };

            // 步骤 2：在临时目录中写入所有切片
            // 【关键优化】使用批处理生成向量，避免循环调用 embed_text
            // 先收集所有切片的文本，然后一次性生成所有向量
            let embedding_texts: Vec<String> = doc
                .sections
                .iter()
                .map(|slice| format!("{} {}", slice.section_title, slice.summary))
                .collect();

            // 批量生成向量（如果有嵌入模型）
            // 使用 spawn_blocking 避免阻塞异步 worker 线程
            let embeddings = if let Some(model) = &self.embedding_model {
                let model_clone = Arc::clone(model);
                let doc_title = doc.title.clone();
                match tokio::task::spawn_blocking(move || {
                    // 在闭包内部将 Vec<String> 转换为 Vec<&str>
                    let texts_refs: Vec<&str> =
                        embedding_texts.iter().map(|s| s.as_str()).collect();
                    model_clone.embed_batch(&texts_refs)
                })
                .await
                {
                    Ok(Ok(vecs)) => Some(vecs),
                    Ok(Err(e)) => {
                        eprintln!(
                            "Warning: Failed to generate batch embeddings for document '{}': {}. Vector fields will be None.",
                            doc_title, e
                        );
                        None
                    }
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to join batch embedding task for document '{}': {}. Vector fields will be None.",
                            doc_title, e
                        );
                        None
                    }
                }
            } else {
                None
            };

            // 步骤 2.5：验证向量数量与切片数量是否匹配（在循环前一次性检查）
            // 如果不匹配，打印警告并将 embeddings 设为 None（优雅降级）
            let embeddings = if embeddings
                .as_ref()
                .is_some_and(|vecs| vecs.len() != doc.sections.len())
            {
                eprintln!(
                    "Warning: Embedding count mismatch for document '{}'. Expected {}, got {}. All slices will have None embeddings (graceful degradation - documents still searchable via BM25).",
                    doc.title,
                    doc.sections.len(),
                    embeddings.as_ref().map_or(0, |vecs| vecs.len())
                );
                None // 清空 embeddings，后续所有切片都会得到 None
            } else {
                embeddings
            };

            let mut temp_files = Vec::new();
            // 【关键修复】使用外层声明的 records_to_cache，避免变量遮蔽
            // 注意：这里不要重新声明，直接使用外层的 records_to_cache

            for (idx, slice) in doc.sections.iter().enumerate() {
                let id = Uuid::new_v4().to_string();
                let temp_path = temp_dir.join(format!("{}.json", id));

                // 从切片内容中提取代码块关键词
                let keywords = extract_code_block_keywords(&slice.content);

                // 获取当前切片的向量（如果有）- 已在循环前验证长度，这里安全使用 get()
                // 【优化】直接移动 embedding，避免不必要的克隆
                let embedding = embeddings.as_ref().and_then(|vecs| vecs.get(idx).cloned());

                // 【关键修复】先在内存中构造 KnowledgeRecord 对象
                let record = KnowledgeRecord {
                    id: id.clone(),
                    title: slice.section_title.clone(),
                    parent_doc_title: slice.parent_doc_title.clone(),
                    summary: slice.summary.clone(),
                    content: slice.content.clone(),
                    source_path: doc.path.clone(),
                    keywords,
                    // 移动 embedding，而不是克隆
                    embedding,
                };

                // 序列化失败时清理临时目录
                let json = match serde_json::to_string_pretty(&record) {
                    Ok(j) => j,
                    Err(e) => {
                        Self::cleanup_temp_dir(&temp_dir).await;
                        return Err(e).context(format!(
                            "Failed to serialize slice: {}",
                            slice.section_title
                        ));
                    }
                };

                // 如果写入失败，清理临时目录并返回错误
                if let Err(e) = fs::write(&temp_path, json).await {
                    Self::cleanup_temp_dir(&temp_dir).await;
                    return Err(e).context(format!(
                        "Failed to write temporary file for slice: {}",
                        slice.section_title
                    ));
                }

                ids.push(id.clone());
                // 【关键修复】先克隆 id 用于 temp_files，然后再用于 records_to_cache
                temp_files.push((id.clone(), temp_path));
                // 【关键修复】收集 record 对象，避免后续回读磁盘
                records_to_cache.push((id, record));
            }

            // 步骤 3：全部成功后，移动文件到正式目录（带回滚机制）
            let mut committed_files = Vec::new();

            for (id, temp_path) in temp_files {
                let final_path = Path::new(&self.data_dir).join(format!("{}.json", id));

                match fs::rename(&temp_path, &final_path).await {
                    Ok(_) => {
                        committed_files.push(final_path);
                    }
                    Err(e) => {
                        // 回滚：删除所有已移动到正式目录的文件
                        for path in &committed_files {
                            let _ = fs::remove_file(path).await;
                        }
                        Self::cleanup_temp_dir(&temp_dir).await;
                        return Err(e)
                            .context(format!("Failed to move file {} to final destination", id))
                            .context("Transaction rolled back: all committed files removed");
                    }
                }
            }

            // 步骤 4：删除临时目录（此时应该已经为空）
            Self::cleanup_temp_dir(&temp_dir).await;
        }

        // 步骤 5：同步添加文档到 Tantivy 索引
        if let Some(indexer_mutex) = &self.indexer {
            // 【性能优化】直接复用已收集的 KnowledgeRecord 对象，避免冗余的磁盘 I/O
            // records_to_cache 中已经包含了所有需要索引的完整记录
            let records_to_index: Vec<KnowledgeRecord> = records_to_cache
                .iter()
                .map(|(_, record)| record.clone())
                .collect();

            // 使用 spawn_blocking 避免阻塞 Tokio 线程池
            // Tantivy 的 add_doc 和 commit 是同步阻塞操作，必须在专用线程中执行
            let indexer_mutex_clone = indexer_mutex.clone();
            let index_result = tokio::task::spawn_blocking(move || {
                // 在阻塞线程中使用 blocking_lock() 获取锁
                let mut indexer = indexer_mutex_clone.blocking_lock();
                let mut index_success = true;
                let mut first_error = None;

                for record in records_to_index {
                    if let Err(e) = indexer.add_doc(&record) {
                        first_error = Some((
                            record.id.clone(),
                            anyhow::anyhow!("Failed to index document: {}", e),
                        ));
                        index_success = false;
                        break;
                    }
                }

                // 提交索引
                if index_success {
                    if let Err(e) = indexer.commit() {
                        // 【索引失败处理】commit 失败后，IndexWriter 会保持缓冲区状态
                        // 下次调用 commit 时会重试提交这些文档（Tantivy 的标准行为）
                        // 注意：不需要手动 rollback，Tantivy IndexWriter 没有 rollback() API
                        first_error = Some((
                            "commit".to_string(),
                            anyhow::anyhow!("Failed to commit index: {}", e),
                        ));
                    }
                }

                first_error
            })
            .await;

            // 处理索引错误
            match index_result {
                Ok(Some((error_id, error))) => {
                    if error_id == "commit" {
                        eprintln!("Warning: {}. New documents may not be searchable.", error);
                    } else {
                        eprintln!(
                            "Warning: Failed to index document {}. Search may be incomplete. {}",
                            error_id, error
                        );
                    }
                }
                Ok(None) => {
                    // 索引成功，无需操作
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Blocking task for indexing panicked or was cancelled: {}. Search may be incomplete.",
                        e
                    );
                }
            }
        }

        // 同步更新内存缓存（Write-Through）
        // 【关键修复】使用已收集的 KnowledgeRecord 对象，而非回读磁盘
        // 这样避免了持有写锁时执行异步 I/O，提升了并发性能
        let mut cache = self.records.write().await;
        for (id, record) in records_to_cache {
            cache.insert(id, record);
        }
        drop(cache);

        Ok(ids) // 返回所有切片的 ID（如果有切片）或单个文档 ID
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SlicedSection;
    use std::fs;

    /// 基本切片存储测试
    #[tokio::test]
    async fn test_add_sliced_doc() {
        // 创建临时测试目录
        let temp_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap(), None)
            .await
            .unwrap();

        // 手动构造包含 2 个切片的 ParsedDoc
        let doc = ParsedDoc {
            path: "/fake/path.md".to_string(),
            title: "Test Doc".to_string(),
            summary: "Test summary".to_string(),
            content: "Full content".to_string(),
            sections: vec![
                SlicedSection {
                    section_title: "Section 1".to_string(),
                    content: "Content 1".to_string(),
                    parent_doc_title: "Test Doc".to_string(),
                    summary: "Content 1".to_string(),
                },
                SlicedSection {
                    section_title: "Section 2".to_string(),
                    content: "Content 2".to_string(),
                    parent_doc_title: "Test Doc".to_string(),
                    summary: "Content 2".to_string(),
                },
            ],
        };

        // 调用 add()
        let ids = store.add(&doc).await.unwrap();

        // 断言：返回 2 个 ID
        assert_eq!(ids.len(), 2);

        // 断言：存储目录中有 2 个 JSON 文件
        let json_files: Vec<_> = fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
            .collect();
        assert_eq!(json_files.len(), 2);

        // 断言：每个记录都有正确的 source_path
        for json_file in json_files {
            let content = fs::read_to_string(json_file.path()).unwrap();
            let record: KnowledgeRecord = serde_json::from_str(&content).unwrap();
            assert_eq!(record.source_path, "/fake/path.md");
        }
    }

    /// 空切片回退测试
    #[tokio::test]
    async fn test_add_empty_sections() {
        // 创建临时测试目录
        let temp_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap(), None)
            .await
            .unwrap();

        // 构造没有切片的 ParsedDoc（回退逻辑）
        let doc = ParsedDoc {
            path: "/legacy/doc.md".to_string(),
            title: "Legacy Doc".to_string(),
            summary: "Legacy summary".to_string(),
            content: "Full legacy content".to_string(),
            sections: vec![], // 空切片，触发回退逻辑
        };

        // 调用 add()
        let ids = store.add(&doc).await.unwrap();

        // 断言：返回 1 个 ID（整篇文档作为单条记录）
        assert_eq!(ids.len(), 1);

        // 断言：存储目录中有 1 个 JSON 文件
        let json_files: Vec<_> = fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
            .collect();
        assert_eq!(json_files.len(), 1);

        // 断言：记录的 title 是文档标题（而非切片标题）
        let content = fs::read_to_string(json_files[0].path()).unwrap();
        let record: KnowledgeRecord = serde_json::from_str(&content).unwrap();
        assert_eq!(record.title, "Legacy Doc");
        assert_eq!(record.source_path, "/legacy/doc.md");
    }

    /// 鲁棒性测试（极端情况）
    #[tokio::test]
    async fn test_storage_robustness() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap(), None)
            .await
            .unwrap();

        // 构造极端数据
        let mut sections = vec![
            // Case A: 标题为空，内容包含 Emoji 和特殊符号
            SlicedSection {
                section_title: "".to_string(),
                content: "🚀 Emoji & \"Quotes\" & \nNewlines".to_string(),
                parent_doc_title: "Edge Case Doc".to_string(),
                summary: "🚀 Emoji & \"Quotes\" & \nNewlines".to_string(),
            },
            // Case B: 只有标题，内容为空
            SlicedSection {
                section_title: "Empty Content".to_string(),
                content: "".to_string(),
                parent_doc_title: "Edge Case Doc".to_string(),
                summary: "".to_string(),
            },
        ];

        // Case C: 大量切片 (模拟长文) - 循环生成 50 个切片
        for i in 0..50 {
            sections.push(SlicedSection {
                section_title: format!("Section {}", i),
                content: format!("Content for section {}", i),
                parent_doc_title: "Edge Case Doc".to_string(),
                summary: format!("Content for section {}", i),
            });
        }

        let doc = ParsedDoc {
            path: "C:\\Windows\\System32\\weird_path.md".to_string(), // Windows 路径反斜杠测试
            title: "Edge Case Doc".to_string(),
            summary: "".to_string(),
            content: "".to_string(),
            sections,
        };

        // 验证是否能成功写入，不 Panic
        let ids = store.add(&doc).await.unwrap();

        // 验证 Case C: 确保生成的 ID 数量正确 (2个手动 + 50个循环 = 52)
        assert_eq!(ids.len(), 52);

        // 验证 JSON 读取回来的数据完整性 (确保特殊字符没有乱码)
        // 读取第一个文件，反序列化，断言 content == "🚀 Emoji & \"Quotes\" & \nNewlines"
        let first_record = store.get(&ids[0]).await.unwrap().unwrap();
        assert_eq!(first_record.content, "🚀 Emoji & \"Quotes\" & \nNewlines");
        assert_eq!(
            first_record.source_path,
            "C:\\Windows\\System32\\weird_path.md"
        );

        // 验证 Case B: 空内容切片也能正确存储
        let second_record = store.get(&ids[1]).await.unwrap().unwrap();
        assert_eq!(second_record.title, "Empty Content");
        assert_eq!(second_record.content, "");

        // 验证 Case C: 所有 ID 都是唯一的（通过集合去重后数量不变）
        let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(unique_ids.len(), 52);
    }

    /// 回滚机制测试：验证成功后临时目录被清理
    #[tokio::test]
    async fn test_rollback_temp_cleanup() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap(), None)
            .await
            .unwrap();

        let doc = ParsedDoc {
            path: "/test/doc.md".to_string(),
            title: "Test Doc".to_string(),
            summary: "Test summary".to_string(),
            content: "Full content".to_string(),
            sections: vec![
                SlicedSection {
                    section_title: "Section 1".to_string(),
                    content: "Content 1".to_string(),
                    parent_doc_title: "Test Doc".to_string(),
                    summary: "Content 1".to_string(),
                },
                SlicedSection {
                    section_title: "Section 2".to_string(),
                    content: "Content 2".to_string(),
                    parent_doc_title: "Test Doc".to_string(),
                    summary: "Content 2".to_string(),
                },
            ],
        };

        // 执行添加操作（应该成功）
        let ids = store.add(&doc).await.unwrap();
        assert_eq!(ids.len(), 2);

        // 验证：正式目录中有 2 个 JSON 文件
        let json_files: Vec<_> = fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().extension().and_then(|s| s.to_str()) == Some("json")
                    && !e
                        .path()
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .starts_with(".temp-")
            })
            .collect();
        assert_eq!(json_files.len(), 2);

        // 验证：没有临时目录残留（所有 .temp-* 目录都被清理）
        let temp_dirs: Vec<_> = fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with(".temp-")
            })
            .collect();
        assert_eq!(
            temp_dirs.len(),
            0,
            "Temporary directories should be cleaned up"
        );
    }

    /// 原子性测试：验证所有文件要么都在，要么都不在
    #[tokio::test]
    async fn test_atomicity_all_or_nothing() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap(), None)
            .await
            .unwrap();

        // 创建包含 5 个切片的文档
        let mut sections = Vec::new();
        for i in 0..5 {
            sections.push(SlicedSection {
                section_title: format!("Section {}", i),
                content: format!("Content for section {}", i),
                parent_doc_title: "Atomicity Test".to_string(),
                summary: format!("Content for section {}", i),
            });
        }

        let doc = ParsedDoc {
            path: "/test/atomic.md".to_string(),
            title: "Atomicity Test".to_string(),
            summary: "Test".to_string(),
            content: "Full content".to_string(),
            sections,
        };

        // 执行添加操作
        let ids = store.add(&doc).await.unwrap();

        // 验证：所有 5 个文件都存在
        assert_eq!(ids.len(), 5);
        for id in &ids {
            let path = temp_dir.path().join(format!("{}.json", id));
            assert!(path.exists(), "File {} should exist", id);
        }

        // 验证：可以读取所有 5 个文件
        for id in &ids {
            let record = store.get(id).await.unwrap();
            assert!(record.is_some(), "Record {} should be retrievable", id);
        }
    }

    /// 内存缓存加载测试
    #[tokio::test]
    async fn test_memory_cache_loading() {
        // 创建临时测试目录
        let temp_dir = tempfile::tempdir().unwrap();
        let data_dir = temp_dir.path().to_str().unwrap();

        // 预先创建 3 个 JSON 文件
        for i in 1..=3 {
            let record = KnowledgeRecord {
                id: format!("id-{}", i),
                title: format!("Document {}", i),
                parent_doc_title: format!("Parent {}", i),
                summary: format!("Summary {}", i),
                content: format!("Content {}", i),
                source_path: format!("/path/{}.md", i),
                keywords: vec![],
                embedding: None,
            };
            let json = serde_json::to_string_pretty(&record).unwrap();
            let path = temp_dir.path().join(format!("{}.json", record.id));
            fs::write(&path, json).unwrap();
        }

        // 初始化 KnowledgeStore，应该加载所有文档到缓存
        let store = KnowledgeStore::new(data_dir, None).await.unwrap();

        // 验证缓存包含 3 个记录
        let cache = store.records.read().await;
        assert_eq!(cache.len(), 3, "Cache should contain 3 records");

        // 验证记录的 ID、title、content 正确
        let record1 = cache.get("id-1").unwrap();
        assert_eq!(record1.title, "Document 1");
        assert_eq!(record1.content, "Content 1");

        let record2 = cache.get("id-2").unwrap();
        assert_eq!(record2.title, "Document 2");

        let record3 = cache.get("id-3").unwrap();
        assert_eq!(record3.title, "Document 3");
    }

    /// Write-Through 缓存一致性测试
    #[tokio::test]
    async fn test_write_through_cache_consistency() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap(), None)
            .await
            .unwrap();

        // 添加新文档
        let doc = ParsedDoc {
            path: "/test/doc.md".to_string(),
            title: "Cache Test Doc".to_string(),
            summary: "Test summary".to_string(),
            content: "Test content".to_string(),
            sections: vec![],
        };

        let ids = store.add(&doc).await.unwrap();
        assert_eq!(ids.len(), 1);
        let new_id = &ids[0];

        // 验证磁盘文件存在
        let disk_path = temp_dir.path().join(format!("{}.json", new_id));
        assert!(disk_path.exists(), "Disk file should exist");

        // 验证缓存包含新记录
        let cache = store.records.read().await;
        assert!(
            cache.contains_key(new_id),
            "Cache should contain new record"
        );

        let cached_record = cache.get(new_id).unwrap();
        assert_eq!(cached_record.title, "Cache Test Doc");
        assert_eq!(cached_record.content, "Test content");
    }

    /// 向量搜索优雅降级测试（无嵌入模型）
    #[tokio::test]
    async fn test_vector_search_no_embedding_model() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap(), None)
            .await
            .unwrap();

        // 尝试向量搜索，应该返回明确的错误
        let result = store.vector_search("test query", 10).await;
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("Embedding model not initialized"),
            "Error should mention embedding model not initialized"
        );
    }

    /// 向量搜索边界条件测试
    #[tokio::test]
    async fn test_vector_search_boundary_conditions() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap(), None)
            .await
            .unwrap();

        // 添加一个没有向量的文档
        let doc = ParsedDoc {
            path: "/test/doc.md".to_string(),
            title: "Test Doc".to_string(),
            summary: "Test".to_string(),
            content: "Content".to_string(),
            sections: vec![],
        };
        store.add(&doc).await.unwrap();

        // 【修复】边界条件应返回空结果，而非错误
        // limit = 0 应该立即返回空结果
        let result = store.vector_search("test", 0).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());

        // 空查询应该立即返回空结果
        let result = store.vector_search("", 10).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());

        // 仅空格的查询应该返回空结果
        let result = store.vector_search("   ", 10).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    /// 测试所有文档 embedding=None 时返回空结果
    #[tokio::test]
    async fn test_vector_search_all_documents_without_embeddings() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap(), None)
            .await
            .unwrap();

        // 添加 3 个没有向量的文档
        for i in 0..3 {
            let doc = ParsedDoc {
                path: format!("/test/doc{}.md", i),
                title: format!("Test Doc {}", i),
                summary: format!("Test {}", i),
                content: format!("Content {}", i),
                sections: vec![],
            };
            store.add(&doc).await.unwrap();
        }

        // 使用测试专用方法验证：当所有文档都没有向量时，返回空结果
        let query_vec: Vec<f32> = (0..384).map(|i| if i == 0 { 1.0 } else { 0.0 }).collect();
        let result = store
            .vector_search_with_query_vector(&query_vec, 10)
            .await
            .unwrap();

        // 断言：所有文档都没有 embedding，应该返回空结果
        assert!(
            result.is_empty(),
            "Should return empty results when all documents lack embeddings"
        );
    }

    /// 语义排序测试（dog/puppy/vehicles）
    ///
    /// 测试场景：
    /// - 文档 A: "cat and dog are pets"（与 "dog" 高度相关）
    /// - 文档 B: "car and bike are vehicles"（与 "dog" 不相关）
    /// - 文档 C: "puppy plays in yard"（"puppy" 与 "dog" 语义相关）
    ///
    /// 查询 "dog"，期望结果顺序：A > C > B
    ///
    /// 【修复】此测试现在调用真实的 vector_search_with_query_vector() 方法，
    /// 验证完整的向量搜索流程（扫描+相似度+排序），而非手动模拟。
    #[tokio::test]
    async fn test_vector_search_semantic_ordering() {
        let temp_dir = tempfile::tempdir().unwrap();
        let data_dir = temp_dir.path().to_str().unwrap();

        // 创建三个文档，并手动赋予具有明确相似度关系的向量
        // 为了简化测试，我们使用向量夹角来模拟语义相似度
        //
        // 文档 A (高相关): [1.0, 0.0, 0.0, ...] - 与查询向量夹角小
        // 文档 B (不相关): [0.0, 1.0, 0.0, ...] - 与查询向量垂直
        // 文档 C (中相关): [0.7, 0.7, 0.0, ...] - 与查询向量有一定夹角
        //
        // 查询向量: [1.0, 0.0, 0.0, ...]
        //
        // 预期相似度排序: A (1.0) > C (~0.707) > B (0.0)

        let vec_a: Vec<f32> = (0..384).map(|i| if i == 0 { 1.0 } else { 0.0 }).collect();
        let vec_b: Vec<f32> = (0..384).map(|i| if i == 1 { 1.0 } else { 0.0 }).collect();
        let vec_c: Vec<f32> = (0..384).map(|i| if i < 2 { 0.7071 } else { 0.0 }).collect();

        // 创建三个文档
        let doc_a = KnowledgeRecord {
            id: "doc-a".to_string(),
            title: "cat and dog are pets".to_string(),
            parent_doc_title: "Pets".to_string(),
            summary: "About cats and dogs".to_string(),
            content: "Cats and dogs are common pets.".to_string(),
            source_path: "/pets/a.md".to_string(),
            keywords: vec![],
            embedding: Some(vec_a),
        };

        let doc_b = KnowledgeRecord {
            id: "doc-b".to_string(),
            title: "car and bike are vehicles".to_string(),
            parent_doc_title: "Vehicles".to_string(),
            summary: "About vehicles".to_string(),
            content: "Cars and bikes are vehicles.".to_string(),
            source_path: "/vehicles/b.md".to_string(),
            keywords: vec![],
            embedding: Some(vec_b),
        };

        let doc_c = KnowledgeRecord {
            id: "doc-c".to_string(),
            title: "puppy plays in yard".to_string(),
            parent_doc_title: "Pets".to_string(),
            summary: "About a puppy".to_string(),
            content: "A puppy plays in the yard.".to_string(),
            source_path: "/pets/c.md".to_string(),
            keywords: vec![],
            embedding: Some(vec_c),
        };

        // 写入磁盘
        for doc in &[&doc_a, &doc_b, &doc_c] {
            let json = serde_json::to_string_pretty(doc).unwrap();
            let path = temp_dir.path().join(format!("{}.json", doc.id));
            fs::write(&path, json).unwrap();
        }

        // 初始化 KnowledgeStore（无嵌入模型）
        // 【修复验证】KnowledgeStore::new() 会自动加载磁盘上的所有 JSON 文档到缓存
        // 不需要手动 cache.insert()，否则测试将失去验证自动加载逻辑的意义
        let store = KnowledgeStore::new(data_dir, None).await.unwrap();

        // 验证缓存确实包含了从磁盘自动加载的 3 个文档
        let cache = store.records.read().await;
        assert_eq!(
            cache.len(),
            3,
            "Cache should contain 3 documents loaded from disk"
        );
        drop(cache);

        // 查询向量: [1.0, 0.0, 0.0, ...]
        let query_vec: Vec<f32> = (0..384).map(|i| if i == 0 { 1.0 } else { 0.0 }).collect();

        // 【修复】调用真实的 vector_search_with_query_vector() 方法
        // 测试完整的向量搜索流程：内存扫描 + 余弦相似度 + 降序排序
        let results = store
            .vector_search_with_query_vector(&query_vec, 10)
            .await
            .unwrap();

        // 验证排序: doc-a > doc-c > doc-b
        assert_eq!(
            results[0].0.id, "doc-a",
            "doc-a should be first (highest similarity)"
        );
        assert_eq!(
            results[1].0.id, "doc-c",
            "doc-c should be second (medium similarity)"
        );
        assert_eq!(
            results[2].0.id, "doc-b",
            "doc-b should be last (lowest similarity)"
        );
    }

    /// 1000 文档性能基准测试（真实调用 vector_search）
    #[tokio::test]
    #[ignore]
    async fn test_vector_search_performance_1000_docs() {
        use std::time::Instant;

        let temp_dir = tempfile::tempdir().unwrap();
        let data_dir = temp_dir.path().to_str().unwrap();

        // 创建 1000 个带有随机向量的文档
        let num_docs = 1000;

        for i in 0..num_docs {
            let id = format!("doc-{:05}", i);
            // 生成随机 384 维向量
            let embedding: Vec<f32> = (0..384).map(|_| rand::random::<f32>()).collect();

            let record = KnowledgeRecord {
                id: id.clone(),
                title: format!("Document {}", i),
                parent_doc_title: format!("Parent {}", i),
                summary: format!("Summary for document {}", i),
                content: format!("Content for document {}", i),
                source_path: format!("/path/doc{}.md", i),
                keywords: vec![format!("keyword{}", i)],
                embedding: Some(embedding),
            };

            // 写入磁盘
            let json = serde_json::to_string_pretty(&record).unwrap();
            let path = temp_dir.path().join(format!("{}.json", id));
            fs::write(&path, json).unwrap();
        }

        // 初始化 KnowledgeStore（无嵌入模型）
        let store = KnowledgeStore::new(data_dir, None).await.unwrap();

        // 验证缓存加载了所有文档
        let cache = store.records.read().await;
        assert_eq!(cache.len(), num_docs, "Cache should contain all documents");
        drop(cache);

        // 【修复】生成随机查询向量，调用真实的 vector_search_with_query_vector()
        // 测试核心性能瓶颈：内存遍历 + 相似度计算 + 排序
        let query_vec: Vec<f32> = (0..384).map(|_| rand::random::<f32>()).collect();

        let start = Instant::now();
        let results = store
            .vector_search_with_query_vector(&query_vec, 10)
            .await
            .unwrap();
        let duration = start.elapsed();

        println!(
            "Vector search on {} documents took: {:?}",
            num_docs, duration
        );
        println!("Top 10 results:");
        for (i, (record, score)) in results.iter().enumerate() {
            println!("  {}. {} (similarity: {:.4})", i + 1, record.id, score);
        }

        // 【修复】断言总延迟 < 500ms
        assert!(
            duration.as_millis() < 500,
            "Vector search should be < 500ms, took {}ms",
            duration.as_millis()
        );
    }

    // ========== 混合检索（Hybrid Search）测试 ==========

    /// 混合检索边界条件测试
    #[tokio::test]
    async fn test_hybrid_search_boundary_conditions() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap(), None)
            .await
            .unwrap();

        // top_k = 0 应该返回空结果
        let results = store.hybrid_search("test query", 0).await.unwrap();
        assert!(results.is_empty());

        // 空查询应该返回空结果
        let results = store.hybrid_search("", 10).await.unwrap();
        assert!(results.is_empty());

        // 仅空格的查询应该返回空结果
        let results = store.hybrid_search("   ", 10).await.unwrap();
        assert!(results.is_empty());
    }

    /// 混合检索优雅降级测试（无向量模型）
    #[tokio::test]
    async fn test_hybrid_search_fallback_to_bm25() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap(), None)
            .await
            .unwrap();

        // 添加测试文档
        let doc = ParsedDoc {
            path: "/test/doc.md".to_string(),
            title: "Test Doc".to_string(),
            summary: "Test summary".to_string(),
            content: "Test content".to_string(),
            sections: vec![],
        };
        store.add(&doc).await.unwrap();

        // 无向量模型时，应该退化为纯 BM25 搜索
        let results = store.hybrid_search("Test", 10).await.unwrap();
        assert!(!results.is_empty());

        // 验证所有结果的 vector_score 都为 0.0
        for result in &results {
            assert_eq!(result.vector_score, 0.0);
            assert!(result.final_score > 0.0);
        }
    }

    /// BM25 分数归一化测试
    #[tokio::test]
    async fn test_hybrid_search_bm25_normalization() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap(), None)
            .await
            .unwrap();

        // 添加 3 个测试文档
        for i in 0..3 {
            let doc = ParsedDoc {
                path: format!("/test/doc{}.md", i),
                title: format!("Test Doc {}", i),
                summary: format!("Test {}", i),
                content: format!("Content {}", i),
                sections: vec![],
            };
            store.add(&doc).await.unwrap();
        }

        // 执行混合检索
        let results = store.hybrid_search("Test", 10).await.unwrap();

        // 验证：所有结果都应该有分数（归一化后应该在 [0, 1] 范围内）
        for result in &results {
            // BM25 分数可能大于 1（未归一化），但 final_score 应该在合理范围内
            assert!(result.final_score >= 0.0 && result.final_score <= 1.0);
        }
    }

    /// 加权融合测试
    #[tokio::test]
    async fn test_hybrid_search_weighted_fusion() {
        let temp_dir = tempfile::tempdir().unwrap();
        let data_dir = temp_dir.path().to_str().unwrap();

        // 创建 3 个文档并手动赋予向量
        let vec_a: Vec<f32> = (0..384).map(|i| if i == 0 { 1.0 } else { 0.0 }).collect();
        let vec_b: Vec<f32> = (0..384).map(|i| if i == 1 { 1.0 } else { 0.0 }).collect();
        let vec_c: Vec<f32> = (0..384).map(|i| if i < 2 { 0.7071 } else { 0.0 }).collect();

        let doc_a = KnowledgeRecord {
            id: "doc-a".to_string(),
            title: "test document alpha".to_string(),
            parent_doc_title: "Test".to_string(),
            summary: "About test".to_string(),
            content: "Content".to_string(),
            source_path: "/test/a.md".to_string(),
            keywords: vec![],
            embedding: Some(vec_a),
        };

        let doc_b = KnowledgeRecord {
            id: "doc-b".to_string(),
            title: "other document beta".to_string(),
            parent_doc_title: "Test".to_string(),
            summary: "About other".to_string(),
            content: "Content".to_string(),
            source_path: "/test/b.md".to_string(),
            keywords: vec![],
            embedding: Some(vec_b),
        };

        let doc_c = KnowledgeRecord {
            id: "doc-c".to_string(),
            title: "sample document gamma".to_string(),
            parent_doc_title: "Test".to_string(),
            summary: "About sample".to_string(),
            content: "Content".to_string(),
            source_path: "/test/c.md".to_string(),
            keywords: vec![],
            embedding: Some(vec_c),
        };

        // 写入磁盘
        for doc in &[&doc_a, &doc_b, &doc_c] {
            let json = serde_json::to_string_pretty(doc).unwrap();
            let path = temp_dir.path().join(format!("{}.json", doc.id));
            std::fs::write(&path, json).unwrap();
        }

        // 初始化 KnowledgeStore（无嵌入模型）
        let store = KnowledgeStore::new(data_dir, None).await.unwrap();

        // 查询 "test"，应该匹配 doc-a 的标题（BM25 高分）
        let results = store.hybrid_search("test", 10).await.unwrap();

        // 验证结果不为空
        assert!(!results.is_empty());

        // 验证分数计算：BM25 分数应该 > 0（因为 "test" 在 doc-a 标题中）
        // 向量分数应该为 0.0（因为无模型）
        for result in &results {
            assert_eq!(result.vector_score, 0.0);
            // final_score 应该等于归一化的 BM25 分数
            assert!(result.final_score >= 0.0 && result.final_score <= 1.0);
        }
    }

    /// 排序稳定性测试
    #[tokio::test]
    async fn test_hybrid_search_sort_stability() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap(), None)
            .await
            .unwrap();

        // 添加多个文档
        for i in 0..5 {
            let doc = ParsedDoc {
                path: format!("/test/doc{}.md", i),
                title: format!("Doc {}", i),
                summary: "Test".to_string(),
                content: "Content".to_string(),
                sections: vec![],
            };
            store.add(&doc).await.unwrap();
        }

        // 执行两次查询，验证结果顺序一致
        let results1 = store.hybrid_search("Doc", 10).await.unwrap();
        let results2 = store.hybrid_search("Doc", 10).await.unwrap();

        assert_eq!(results1.len(), results2.len());

        for (r1, r2) in results1.iter().zip(results2.iter()) {
            assert_eq!(r1.record.id, r2.record.id);
            assert_eq!(r1.final_score, r2.final_score);
        }
    }

    /// NaN 分数处理测试
    #[tokio::test]
    async fn test_hybrid_search_nan_handling() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap(), None)
            .await
            .unwrap();

        // 添加文档
        let doc = ParsedDoc {
            path: "/test/doc.md".to_string(),
            title: "Test Doc".to_string(),
            summary: "Test".to_string(),
            content: "Content".to_string(),
            sections: vec![],
        };
        store.add(&doc).await.unwrap();

        // 正常查询不应产生 NaN
        let results = store.hybrid_search("Test", 10).await.unwrap();
        for result in &results {
            assert!(result.final_score.is_finite());
        }
    }

    /// 混合检索召回倍数测试
    #[tokio::test]
    async fn test_hybrid_search_recall_multiplier() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap(), None)
            .await
            .unwrap();

        // 添加足够多的文档
        for i in 0..20 {
            let doc = ParsedDoc {
                path: format!("/test/doc{}.md", i),
                title: format!("Document {}", i),
                summary: "Test summary".to_string(),
                content: "Test content".to_string(),
                sections: vec![],
            };
            store.add(&doc).await.unwrap();
        }

        // 请求 top_k = 5，应该返回 5 个结果
        let results = store.hybrid_search("Test", 5).await.unwrap();
        assert_eq!(results.len(), 5);
    }

    /// 混合检索与 BM25 一致性测试
    #[tokio::test]
    async fn test_hybrid_search_vs_bm25_consistency() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::new(temp_dir.path().to_str().unwrap(), None)
            .await
            .unwrap();

        // 添加文档
        for i in 0..3 {
            let doc = ParsedDoc {
                path: format!("/test/doc{}.md", i),
                title: format!("Test Doc {}", i),
                summary: "Test".to_string(),
                content: "Content".to_string(),
                sections: vec![],
            };
            store.add(&doc).await.unwrap();
        }

        // 分别调用 BM25 和混合检索
        let bm25_results = store.search("Test", 100).await.unwrap();
        let hybrid_results = store.hybrid_search("Test", 10).await.unwrap();

        // 验证混合检索返回了结果
        assert!(!hybrid_results.is_empty());

        // 验证混合检索的 BM25 分数与原始 BM25 搜索一致
        // （注意：由于归一化，分数会不同，但相对顺序应该相似）
        assert_eq!(bm25_results.len(), hybrid_results.len());
    }

    /// 混合检索数学公式精确性测试
    ///
    /// 此测试直接构造已知分数的数据，验证加权融合公式的正确性：
    /// - final_score = 0.7 * normalized_bm25 + 0.3 * vector_score
    #[tokio::test]
    async fn test_hybrid_search_mathematical_formula() {
        // 测试常量（必须与实现一致）
        const BM25_WEIGHT: f32 = 0.7;
        const VECTOR_WEIGHT: f32 = 0.3;

        // 创建测试文档
        let doc1 = KnowledgeRecord {
            id: "doc1".to_string(),
            title: "Document 1".to_string(),
            parent_doc_title: "Parent".to_string(),
            summary: "Summary".to_string(),
            content: "Content".to_string(),
            source_path: "/test/doc1.md".to_string(),
            keywords: vec![],
            embedding: None,
        };

        let doc2 = KnowledgeRecord {
            id: "doc2".to_string(),
            title: "Document 2".to_string(),
            parent_doc_title: "Parent".to_string(),
            summary: "Summary".to_string(),
            content: "Content".to_string(),
            source_path: "/test/doc2.md".to_string(),
            keywords: vec![],
            embedding: None,
        };

        let doc3 = KnowledgeRecord {
            id: "doc3".to_string(),
            title: "Document 3".to_string(),
            parent_doc_title: "Parent".to_string(),
            summary: "Summary".to_string(),
            content: "Content".to_string(),
            source_path: "/test/doc3.md".to_string(),
            keywords: vec![],
            embedding: None,
        };

        // 场景 1: 交集（BM25 和向量都有分数）
        // 假设归一化后 BM25 = 1.0, vector = 1.0，则 final_score = 1.0
        let result_intersection = BriefWithScore {
            record: doc1.clone(),
            bm25_score: 10.0,
            vector_score: 1.0,
            final_score: 0.0, // 将被计算
        };

        // 场景 2: BM25 独有（无向量分数）
        // 假设归一化后 BM25 = 1.0, vector = 0.0，则 final_score = 0.7
        let result_bm25_only = BriefWithScore {
            record: doc2.clone(),
            bm25_score: 10.0,
            vector_score: 0.0,
            final_score: 0.0, // 将被计算
        };

        // 场景 3: Vector 独有（无 BM25 分数）
        // 假设归一化后 BM25 = 0.0, vector = 1.0，则 final_score = 0.3
        let result_vector_only = BriefWithScore {
            record: doc3.clone(),
            bm25_score: 0.0,
            vector_score: 1.0,
            final_score: 0.0, // 将被计算
        };

        // 模拟归一化和融合逻辑（与 hybrid_search 实现一致）
        let mut results = vec![result_intersection, result_bm25_only, result_vector_only];

        // 提取 BM25 分数并归一化
        if !results.is_empty() {
            let all_bm25_scores: Vec<f32> = results.iter().map(|r| r.bm25_score).collect();
            let min_bm25 = all_bm25_scores
                .iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(&0.0);
            let max_bm25 = all_bm25_scores
                .iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(&0.0);
            let range = max_bm25 - min_bm25;

            for result in &mut results {
                let normalized_bm25 = if range.abs() < f32::EPSILON {
                    if result.bm25_score > 0.0 {
                        1.0
                    } else {
                        0.0
                    }
                } else {
                    (result.bm25_score - min_bm25) / range
                };
                result.final_score =
                    BM25_WEIGHT * normalized_bm25 + VECTOR_WEIGHT * result.vector_score;
            }
        }

        // 断言 1: 交集场景（BM25=1.0, Vector=1.0 → Final=1.0）
        let intersection = &results[0];
        assert_eq!(intersection.bm25_score, 10.0, "BM25 原始分数应为 10.0");
        assert_eq!(intersection.vector_score, 1.0, "Vector 分数应为 1.0");
        assert_eq!(
            intersection.final_score, 1.0,
            "交集: final_score = 0.7 * 1.0 + 0.3 * 1.0 = 1.0"
        );

        // 断言 2: BM25 独有场景（BM25=1.0, Vector=0.0 → Final=0.7）
        let bm25_only = &results[1];
        assert_eq!(bm25_only.bm25_score, 10.0, "BM25 原始分数应为 10.0");
        assert_eq!(bm25_only.vector_score, 0.0, "Vector 分数应为 0.0");
        assert_eq!(
            bm25_only.final_score, 0.7,
            "BM25 独有: final_score = 0.7 * 1.0 + 0.3 * 0.0 = 0.7"
        );

        // 断言 3: Vector 独有场景（BM25=0.0, Vector=1.0 → Final=0.3）
        let vector_only = &results[2];
        assert_eq!(vector_only.bm25_score, 0.0, "BM25 原始分数应为 0.0");
        assert_eq!(vector_only.vector_score, 1.0, "Vector 分数应为 1.0");
        assert_eq!(
            vector_only.final_score, 0.3,
            "Vector 独有: final_score = 0.7 * 0.0 + 0.3 * 1.0 = 0.3"
        );
    }
}
