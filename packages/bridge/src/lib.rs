//! # Contextfy Bridge
//!
//! This crate provides Node.js bindings for the Contextfy core library using NAPI-RS.
//!
//! ## Example
//!
//! ```javascript
//! const { ContextfyKit } = require('contextfy-bridge');
//! const kit = new ContextfyKit();
//! const results = await kit.scout('query');
//! ```

use napi_derive::napi;

/// Contextfy API module exported to Node.js.
#[napi]
pub mod contextfy {
    use super::{Brief, Details};

    /// Main API wrapper for Contextfy functionality.
    ///
    /// This struct provides the primary interface for interacting with the Contextfy
    /// knowledge base from Node.js.
    ///
    /// # Example
    ///
    /// ```javascript
    /// const { ContextfyKit } = require('contextfy-bridge');
    /// const kit = new ContextfyKit();
    /// ```
    #[napi]
    pub struct ContextfyKit {
        _private: (),
    }

    impl Default for ContextfyKit {
        fn default() -> Self {
            Self::new()
        }
    }

    #[napi]
    impl ContextfyKit {
        /// Creates a new `ContextfyKit` instance.
        ///
        /// # Example
        ///
        /// ```javascript
        /// const kit = new ContextfyKit();
        /// ```
        #[napi(constructor)]
        pub fn new() -> Self {
            Self { _private: () }
        }

        /// Searches the knowledge base for matching records.
        ///
        /// # Arguments
        ///
        /// * `query` - Search query string
        ///
        /// # Returns
        ///
        /// Returns a vector of brief information about matching records.
        ///
        /// # Example
        ///
        /// ```javascript
        /// const results = await kit.scout('Rust');
        /// console.log(results); // [{ id, title, summary }, ...]
        /// ```
        #[napi]
        pub async fn scout(&self, _query: String) -> napi::Result<Vec<Brief>> {
            Ok(vec![Brief {
                id: "stub-id-1".to_string(),
                title: "Stub Result".to_string(),
                summary: "This is a stub implementation".to_string(),
            }])
        }

        /// Retrieves detailed information about a specific record.
        ///
        /// # Arguments
        ///
        /// * `id` - The unique identifier of the record
        ///
        /// # Returns
        ///
        /// Returns detailed information including the full content of the record.
        ///
        /// # Example
        ///
        /// ```javascript
        /// const details = await kit.inspect('record-id');
        /// console.log(details.content);
        /// ```
        #[napi]
        pub async fn inspect(&self, id: String) -> napi::Result<Details> {
            Ok(Details {
                id,
                title: "Stub Details".to_string(),
                content: "This is stub content from the bridge layer".to_string(),
            })
        }
    }
}

/// Re-exports the main ContextfyKit type for convenience.
pub use contextfy::ContextfyKit;

/// Brief information about a knowledge record.
///
/// This struct is returned by search operations and contains summary information
/// about a matching record.
///
/// # Fields
///
/// * `id` - Unique identifier for the record
/// * `title` - Title of the record
/// * `summary` - Brief summary of the content (first 200 characters)
#[napi(object)]
pub struct Brief {
    /// Unique identifier for the record
    pub id: String,
    /// Title of the record
    pub title: String,
    /// Brief summary of the content (first 200 characters)
    pub summary: String,
}

/// Detailed information about a knowledge record.
///
/// This struct contains the complete content of a record and is returned by
/// the inspect operation.
///
/// # Fields
///
/// * `id` - Unique identifier for the record
/// * `title` - Title of the record
/// * `content` - Full content of the record
#[napi(object)]
pub struct Details {
    /// Unique identifier for the record
    pub id: String,
    /// Title of the record
    pub title: String,
    /// Full content of the record
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contextfy_kit_default() {
        let _kit = contextfy::ContextfyKit::default();
        // Verifies that Default implementation works without panic
    }

    #[test]
    fn test_contextfy_kit_new() {
        let _kit = contextfy::ContextfyKit::new();
        // Verifies that constructor works without panic
    }

    #[test]
    fn test_reexport() {
        // This test verifies that the re-export works correctly
        let _kit: ContextfyKit = ContextfyKit::new();
    }
}

// Note: Integration tests for async methods (scout, inspect) require a Node.js runtime
// and should be placed in the tests/ directory with a proper test harness.
// For NAPI-RS packages, consider using the JavaScript test suite in the parent project.
