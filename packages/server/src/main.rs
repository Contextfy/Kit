use axum::{
    extract::{Path as ApiPath, Query, State},
    response::Json,
    routing::{get, Router},
};
use contextfy_core::{KnowledgeStore, Retriever};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: String,
}

#[derive(Debug, Serialize)]
struct SearchResult {
    id: String,
    title: String,
    summary: String,
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

type AppState = Arc<Mutex<KnowledgeStore>>;

#[tokio::main]
async fn main() {
    let store = Arc::new(Mutex::new(
        KnowledgeStore::new(".contextfy/data").await.unwrap(),
    ));

    let app = Router::new()
        .route("/api/search", get(search_handler))
        .route("/api/document/:id", get(document_handler))
        .nest_service("/", ServeDir::new("packages/web/static"))
        .route("/health", get(health_handler))
        .with_state(store);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("Server listening on http://127.0.0.1:3000");
    println!("Web UI available at http://127.0.0.1:3000/");
    let _ = axum::serve(listener, app).await;
}

async fn health_handler() -> &'static str {
    "Contextfy Server - OK"
}

async fn search_handler(
    State(store): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Json<SearchResponse> {
    println!("Search query: {}", params.q);

    let guard = store.lock().await;
    let retriever = Retriever::new(&*guard);

    let results = retriever.scout(&params.q).await;

    match results {
        Ok(r) => {
            let search_results = r
                .into_iter()
                .map(|res| SearchResult {
                    id: res.id,
                    title: res.title,
                    summary: res.summary,
                })
                .collect();

            Json(SearchResponse {
                results: search_results,
            })
        }
        Err(e) => {
            eprintln!("Search error: {}", e);
            Json(SearchResponse { results: vec![] })
        }
    }
}

async fn document_handler(
    State(store): State<AppState>,
    ApiPath(doc_id): ApiPath<String>,
) -> Json<DocumentResponse> {
    println!("Fetching document: {}", doc_id);

    let guard = store.lock().await;
    let retriever = Retriever::new(&*guard);

    let result = retriever.inspect(&doc_id).await;

    match result {
        Ok(Some(details)) => Json(DocumentResponse {
            id: details.id,
            title: details.title,
            content: details.content,
        }),
        Ok(None) => Json(DocumentResponse {
            id: doc_id,
            title: "Not Found".to_string(),
            content: "Document not found".to_string(),
        }),
        Err(e) => {
            eprintln!("Error fetching document: {}", e);
            Json(DocumentResponse {
                id: doc_id,
                title: "Error".to_string(),
                content: format!("Failed to fetch document: {}", e),
            })
        }
    }
}
