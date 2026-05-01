//! Bridge error mapping layer
//!
//! This module defines the bridge-specific error type and provides conversion
//! from kernel errors to N-API compatible errors.
//!
//! **MANDATORY CONSTRAINT**: Error mapping must preserve root cause chains.
//! - NEVER use `.to_string()` to flatten errors
//! - ALWAYS use `#[source]` attribute to preserve the original error
//! - Map domain and infra errors WITHOUT semantic rewriting
//!
//! Ref: `openspec/changes/refactor-pragmatic-slice-architecture/design.md` - Rule 3
//! Ref: `openspec/changes/refactor-pragmatic-slice-architecture/specs/bridge-layer/spec.md`

use crate::kernel::errors::{AppError, DomainError, InfraError};
use napi::Error as NapiError;
use napi::Status;
use std::fmt;

/// Bridge layer error
///
/// Represents errors that occur specifically in the bridge/FFI layer.
/// This is distinct from domain and infra errors.
#[derive(Debug)]
pub enum BridgeError {
    /// Invalid argument passed from JavaScript
    InvalidArgument {
        context: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Requested resource was not found
    NotFound {
        context: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Runtime execution error (e.g., Tokio panic, deadlock)
    Runtime {
        context: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Serialization error during DTO conversion
    Serialization {
        context: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Generic bridge error
    Other(String),
}

impl BridgeError {
    /// Create a new invalid argument error
    pub fn invalid_argument(
        context: impl Into<String>,
        source: Option<impl Into<Box<dyn std::error::Error + Send + Sync>>>,
    ) -> Self {
        Self::InvalidArgument {
            context: context.into(),
            source: source.map(|s| s.into()),
        }
    }

    /// Create a new runtime error
    pub fn runtime(
        context: impl Into<String>,
        source: Option<impl Into<Box<dyn std::error::Error + Send + Sync>>>,
    ) -> Self {
        Self::Runtime {
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

    /// Create a new not found error
    pub fn not_found(
        context: impl Into<String>,
        source: Option<impl Into<Box<dyn std::error::Error + Send + Sync>>>,
    ) -> Self {
        Self::NotFound {
            context: context.into(),
            source: source.map(|s| s.into()),
        }
    }

    /// Get the error message with context
    pub fn message(&self) -> String {
        match self {
            Self::InvalidArgument { context, .. } => format!("Invalid argument: {}", context),
            Self::NotFound { context, .. } => format!("Not found: {}", context),
            Self::Runtime { context, .. } => format!("Runtime error: {}", context),
            Self::Serialization { context, .. } => format!("Serialization error: {}", context),
            Self::Other(msg) => msg.clone(),
        }
    }

    /// Get the appropriate N-API status code for this error
    pub fn status(&self) -> Status {
        match self {
            Self::InvalidArgument { .. } => Status::InvalidArg,
            Self::NotFound { .. } => Status::GenericFailure, // NAPI doesn't have NotFound, use GenericFailure
            Self::Runtime { .. } => Status::GenericFailure,
            Self::Serialization { .. } => Status::InvalidArg,
            Self::Other(_) => Status::GenericFailure,
        }
    }
}

impl fmt::Display for BridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for BridgeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidArgument { source, .. }
            | Self::NotFound { source, .. }
            | Self::Runtime { source, .. }
            | Self::Serialization { source, .. } => source
                .as_ref()
                .map(|s| s.as_ref() as &dyn std::error::Error),
            Self::Other(_) => None,
        }
    }
}

/// Convert from kernel AppError to BridgeError
///
/// This conversion preserves the original error semantics and source chain.
/// For InfraError variants with a source, the original error is boxed and preserved.
impl From<AppError> for BridgeError {
    fn from(err: AppError) -> Self {
        match err {
            AppError::Domain(domain_err) => {
                // Domain errors don't have an underlying source error
                // Box the original domain error to preserve type information
                Self::InvalidArgument {
                    context: domain_err.to_string(),
                    source: Some(Box::new(domain_err)),
                }
            }
            AppError::Infra(infra_err) => {
                // Delegate to From<InfraError> for BridgeError to avoid duplication
                infra_err.into()
            }
        }
    }
}

/// Convert from kernel DomainError to BridgeError
impl From<DomainError> for BridgeError {
    fn from(err: DomainError) -> Self {
        match err {
            DomainError::InvalidQuery(msg) => {
                Self::InvalidArgument {
                    context: msg,
                    source: None,
                }
            }
            DomainError::NotFound(msg) => {
                Self::NotFound {
                    context: msg,
                    source: None,
                }
            }
            DomainError::NotAllowed(msg) => {
                Self::InvalidArgument {
                    context: msg,
                    source: None,
                }
            }
            DomainError::Other(msg) => {
                Self::Other(msg)
            }
        }
    }
}

/// Convert from kernel InfraError to BridgeError
impl From<InfraError> for BridgeError {
    fn from(err: InfraError) -> Self {
        match err {
            InfraError::Database { context, source } => {
                Self::Runtime {
                    context: format!("Database error: {}", context),
                    source,
                }
            }
            InfraError::Io { path, context, source } => {
                Self::Runtime {
                    context: format!("IO error at {}: {}", path.display(), context),
                    source,
                }
            }
            InfraError::Network { context, source } => {
                Self::Runtime {
                    context: format!("Network error: {}", context),
                    source,
                }
            }
            InfraError::Serialization { context, source } => {
                Self::Serialization {
                    context,
                    source,
                }
            }
            InfraError::Other(msg) => {
                Self::Runtime {
                    context: format!("Infrastructure error: {}", msg),
                    source: None,
                }
            }
        }
    }
}

/// Convert BridgeError to N-API Error
///
/// This conversion is used when propagating errors to JavaScript.
/// It preserves the error message and status code.
impl From<BridgeError> for NapiError {
    fn from(err: BridgeError) -> Self {
        NapiError::new(err.status(), err.message())
    }
}

/// Convert from AppError to N-API Error
///
/// This is a convenience conversion that goes through BridgeError.
impl From<AppError> for NapiError {
    fn from(err: AppError) -> Self {
        let bridge_err: BridgeError = err.into();
        bridge_err.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_bridge_error_display() {
        let err = BridgeError::invalid_argument("query text is empty", None::<std::io::Error>);
        assert_eq!(err.to_string(), "Invalid argument: query text is empty");

        let err = BridgeError::runtime("Tokio runtime panicked", None::<std::io::Error>);
        assert_eq!(err.to_string(), "Runtime error: Tokio runtime panicked");
    }

    #[test]
    fn test_bridge_error_status() {
        let err = BridgeError::invalid_argument("test", None::<std::io::Error>);
        assert_eq!(err.status(), Status::InvalidArg);

        let err = BridgeError::runtime("test", None::<std::io::Error>);
        assert_eq!(err.status(), Status::GenericFailure);
    }

    #[test]
    fn test_bridge_error_source_chain() {
        // Create an error with a source
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let bridge_err =
            BridgeError::runtime("Failed to read file", Some(io_err));

        // The source should be preserved
        assert!(bridge_err.source().is_some());
        let source_msg = bridge_err.source()
            .expect("Test should have a source error")
            .to_string();
        assert!(source_msg.contains("file not found"));
    }

    #[test]
    fn test_from_app_error() {
        let domain_err = DomainError::not_found("document 'doc1'");
        let app_err: AppError = domain_err.into();
        let bridge_err: BridgeError = app_err.into();

        // After the fix, DomainError is mapped to InvalidArgument
        assert_eq!(
            bridge_err.to_string(),
            "Invalid argument: resource not found: document 'doc1'"
        );
    }

    #[test]
    fn test_from_infra_error() {
        let infra_err = InfraError::database("connection failed", None::<std::io::Error>);
        let bridge_err: BridgeError = infra_err.into();

        // After the fix, InfraError::Database is mapped to Runtime
        assert_eq!(
            bridge_err.to_string(),
            "Runtime error: Database error: connection failed"
        );
    }

    #[test]
    fn test_to_napi_error() {
        let bridge_err = BridgeError::invalid_argument("limit must be > 0", None::<std::io::Error>);
        let napi_err: NapiError = bridge_err.into();

        assert_eq!(napi_err.status, Status::InvalidArg);
        assert!(napi_err.to_string().contains("Invalid argument"));
        assert!(napi_err.to_string().contains("limit must be > 0"));
    }

    #[test]
    fn test_app_error_to_napi_error() {
        let domain_err = DomainError::invalid_query("query text is empty");
        let app_err: AppError = domain_err.into();
        let napi_err: NapiError = app_err.into();

        // After the fix, DomainError maps to InvalidArgument which has InvalidArg status
        assert_eq!(napi_err.status, Status::InvalidArg);
        assert!(napi_err.to_string().contains("Invalid argument"));
        assert!(napi_err.to_string().contains("query text is empty"));
    }

    #[test]
    fn test_error_chain_preserved() {
        // Create a chain: std::io::Error -> InfraError -> AppError -> BridgeError -> NapiError

        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let infra_err = InfraError::io("/path/to/file", "failed to read", Some(io_err));
        let app_err: AppError = infra_err.into();
        let napi_err: NapiError = app_err.into();

        // The N-API error should contain all the context
        let err_msg = napi_err.to_string();
        assert!(err_msg.contains("Runtime error") || err_msg.contains("IO error"));
        assert!(err_msg.contains("/path/to/file"));
        assert!(err_msg.contains("failed to read"));
    }

    #[test]
    fn test_serialization_error() {
        // Create a test IO error to wrap
        let io_err = std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid JSON");
        let bridge_err = BridgeError::serialization("Failed to parse query", Some(io_err));

        assert_eq!(
            bridge_err.to_string(),
            "Serialization error: Failed to parse query"
        );
        assert_eq!(bridge_err.status(), Status::InvalidArg);
        assert!(bridge_err.source().is_some());
    }
}
