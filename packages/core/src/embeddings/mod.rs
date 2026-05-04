//! Text embedding module using FastEmbed.
//!
//! This module provides a wrapper around FastEmbed's TextEmbedding to generate
//! 384-dimensional vectors from text using the BGE-small-en-v1.5 model.
//!
//! # Modules
//!
//! - [`math`] - Vector math operations (cosine similarity)
//!
//! # Thread Safety
//!
//! [`EmbeddingModel`] implements `Send + Sync` and can be safely wrapped in `Arc`
//! for concurrent use across multiple threads. The `embed_text` and `embed_batch`
//! methods use interior mutability via `Mutex` for thread-safe concurrent access.
//!
//! # Performance
//!
//! Model initialization is expensive (downloads and loads ONNX model) and should
//! be done once at application startup. Subsequent `embed_text` calls are fast
//! (< 100ms for single text).
//!
//! # Environment Variables
//!
//! - `FASTEMBED_CACHE_DIR`: Optional path to store downloaded ONNX models.
//!   If not set, FastEmbed uses the system default cache directory.
//!   Set this to share model files across multiple worktrees or CI environments.
//!
//! # Example
//!
//! ```rust,no_run
//! use contextfy_core::embeddings::EmbeddingModel;
//!
//! # fn main() -> anyhow::Result<()> {
//! // Initialize once at startup
//! let model = EmbeddingModel::new()?;
//!
//! // Generate embedding for text
//! let vector = model.embed_text("Hello, world!")?;
//! assert_eq!(vector.len(), 384);
//! # Ok(())
//! # }
//! ```

pub mod math;

use anyhow::Context;
use fastembed::{EmbeddingModel as FastEmbedModel, InitOptions, TextEmbedding};
use std::sync::Mutex;

/// Private trait abstracting over different embedding implementations
///
/// This trait allows us to use either real FastEmbed TextEmbedding or fake test embeddings
/// interchangeably within the EmbeddingModel.
trait EmbeddingInner: Send + Sync {
    /// Generate embeddings for a batch of texts
    fn embed(&mut self, texts: Vec<&str>) -> anyhow::Result<Vec<Vec<f32>>>;
}

/// Wrapper for FastEmbed's TextEmbedding to implement our EmbeddingInner trait
struct RealEmbeddingWrapper(TextEmbedding);

impl EmbeddingInner for RealEmbeddingWrapper {
    fn embed(&mut self, texts: Vec<&str>) -> anyhow::Result<Vec<Vec<f32>>> {
        // Delegate to the real TextEmbedding implementation
        self.0
            .embed(texts, None)
            .map_err(|e| anyhow::anyhow!("FastEmbed embedding failed: {}", e))
    }
}

/// Text embedding model wrapper.
///
/// Wraps FastEmbed's `TextEmbedding` with a simplified API optimized for
/// single-text embedding generation. Uses BGE-small-en-v1.5 model by default,
/// producing 384-dimensional float vectors.
///
/// # Thread Safety
///
/// This type is `Send + Sync` and can be safely wrapped in `Arc` for concurrent access.
/// The `embed_text()` and `embed_batch()` methods use `Mutex` to ensure thread-safe
/// access to the underlying ONNX model. Multiple threads can safely call these methods
/// concurrently with automatic locking.
///
/// # Performance
///
/// - **Initialization**: Expensive (model download + ONNX loading), do once at startup
/// - **Per-query**: < 100ms for single text (after first call)
/// - **Concurrency**: Thread-safe via Mutex with minimal contention (embedding is CPU-bound)
pub struct EmbeddingModel {
    inner: Mutex<Box<dyn EmbeddingInner>>,
}

impl EmbeddingModel {
    /// Initializes a new embedding model with BGE-small-en-v1.5.
    ///
    /// This method downloads the ONNX model on first run (cached locally) and
    /// initializes the inference runtime. Subsequent calls load from cache.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Model download fails (network issue or disk full)
    /// - ONNX runtime initialization fails
    /// - Model file is corrupted
    ///
    /// # Example
    ///
    /// ```rust
    /// use anyhow::Context;
    /// use contextfy_core::embeddings::EmbeddingModel;
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let model = EmbeddingModel::new()
    ///     .context("Failed to initialize embedding model")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new() -> anyhow::Result<Self> {
        let inner = TextEmbedding::try_new(
            InitOptions::new(FastEmbedModel::BGESmallENV15).with_show_download_progress(true),
        )
        .context("Failed to initialize FastEmbed TextEmbedding with BGE-small-en-v1.5")?;

        Ok(Self {
            inner: Mutex::new(Box::new(RealEmbeddingWrapper(inner))),
        })
    }

    /// Creates a lightweight test stub that doesn't load the real BGE model.
    ///
    /// This is intended for unit tests where:
    /// - Real embedding semantics aren't important
    /// - Fast test execution is critical
    /// - Network access or model downloads should be avoided
    ///
    /// # Important
    ///
    /// **TESTING ONLY**. Never use this in production code.
    /// The stub generates deterministic vectors based on text hash, not real embeddings.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[cfg(test)]
    /// use contextfy_core::embeddings::EmbeddingModel;
    ///
    /// #[test]
    /// fn test_with_stub() {
    ///     let model = EmbeddingModel::test_stub();
    ///     let vector = model.embed_text("test").unwrap();
    ///     assert_eq!(vector.len(), 384);
    /// }
    /// ```
    #[cfg(test)]
    pub fn test_stub() -> Self {
        // For testing, we use a fake TextEmbedding that generates deterministic vectors
        // This avoids the expensive model download and initialization
        let fake_inner = FakeTextEmbedding::new();
        Self {
            inner: Mutex::new(Box::new(fake_inner)),
        }
    }

    /// Generates a 384-dimensional embedding vector for the given text.
    ///
    /// # Arguments
    ///
    /// * `text` - The input text to embed (can be empty string)
    ///
    /// # Returns
    ///
    /// A `Vec<f32>` of length 384 containing the embedding vector.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Text encoding fails
    /// - ONNX inference fails
    /// - Returned embedding has unexpected dimension
    ///
    /// # Safety Guarantees
    ///
    /// This method:
    /// - Never panics on valid UTF-8 input
    /// - Returns descriptive errors via `anyhow::Context`
    /// - Validates output dimension (must be 384)
    /// - Uses Mutex to ensure thread-safe access
    ///
    /// # Example
    ///
    /// ```rust
    /// use contextfy_core::embeddings::EmbeddingModel;
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// # let model = EmbeddingModel::new()?;
    /// let vector = model.embed_text("Hello, world!")?;
    /// assert_eq!(vector.len(), 384);
    /// # Ok(())
    /// # }
    /// ```
    pub fn embed_text(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        // Use Mutex to ensure thread-safe access
        let mut inner = self
            .inner
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire lock: {}", e))?;

        // FastEmbed's embed method accepts Vec<&str> and returns Vec<Vec<f32>>
        // We wrap the single text in an array and extract the first result safely
        let embeddings = inner
            .embed(vec![text])
            .context("Failed to generate embedding for text")?;

        // Safely extract the first (and only) embedding from the batch result
        let embedding = embeddings
            .into_iter()
            .next()
            .context("Embedding batch returned empty results")?;

        // Validate dimension (BGE-small-en-v1.5 always produces 384-dimensional vectors)
        if embedding.len() != 384 {
            anyhow::bail!(
                "Expected 384-dimensional embedding, got {} dimensions",
                embedding.len()
            );
        }

        Ok(embedding)
    }

    /// 批量生成 384 维嵌入向量
    ///
    /// 一次性对多个文本生成向量，利用 FastEmbed 的原生批处理能力。
    /// 相比多次调用 `embed_text`，批处理可以显著减少总耗时。
    ///
    /// # Arguments
    ///
    /// * `texts` - 待嵌入的文本切片引用
    ///
    /// # Returns
    ///
    /// `Vec<Vec<f32>>`，每个向量的长度为 384。返回的向量数量与输入文本数量一致。
    ///
    /// # Errors
    ///
    /// 返回错误如果：
    /// - 任何文本编码失败
    /// - ONNX 推理失败
    /// - 任何返回的嵌入向量维度不是 384
    ///
    /// # Performance
    ///
    /// - 批处理 N 条文本的总耗时显著低于 N 次单独调用 `embed_text` 的总和
    /// - 利用 FastEmbed 的批处理优化，减少模型推理开销
    /// - API 接受借用切片，调用方无需分配内存
    ///
    /// # Safety Guarantees
    ///
    /// 本方法：
    /// - 在有效的 UTF-8 输入上从不 panic
    /// - 通过 `anyhow::Context` 返回描述性错误
    /// - 验证所有输出维度（必须都是 384）
    /// - 使用 Mutex 确保线程安全访问（与 `embed_text` 相同的安全保证）
    ///
    /// # Example
    ///
    /// ```rust
    /// use contextfy_core::embeddings::EmbeddingModel;
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// # let model = EmbeddingModel::new()?;
    /// let texts = vec!["Hello, world!", "Goodbye, world!"];
    /// let vectors = model.embed_batch(&texts)?;
    /// assert_eq!(vectors.len(), 2);
    /// assert_eq!(vectors[0].len(), 384);
    /// assert_eq!(vectors[1].len(), 384);
    /// # Ok(())
    /// # }
    /// ```
    pub fn embed_batch(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        // 记录输入文本数量，用于后续校验
        let expected_len = texts.len();

        // Use Mutex to ensure thread-safe access
        let mut inner = self
            .inner
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire lock: {}", e))?;

        // 直接调用 FastEmbed 的批处理接口
        let embeddings = inner
            .embed(texts.to_vec())
            .context("Failed to generate embeddings for batch")?;

        // 契约防线：校验返回的向量数量是否等于输入的文本数量
        if embeddings.len() != expected_len {
            anyhow::bail!(
                "Embedding batch contract violation: expected {} vectors, got {}",
                expected_len,
                embeddings.len()
            );
        }

        // 预分配容量以减少扩容开销
        let mut result = Vec::with_capacity(embeddings.len());

        // 转换为 Vec 并验证每个向量的维度
        for (idx, embedding) in embeddings.into_iter().enumerate() {
            if embedding.len() != 384 {
                anyhow::bail!(
                    "Expected 384-dimensional embedding for text {}, got {} dimensions",
                    idx,
                    embedding.len()
                );
            }
            result.push(embedding);
        }

        Ok(result)
    }
}

// Note: EmbeddingModel is Send + Sync because Mutex<T> is Send + Sync when T is Send.
// No unsafe impl needed - Mutex provides the necessary guarantees.

/// Fake TextEmbedding implementation for lightweight testing.
///
/// This type mimics fastembed::TextEmbedding but generates deterministic vectors
/// based on text hash instead of running actual ONNX inference.
#[cfg(test)]
struct FakeTextEmbedding;

#[cfg(test)]
impl FakeTextEmbedding {
    fn new() -> Self {
        Self
    }
}

#[cfg(test)]
impl EmbeddingInner for FakeTextEmbedding {
    /// Mimics fastembed's embed() method but returns deterministic fake vectors
    fn embed(&mut self, texts: Vec<&str>) -> anyhow::Result<Vec<Vec<f32>>> {
        texts
            .iter()
            .map(|&text| {
                let mut vector = Vec::with_capacity(384);
                let mut hash: u64 = 5381;
                for byte in text.bytes() {
                    hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
                }
                for i in 0..384 {
                    let mixed_hash = hash.wrapping_mul(i as u64).wrapping_add(i as u64);
                    let value = (mixed_hash % 1000) as f32 / 1000.0;
                    vector.push(value);
                }
                Ok(vector)
            })
            .collect()
    }
}

/// Fake embedding backend for testing purposes.
///
/// This type provides a deterministic, no-op implementation of the embedding
/// interface for unit tests that don't require real embedding generation.
/// It generates 384-dimensional vectors based on a simple hash of the input text,
/// ensuring identical inputs produce identical outputs.
///
/// **IMPORTANT**: This is for TESTING ONLY. Never use this in production code.
///
/// # Properties
///
/// - Deterministic: Same text always produces the same vector
/// - Fast: No ONNX model loading or inference
/// - No external dependencies: Works offline without model downloads
/// - Predictable dimension: Always returns 384-dimensional vectors
///
/// # Example
///
/// ```rust
/// use contextfy_core::embeddings::FakeEmbeddingBackend;
///
/// # fn main() -> anyhow::Result<()> {
/// let fake = FakeEmbeddingBackend::new();
/// let vector = fake.embed_text("test")?;
/// assert_eq!(vector.len(), 384);
/// # Ok(())
/// # }
/// ```
#[cfg(test)]
pub struct FakeEmbeddingBackend;

#[cfg(test)]
impl FakeEmbeddingBackend {
    /// Creates a new fake embedding backend.
    pub fn new() -> Self {
        Self
    }

    /// Generates a deterministic 384-dimensional fake embedding vector.
    ///
    /// The vector is generated using a simple hash of the input text, ensuring
    /// that identical inputs produce identical outputs. This is useful for
    /// deterministic testing.
    pub fn embed_text(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let mut vector = Vec::with_capacity(384);

        // Use a simple hash algorithm to generate deterministic values
        let mut hash: u64 = 5381;
        for byte in text.bytes() {
            hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
        }

        // Generate 384 dimensions from the hash
        for i in 0..384 {
            // Mix the hash with the index to get different values per dimension
            let mixed_hash = hash.wrapping_mul(i as u64).wrapping_add(i as u64);
            // Normalize to [0, 1] range
            let value = (mixed_hash % 1000) as f32 / 1000.0;
            vector.push(value);
        }

        Ok(vector)
    }

    /// Generates multiple deterministic fake embedding vectors.
    pub fn embed_batch(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        texts.iter().map(|&text| self.embed_text(text)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_model_loading() {
        // Test that model initialization succeeds
        let model = EmbeddingModel::new();
        assert!(model.is_ok(), "Model initialization should succeed");
    }

    #[test]
    #[serial]
    fn test_embedding_dimension() {
        // Test that embeddings have exactly 384 dimensions
        let model = EmbeddingModel::new().expect("Model should initialize");
        let vector = model
            .embed_text("Test text for dimension validation")
            .expect("Embedding generation should succeed");

        assert_eq!(
            vector.len(),
            384,
            "Embedding vector must have exactly 384 dimensions"
        );
    }

    #[test]
    #[serial]
    fn test_embedding_determinism() {
        // Test that identical text produces identical embeddings
        let model = EmbeddingModel::new().expect("Model should initialize");
        let text = "Deterministic test text";

        let vector1 = model
            .embed_text(text)
            .expect("First embedding should succeed");
        let vector2 = model
            .embed_text(text)
            .expect("Second embedding should succeed");

        // All dimensions should match (floating-point equality is fine here)
        assert_eq!(
            vector1.len(),
            vector2.len(),
            "Embedding dimensions should match"
        );

        for (i, (v1, v2)) in vector1.iter().zip(vector2.iter()).enumerate() {
            assert!(
                (v1 - v2).abs() < 1e-6,
                "Dimension {} differs: {} vs {}",
                i,
                v1,
                v2
            );
        }
    }

    #[test]
    #[serial]
    fn test_empty_text_embedding() {
        // Test that empty text produces valid embedding
        let model = EmbeddingModel::new().expect("Model should initialize");
        let vector = model
            .embed_text("")
            .expect("Empty text embedding should succeed");

        assert_eq!(
            vector.len(),
            384,
            "Empty text embedding should be 384-dimensional"
        );
    }

    #[test]
    #[serial]
    fn test_unicode_text_embedding() {
        // Test that Unicode text (Chinese, emojis) works correctly
        let model = EmbeddingModel::new().expect("Model should initialize");
        let text = "你好世界 🚀 Hello 世界!";

        let vector = model
            .embed_text(text)
            .expect("Unicode text embedding should succeed");

        assert_eq!(
            vector.len(),
            384,
            "Unicode text embedding should be 384-dimensional"
        );
    }

    #[test]
    #[serial]
    #[ignore]
    fn bench_embedding_speed() {
        // Performance benchmark: single text embedding should be < 100ms
        // Run with: cargo test --package contextfy-core bench_embedding_speed -- --ignored
        let model = EmbeddingModel::new().expect("Model should initialize");

        // Warm-up call (model loading overhead)
        let _ = model
            .embed_text("Warm-up text")
            .expect("Warm-up should succeed");

        // Measure single text embedding (typical use case)
        let test_text = "This is a typical text document that might be embedded in a production environment. It contains enough content to represent a realistic scenario.";
        let start = std::time::Instant::now();
        let vector = model
            .embed_text(test_text)
            .expect("Embedding should succeed");
        let duration = start.elapsed();

        println!("Embedding generation took: {:?}", duration);
        println!("Generated vector of {} dimensions", vector.len());

        // Assert performance requirement: < 100ms
        assert!(
            duration.as_millis() < 100,
            "Embedding generation should be < 100ms, took {}ms",
            duration.as_millis()
        );

        // Also verify dimension
        assert_eq!(vector.len(), 384, "Vector must be 384-dimensional");
    }

    #[test]
    #[serial]
    fn test_batch_embedding_basic() {
        // Test basic batch embedding functionality
        let model = EmbeddingModel::new().expect("Model should initialize");
        let texts = vec!["Hello, world!", "Goodbye, world!"];

        let vectors = model
            .embed_batch(&texts)
            .expect("Batch embedding should succeed");

        assert_eq!(vectors.len(), 2, "Should return 2 vectors");
        assert_eq!(
            vectors[0].len(),
            384,
            "First vector must be 384-dimensional"
        );
        assert_eq!(
            vectors[1].len(),
            384,
            "Second vector must be 384-dimensional"
        );
    }

    #[test]
    #[serial]
    fn test_batch_embedding_empty() {
        // Test that empty input returns empty result
        let model = EmbeddingModel::new().expect("Model should initialize");
        let texts: Vec<&str> = vec![];

        let vectors = model
            .embed_batch(&texts)
            .expect("Empty batch embedding should succeed");

        assert_eq!(vectors.len(), 0, "Empty input should return empty result");
    }

    #[test]
    #[serial]
    fn test_batch_embedding_single() {
        // Test that single-item batch works the same as embed_text
        let model = EmbeddingModel::new().expect("Model should initialize");
        let text = "Test text for single-item batch";

        // Using embed_batch with single item
        let batch_vectors = model
            .embed_batch(&[text])
            .expect("Single-item batch should succeed");

        // Using embed_text
        let single_vector = model.embed_text(text).expect("embed_text should succeed");

        assert_eq!(batch_vectors.len(), 1, "Should return 1 vector");
        assert_eq!(
            batch_vectors[0].len(),
            384,
            "Vector must be 384-dimensional"
        );

        // Verify they produce identical results
        assert_eq!(
            batch_vectors[0].len(),
            single_vector.len(),
            "Dimensions should match"
        );
        for (i, (v1, v2)) in batch_vectors[0]
            .iter()
            .zip(single_vector.iter())
            .enumerate()
        {
            assert!(
                (v1 - v2).abs() < 1e-6,
                "Dimension {} differs between batch and single: {} vs {}",
                i,
                v1,
                v2
            );
        }
    }

    #[test]
    #[serial]
    fn test_batch_embedding_multiple() {
        // Test batch processing with multiple texts
        let model = EmbeddingModel::new().expect("Model should initialize");
        let texts = vec![
            "First text",
            "Second text",
            "Third text",
            "Fourth text",
            "Fifth text",
        ];

        let vectors = model
            .embed_batch(&texts)
            .expect("Multi-item batch should succeed");

        assert_eq!(vectors.len(), 5, "Should return 5 vectors");
        for (idx, vector) in vectors.iter().enumerate() {
            assert_eq!(vector.len(), 384, "Vector {} must be 384-dimensional", idx);
        }
    }

    #[test]
    #[serial]
    fn test_batch_embedding_unicode() {
        // Test batch processing with Unicode text
        let model = EmbeddingModel::new().expect("Model should initialize");
        let texts = vec!["你好世界 🚀", "Hello 世界", "Test 测试"];

        let vectors = model
            .embed_batch(&texts)
            .expect("Unicode batch should succeed");

        assert_eq!(vectors.len(), 3, "Should return 3 vectors");
        for vector in &vectors {
            assert_eq!(vector.len(), 384, "All vectors must be 384-dimensional");
        }
    }

    #[test]
    #[serial]
    #[ignore]
    fn bench_batch_vs_single() {
        // Performance benchmark: batch should be faster than multiple singles
        // Run with: cargo test --package contextfy-core bench_batch_vs_single -- --ignored
        let model = EmbeddingModel::new().expect("Model should initialize");

        let texts: Vec<&str> = vec![
            "Document 1: This is a test document with some content.",
            "Document 2: Another document with different content.",
            "Document 3: Third document for batch processing.",
            "Document 4: Fourth document to test performance.",
            "Document 5: Fifth document completes the set.",
        ];

        // Warm-up
        let _ = model.embed_batch(&texts);

        // Measure batch processing
        let start_batch = std::time::Instant::now();
        let batch_vectors = model.embed_batch(&texts).unwrap();
        let duration_batch = start_batch.elapsed();

        // Measure individual processing
        let start_single = std::time::Instant::now();
        let mut single_vectors = Vec::new();
        for text in &texts {
            single_vectors.push(model.embed_text(text).unwrap());
        }
        let duration_single = start_single.elapsed();

        println!("Batch processing took: {:?}", duration_batch);
        println!("Individual processing took: {:?}", duration_single);
        println!(
            "Speedup: {:.2}x",
            duration_single.as_millis() as f64 / duration_batch.as_millis() as f64
        );

        // Verify results match
        assert_eq!(batch_vectors.len(), single_vectors.len());
        for (i, (batch_vec, single_vec)) in
            batch_vectors.iter().zip(single_vectors.iter()).enumerate()
        {
            assert_eq!(batch_vec.len(), single_vec.len());
            for (j, (v1, v2)) in batch_vec.iter().zip(single_vec.iter()).enumerate() {
                assert!(
                    (v1 - v2).abs() < 1e-6,
                    "Vector {} dimension {} differs: {} vs {}",
                    i,
                    j,
                    v1,
                    v2
                );
            }
        }

        // Batch should be faster or at least not significantly slower
        assert!(
            duration_batch <= duration_single,
            "Batch processing should be faster or equal to individual processing"
        );
    }
}
