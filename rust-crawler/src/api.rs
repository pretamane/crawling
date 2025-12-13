use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;
use crate::crawler;
use utoipa::{ToSchema, OpenApi};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

#[derive(Deserialize, ToSchema)]
pub struct CrawlRequest {
    #[schema(example = "rust programming")]
    pub keyword: String,
    #[schema(example = "bing", default = "bing")]
    pub engine: Option<String>, 
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
    let pool = state.pool.clone();
    let keyword = payload.keyword.clone();
    let engine = payload.engine.unwrap_or_else(|| "bing".to_string());
    let engine_clone = engine.clone();

    let task_id_clone = task_id.clone();
    tokio::spawn(async move {
        // 1. Search
        let search_results = if engine_clone == "google" {
            crawler::search_google(&keyword).await
        } else {
            crawler::search_bing(&keyword).await
        };

        match search_results {
            Ok(serp_data) => {
                // 2. Extract content from the first result (Deep Crawl)
                let first_result_data = if let Some(first_result) = serp_data.results.first() {
                    crawler::extract_website_data(&first_result.link).await.ok()
                } else {
                    None
                };

                let results_json = serde_json::to_string(&serp_data).unwrap_or_default();

                // 3. Save to Disk (User Requirement)
                let storage_path = std::env::var("STORAGE_PATH").unwrap_or_else(|_| "crawl-results".to_string());
                if let Err(e) = std::fs::create_dir_all(&storage_path) {
                    eprintln!("Failed to create storage dir: {}", e);
                }

                let safe_keyword = keyword.replace(" ", "_").replace("/", "-");
                let filename_base = format!("{}/{}_{}_{}", storage_path, safe_keyword, engine, task_id_clone);

                // Decode Bing/Google redirect URLs to get actual website list
                let websites: Vec<String> = serp_data.results.iter().map(|r| {
                    crawler::decode_search_url(&r.link)
                }).collect();

                // Create structured response with keyword (business requirement)
                let structured_response = serde_json::json!({
                    "keyword": &keyword,
                    "engine": &engine,
                    "websites": &websites,
                    "serp_data": &serp_data,
                    "first_result_data": &first_result_data,
                    "results_count": serp_data.results.len()
                });

                // Save JSON (pretty-printed for readability)
                let results_json_pretty = serde_json::to_string_pretty(&structured_response).unwrap_or_else(|_| results_json.clone());
                if let Err(e) = std::fs::write(format!("{}.json", filename_base), &results_json_pretty) {
                    eprintln!("Failed to write JSON: {}", e);
                }

                // Prepare data for DB
                let (extracted_text, extracted_html, md, ma, mdate) = if let Some(data) = &first_result_data {
                    (
                        data.main_text.clone(),
                        data.html.clone(),
                        data.meta_description.clone(),
                        data.meta_author.clone(),
                        data.meta_date.clone()
                    )
                } else {
                    (String::new(), String::new(), None, None, None)
                };

                // Save HTML
                if !extracted_html.is_empty() {
                    if let Err(e) = std::fs::write(format!("{}.html", filename_base), &extracted_html) {
                        eprintln!("Failed to write HTML: {}", e);
                    }
                }

                // 4. Save to DB
                let _ = sqlx::query(
                    "INSERT INTO tasks (id, keyword, engine, status, results_json, extracted_text, first_page_html, meta_description, meta_author, meta_date) VALUES ($1, $2, $3, 'completed', $4, $5, $6, $7, $8, $9)"
                )
                .bind(&task_id_clone)
                .bind(&keyword)
                .bind(&engine_clone)
                .bind(&results_json)
                .bind(&extracted_text)
                .bind(&extracted_html)
                .bind(&md)
                .bind(&ma)
                .bind(&mdate)
                .execute(&pool)
                .await;
            }
            Err(e) => {
                eprintln!("Crawl failed: {}", e);
                let _ = std::fs::write("crawl_errors.log", format!("Error: {}\n", e));
            }
        }
    });

    Json(CrawlResponse {
        task_id,
        message: "Crawl started".to_string(),
    })
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

// ============================================================================
// Proxy Management API
// ============================================================================

use crate::proxy::{PROXY_MANAGER, ProxyInfo, ProxyStats};

/// List all proxies with their health status
pub async fn list_proxies() -> Json<Vec<ProxyInfo>> {
    Json(PROXY_MANAGER.list_proxies())
}

/// Add a new proxy at runtime
#[derive(Deserialize)]
pub struct AddProxyRequest {
    /// Proxy string: host:port or user:pass@host:port
    pub proxy: String,
}

#[derive(Serialize)]
pub struct AddProxyResponse {
    pub success: bool,
    pub proxy: Option<ProxyInfo>,
    pub error: Option<String>,
}

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
#[derive(Serialize)]
pub struct RemoveProxyResponse {
    pub success: bool,
    pub error: Option<String>,
}

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
pub async fn proxy_stats() -> Json<ProxyStats> {
    Json(PROXY_MANAGER.get_stats())
}
