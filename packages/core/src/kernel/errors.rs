//! Layered error model for the knowledge engine
//!
//! This module defines a hierarchical error model that distinguishes between:
//! - **DomainError**: Business logic errors (invalid input, not found, etc.)
//! - **InfraError**: Infrastructure adapter errors (database, network, etc.)
//!
//! **MANDATORY CONSTRAINT**: Errors must preserve root cause chains.
//! NEVER use `.to_string()` to flatten errors - always use `#[source]` attribute.
//!
//! Ref: `openspec/changes/refactor-pragmatic-slice-architecture/design.md` - Rule 3

use std::path::PathBuf;
use thiserror::Error;

/// Unified application error
///
/// Wraps all error types that can occur in the core engine.
/// This allows clean error propagation with proper source chain preservation.
#[derive(Error, Debug)]
pub enum AppError {
    /// Business logic errors (invalid input, not found, etc.)
    #[error("domain error: {0}")]
    Domain(#[from] DomainError),

    /// Infrastructure adapter errors (database, network, etc.)
    #[error("infrastructure error: {0}")]
    Infra(#[from] InfraError),
}

/// Domain layer errors
///
/// These errors represent business logic violations and invalid application states.
/// They are NOT related to infrastructure failures.
#[derive(Error, Debug)]
pub enum DomainError {
    /// Query validation failed (e.g., empty text, invalid limit)
    #[error("invalid query: {0}")]
    InvalidQuery(String),

    /// Requested resource was not found
    #[error("resource not found: {0}")]
    NotFound(String),

    /// Operation is not allowed in the current context
    #[error("operation not allowed: {0}")]
    NotAllowed(String),

    /// Generic domain logic error
    #[error("{0}")]
    Other(String),
}

impl DomainError {
    /// Create a new invalid query error
    pub fn invalid_query(msg: impl Into<String>) -> Self {
        Self::InvalidQuery(msg.into())
    }

    /// Create a new not found error
    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::NotFound(resource.into())
    }

    /// Create a new not allowed error
    pub fn not_allowed(msg: impl Into<String>) -> Self {
        Self::NotAllowed(msg.into())
    }
}

/// Infrastructure layer errors
///
/// These errors represent failures in external systems and infrastructure adapters.
/// They wrap underlying errors (IO, database, network) while preserving context.
#[derive(Error, Debug)]
pub enum InfraError {
    /// Database connection or query error
    #[error("database error: {context}")]
    Database {
        context: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// File system I/O error
    #[error("io error at {path}: {context}")]
    Io {
        path: PathBuf,
        context: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Network or HTTP error
    #[error("network error: {context}")]
    Network {
        context: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Serialization/deserialization error
    #[error("serialization error: {context}")]
    Serialization {
        context: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Generic infrastructure error
    #[error("{0}")]
    Other(String),
}

impl InfraError {
    /// Create a new database error
    pub fn database(
        context: impl Into<String>,
        source: Option<impl Into<Box<dyn std::error::Error + Send + Sync>>>,
    ) -> Self {
        Self::Database {
            context: context.into(),
            source: source.map(|s| s.into()),
        }
    }

    /// Create a new I/O error
    pub fn io(
        path: impl Into<PathBuf>,
        context: impl Into<String>,
        source: Option<impl Into<Box<dyn std::error::Error + Send + Sync>>>,
    ) -> Self {
        Self::Io {
            path: path.into(),
            context: context.into(),
            source: source.map(|s| s.into()),
        }
    }

    /// Create a new network error
    pub fn network(
        context: impl Into<String>,
        source: Option<impl Into<Box<dyn std::error::Error + Send + Sync>>>,
    ) -> Self {
        Self::Network {
            context: context.into(),
            source: source.map(|s| s.into()),
        }
    }

    /// Create a new serialization error
    pub fn serialization(
        context: impl Into<String>,
        source: Option<impl Into<Box<dyn std::error::Error + Send + Sync>>>,
    ) -> Self {
        Self::Serialization {
            context: context.into(),
            source: source.map(|s| s.into()),
        }
    }
}

// NOTE: We intentionally do NOT provide a blanket From<std::io::Error> for AppError
// because it loses important context (file path and operation details).
// Instead, use InfraError::io() with proper context at each call site.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_error_display() {
        let err = DomainError::invalid_query("query text is empty");
        assert_eq!(err.to_string(), "invalid query: query text is empty");

        let err = DomainError::not_found("document 'doc1'");
        assert_eq!(err.to_string(), "resource not found: document 'doc1'");
    }

    #[test]
    fn test_infra_error_display() {
        let err = InfraError::database("connection failed", None::<std::io::Error>);
        assert_eq!(err.to_string(), "database error: connection failed");
    }

    #[test]
    fn test_app_error_from_domain() {
        let domain_err = DomainError::not_found("test");
        let app_err: AppError = domain_err.into();
        assert!(matches!(app_err, AppError::Domain(_)));
        assert_eq!(app_err.to_string(), "domain error: resource not found: test");
    }

    #[test]
    fn test_app_error_from_infra() {
        let infra_err = InfraError::database("query failed", None::<std::io::Error>);
        let app_err: AppError = infra_err.into();
        assert!(matches!(app_err, AppError::Infra(_)));
        assert_eq!(app_err.to_string(), "infrastructure error: database error: query failed");
    }

    #[test]
    fn test_source_chain_preserved() {
        // Create an error with a source
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let infra_err = InfraError::io("/path/to/file", "failed to read", Some(io_err));

        // The source should be preserved in the error chain
        let mut error_chain = Some(&infra_err as &dyn std::error::Error);
        let mut found_source = false;

        while let Some(err) = error_chain {
            if let Some(_source) = err.source() {
                found_source = true;
                break;
            }
            error_chain = std::error::Error::source(err);
        }

        assert!(found_source, "Source chain should be preserved");
    }
}
