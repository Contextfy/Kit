use axum::{
    extract::{Path as ApiPath, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, Router},
};
use contextfy_core::SearchEngine;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: String,
}

#[derive(Debug, Serialize)]
struct SearchResult {
    id: String,
    score: f64,
}

#[derive(Debug, Serialize)]
struct SearchResponse {
    results: Vec<SearchResult>,
}

#[derive(Debug, Serialize)]
struct DocumentResponse {
    id: String,
    title: String,
    content: String,
}

type AppState = Arc<RwLock<SearchEngine>>;

/// API Error type enumeration
///
/// Provides type-safe error categorization with automatic HTTP status code mapping.
/// This avoids fragile string matching and makes error handling more maintainable.
#[derive(Debug, Clone, Copy)]
enum ApiErrorType {
    BadRequest,
    NotFound,
    InternalServerError,
}

impl ApiErrorType {
    /// Get the HTTP status code for this error type
    fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Get the error name for serialization
    fn as_str(&self) -> &'static str {
        match self {
            Self::BadRequest => "Bad Request",
            Self::NotFound => "Not Found",
            Self::InternalServerError => "Internal Server Error",
        }
    }
}

/// API Error response structure
///
/// Uses enum-driven error typing for robust status code mapping.
/// The `error_type` field ensures correct HTTP status codes regardless
/// of message content changes.
#[derive(Debug, Serialize)]
struct ApiError {
    #[serde(skip)]
    error_type: ApiErrorType,
    error: String,
    message: String,
}

impl ApiError {
    /// Create a new API error with the specified type and message
    fn new(error_type: ApiErrorType, message: impl Into<String>) -> Self {
        Self {
            error_type,
            error: error_type.as_str().to_string(),
            message: message.into(),
        }
    }

    /// Convenience method for bad request errors
    fn bad_request(message: impl Into<String>) -> Self {
        Self::new(ApiErrorType::BadRequest, message)
    }

    /// Convenience method for not found errors
    fn not_found(message: impl Into<String>) -> Self {
        Self::new(ApiErrorType::NotFound, message)
    }

    /// Convenience method for internal server errors
    fn internal(message: impl Into<String>) -> Self {
        Self::new(ApiErrorType::InternalServerError, message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.error_type.status_code();
        (status, Json(self)).into_response()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_init()?;

    // Initialize SearchEngine with default paths
    let engine = SearchEngine::new(
        Some(std::path::Path::new(".contextfy/data/bm25_index")),
        ".contextfy/data/lancedb",
        "knowledge",
    )
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "Failed to initialize search engine");
        anyhow::anyhow!("SearchEngine initialization failed: {}", e)
    })?;

    let app_state = Arc::new(RwLock::new(engine));

    let app = Router::new()
        .route("/api/search", get(search_handler))
        .route("/api/document/:id", get(document_handler))
        .nest_service("/", ServeDir::new("packages/web/static"))
        .route("/health", get(health_handler))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to bind to address");
            anyhow::anyhow!("Failed to bind to address: {}", e)
        })?;

    tracing::info!("Server listening on http://127.0.0.1:3000");
    tracing::info!("Web UI available at http://127.0.0.1:3000/");

    axum::serve(listener, app).await.map_err(|e| {
        tracing::error!(error = ?e, "Server error");
        anyhow::anyhow!("Server error: {}", e)
    })?;

    Ok(())
}

fn tracing_init() -> anyhow::Result<()> {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "contextfy_server=debug,tower_http=debug,axum=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .try_init()
        .map_err(|e| anyhow::anyhow!("Failed to initialize tracing: {}", e))?;

    Ok(())
}

async fn health_handler() -> &'static str {
    "Contextfy Server - OK"
}

async fn search_handler(
    State(engine): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<SearchResponse>, ApiError> {
    // Validate query parameter
    let query_text = params.q.trim();
    if query_text.is_empty() {
        tracing::warn!("Received empty search query");
        return Err(ApiError::bad_request("Search query cannot be empty"));
    }

    tracing::info!(query_length = query_text.len(), "Search request received");

    let engine_guard = engine.read().await;

    match engine_guard.search(query_text, 10).await {
        Ok(hits) => {
            tracing::info!(results_count = hits.len(), "Search completed successfully");
            let search_results = hits
                .into_iter()
                .map(|hit| SearchResult {
                    id: hit.id,
                    score: hit.score.value(),
                })
                .collect();

            Ok(Json(SearchResponse {
                results: search_results,
            }))
        }
        Err(e) => {
            tracing::error!(error = ?e, query_length = query_text.len(), "Search failed");
            Err(ApiError::internal(
                "Failed to process search request due to an internal error",
            ))
        }
    }
}

async fn document_handler(
    State(engine): State<AppState>,
    ApiPath(doc_id): ApiPath<String>,
) -> Result<Json<DocumentResponse>, ApiError> {
    tracing::info!(doc_id = %doc_id, "Document request received");

    let engine_guard = engine.read().await;

    match engine_guard.get_document(&doc_id).await {
        Ok(Some(doc)) => {
            tracing::info!(doc_id = %doc_id, "Document retrieved successfully");
            Ok(Json(DocumentResponse {
                id: doc.id,
                title: doc.title,
                content: doc.content.unwrap_or_default(),
            }))
        }
        Ok(None) => {
            tracing::warn!(doc_id = %doc_id, "Document not found");
            Err(ApiError::not_found(format!(
                "Document with ID '{}' was not found",
                doc_id
            )))
        }
        Err(e) => {
            tracing::error!(error = ?e, doc_id = %doc_id, "Failed to retrieve document");
            Err(ApiError::internal(
                "Failed to retrieve document due to an internal error",
            ))
        }
    }
}
