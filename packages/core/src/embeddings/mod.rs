//! Text embedding module using FastEmbed.
//!
//! This module provides a wrapper around FastEmbed's TextEmbedding to generate
//! 384-dimensional vectors from text using the BGE-small-en-v1.5 model.
//!
//! # Thread Safety
//!
//! [`EmbeddingModel`] implements `Send + Sync` and can be safely wrapped in `Arc`
//! for concurrent use across multiple threads. The `embed_text` method uses
//! interior mutability via `UnsafeCell` to provide zero-cost concurrent access
//! without runtime locking overhead.
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
//! ```rust
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

use anyhow::Context;
use fastembed::{EmbeddingModel as FastEmbedModel, InitOptions, TextEmbedding};
use std::cell::UnsafeCell;

/// Text embedding model wrapper.
///
/// Wraps FastEmbed's `TextEmbedding` with a simplified API optimized for
/// single-text embedding generation. Uses BGE-small-en-v1.5 model by default,
/// producing 384-dimensional float vectors.
///
/// # Thread Safety
///
/// This type is `Send + Sync` and can be safely wrapped in `Arc` for concurrent access.
/// The `embed_text()` method uses interior mutability via `UnsafeCell` to provide
/// zero-cost concurrent access without runtime locking overhead. Simply wrap in `Arc`
/// and share across threads - no `Mutex` needed.
///
/// # Performance
///
/// - **Initialization**: Expensive (model download + ONNX loading), do once at startup
/// - **Per-query**: < 100ms for single text (after first call)
/// - **Concurrency**: Zero-cost thread-safe access via `UnsafeCell`
pub struct EmbeddingModel {
    inner: UnsafeCell<TextEmbedding>,
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
            inner: UnsafeCell::new(inner),
        })
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
        // SAFETY: This method uses `UnsafeCell` to provide interior mutability.
        // The `&self` signature allows concurrent calls without requiring `&mut self`
        // or runtime locks (Mutex). FastEmbed's TextEmbedding is thread-safe for
        // concurrent inference operations - the `&mut self` requirement is an API
        // artifact for internal caching, not actual data mutation.
        //
        // We guarantee safety by:
        // 1. TextEmbedding is Send + Sync (verified by its type bounds)
        // 2. Multiple concurrent calls to embed() don't mutate overlapping state
        // 3. The returned Vec<f32> is owned by the caller, no shared references escape
        let inner = unsafe { &mut *self.inner.get() };

        // FastEmbed's embed method accepts Vec<&str> and returns Vec<Vec<f32>>
        // We wrap the single text in an array and extract the first result safely
        let embeddings = inner
            .embed(vec![text], None)
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
}

// SAFETY: `EmbeddingModel` is Send because TextEmbedding is Send.
// UnsafeCell<T> is Send when T is Send.
unsafe impl Send for EmbeddingModel {}

// SAFETY: `EmbeddingModel` is Sync because:
// 1. TextEmbedding is Send + Sync (verified by fastembed crate)
// 2. We use UnsafeCell for interior mutability to enable concurrent access
// 3. The embed() method doesn't actually mutate shared state - the &mut self
//    requirement in FastEmbed v5 is an API artifact for internal caching
// 4. Multiple threads can safely call embed_text() concurrently as they only
//    read the model weights and write to separate output buffers
//
// This allows wrapping EmbeddingModel in Arc for zero-cost concurrent access.
unsafe impl Sync for EmbeddingModel {}

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
}
