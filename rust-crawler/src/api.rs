use axum::{
    extract::{Path, State},
    Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;
use crate::crawler;
use utoipa::{ToSchema, OpenApi};
use chrono::NaiveDateTime;
use crate::proxy::{PROXY_MANAGER, ProxyInfo, ProxyStats};
use crate::storage::StorageManager;
use crate::queue::QueueManager;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub storage: StorageManager,
    pub queue: QueueManager,
}

#[derive(Deserialize, ToSchema)]
pub struct CrawlRequest {
    #[schema(example = "rust programming")]
    pub keyword: String,
    #[schema(example = "bing", default = "bing")]
    pub engine: Option<String>,
    #[schema(example = "{\"title\": \"h1\", \"content\": \".post-body\"}")]
    pub selectors: Option<std::collections::HashMap<String, String>>, 
}

#[derive(Serialize, ToSchema)]
pub struct CrawlResponse {
    #[schema(example = "d31d37a9-b82d-415c-9b57-b266287c37b4")]
    pub task_id: String,
    #[schema(example = "Crawl started")]
    pub message: String,
}

#[derive(Serialize, sqlx::FromRow, ToSchema)]
pub struct TaskResult {
    #[schema(example = "d31d37a9-b82d-415c-9b57-b266287c37b4")]
    pub id: String,
    #[schema(example = "rust programming")]
    pub keyword: String,
    #[schema(example = "bing")]
    pub engine: String,
    #[schema(example = "completed")]
    pub status: String,
    pub results_json: Option<String>,
    pub extracted_text: Option<String>,
    pub first_page_html: Option<String>,
    pub meta_description: Option<String>,
    pub meta_author: Option<String>,
    pub meta_date: Option<String>,
}

#[derive(Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct TaskSummary {
    pub id: String,
    pub keyword: String,
    pub engine: String,
    pub status: String,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub results_json: Option<String>,
    pub extracted_text: Option<String>,
}

#[utoipa::path(
    post,
    path = "/crawl",
    request_body = CrawlRequest,
    responses(
        (status = 200, description = "Crawl started successfully", body = CrawlResponse)
    )
)]
pub async fn trigger_crawl(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CrawlRequest>,
) -> Json<CrawlResponse> {
    let task_id = Uuid::new_v4().to_string();
    let keyword = payload.keyword.clone();
    let engine = payload.engine.unwrap_or_else(|| "bing".to_string());

    let job = crate::queue::CrawlJob {
        id: task_id.clone(),
        keyword,
        engine,
        selectors: payload.selectors,
    };

    // Push to Redis Queue
    match state.queue.push_job(job).await {
        Ok(_) => {
            println!("✅ [API] Job pushed to queue: {}", task_id);
            Json(CrawlResponse {
                task_id,
                message: "Crawl job queued successfully".to_string(),
            })
        },
        Err(e) => {
            eprintln!("❌ [API] Failed to queue job: {}", e);
            Json(CrawlResponse {
                task_id,
                message: "Failed to queue job".to_string(),
            })
        }
    }
}

#[utoipa::path(
    get,
    path = "/crawl/{task_id}",
    params(
        ("task_id" = String, Path, description = "Task ID")
    ),
    responses(
        (status = 200, description = "Crawl status/results", body = Option<TaskResult>)
    )
)]


pub async fn get_crawl_status(
// ... existing code ...
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
) -> Json<Option<TaskResult>> {
    let rec = sqlx::query_as::<_, TaskResult>(
        "SELECT id, keyword, engine, status, results_json, extracted_text, first_page_html, meta_description, meta_author, meta_date FROM tasks WHERE id = $1"
    )
    .bind(task_id)
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    Json(rec)
}

#[utoipa::path(
    get,
    path = "/tasks",
    tag = "crawler",
    responses(
        (status = 200, description = "List recent tasks", body = Vec<TaskSummary>)
    )
)]
pub async fn list_tasks(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<TaskSummary>>, (StatusCode, String)> {
    let tasks = sqlx::query_as::<sqlx::Postgres, TaskSummary>(
        "SELECT id, keyword, engine, status, created_at, results_json, left(extracted_text, 1000) as extracted_text FROM tasks ORDER BY created_at DESC LIMIT 50"
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(tasks))
}

// ============================================================================
// Proxy Management API
// ============================================================================

// Imports moved to top

// ============================================================================
// Proxy Management API
// ============================================================================

/// List all proxies with their health status
#[utoipa::path(
    get,
    path = "/proxies",
    tag = "proxy",
    responses(
        (status = 200, description = "List all proxies", body = Vec<ProxyInfo>)
    )
)]
pub async fn list_proxies() -> Json<Vec<ProxyInfo>> {
    Json(PROXY_MANAGER.list_proxies())
}

/// Add a new proxy at runtime
#[derive(Deserialize, ToSchema)]
pub struct AddProxyRequest {
    /// Proxy string: host:port or user:pass@host:port
    #[schema(example = "user:pass@1.2.3.4:8080")]
    pub proxy: String,
}

#[derive(Serialize, ToSchema)]
pub struct AddProxyResponse {
    pub success: bool,
    pub proxy: Option<ProxyInfo>,
    pub error: Option<String>,
}

#[utoipa::path(
    post,
    path = "/proxies",
    tag = "proxy",
    request_body = AddProxyRequest,
    responses(
        (status = 200, description = "Add a new proxy", body = AddProxyResponse)
    )
)]
pub async fn add_proxy(
    Json(payload): Json<AddProxyRequest>,
) -> Json<AddProxyResponse> {
    match PROXY_MANAGER.add_proxy(&payload.proxy) {
        Ok(info) => Json(AddProxyResponse {
            success: true,
            proxy: Some(info),
            error: None,
        }),
        Err(e) => Json(AddProxyResponse {
            success: false,
            proxy: None,
            error: Some(e),
        }),
    }
}

/// Remove a proxy by ID
#[derive(Serialize, ToSchema)]
pub struct RemoveProxyResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[utoipa::path(
    delete,
    path = "/proxies/{proxy_id}",
    tag = "proxy",
    params(
        ("proxy_id" = String, Path, description = "Proxy ID (e.g., host:port)")
    ),
    responses(
        (status = 200, description = "Remove a proxy", body = RemoveProxyResponse)
    )
)]
pub async fn remove_proxy(
    Path(proxy_id): Path<String>,
) -> Json<RemoveProxyResponse> {
    match PROXY_MANAGER.remove_proxy(&proxy_id) {
        Ok(()) => Json(RemoveProxyResponse {
            success: true,
            error: None,
        }),
        Err(e) => Json(RemoveProxyResponse {
            success: false,
            error: Some(e),
        }),
    }
}

/// Re-enable a disabled proxy
#[utoipa::path(
    post,
    path = "/proxies/{proxy_id}/enable",
    tag = "proxy",
    params(
        ("proxy_id" = String, Path, description = "Proxy ID")
    ),
    responses(
        (status = 200, description = "Re-enable a proxy", body = RemoveProxyResponse)
    )
)]
pub async fn enable_proxy(
    Path(proxy_id): Path<String>,
) -> Json<RemoveProxyResponse> {
    match PROXY_MANAGER.enable_proxy(&proxy_id) {
        Ok(()) => Json(RemoveProxyResponse {
            success: true,
            error: None,
        }),
        Err(e) => Json(RemoveProxyResponse {
            success: false,
            error: Some(e),
        }),
    }
}

/// Get aggregate proxy stats
#[utoipa::path(
    get,
    path = "/proxies/stats",
    tag = "proxy",
    responses(
        (status = 200, description = "Get proxy statistics", body = ProxyStats)
    )
)]
pub async fn proxy_stats() -> Json<ProxyStats> {
    Json(PROXY_MANAGER.get_stats())
}
