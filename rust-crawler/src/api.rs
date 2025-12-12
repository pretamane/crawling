use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;
use crate::crawler;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

#[derive(Deserialize)]
pub struct CrawlRequest {
    pub keyword: String,
    pub engine: Option<String>, 
}

#[derive(Serialize)]
pub struct CrawlResponse {
    pub task_id: String,
    pub message: String,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct TaskResult {
    pub id: String,
    pub keyword: String,
    pub engine: String,
    pub status: String,
    pub results_json: Option<String>,
    pub extracted_text: Option<String>,
    pub first_page_html: Option<String>,
    pub meta_description: Option<String>,
    pub meta_author: Option<String>,
    pub meta_date: Option<String>,
}

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
            Ok(results) => {
                // 2. Extract content from the first result (Deep Crawl)
                let extracted_content = if let Some(first_result) = results.first() {
                    crawler::extract_content(&first_result.link).await.unwrap_or_default()
                } else {
                    crawler::ExtractedContent::default()
                };


                let results_json = serde_json::to_string(&results).unwrap_or_default();

                // 3. Save to Disk (User Requirement)
                let storage_path = "/home/guest/tzdump/crawl-results";
                if let Err(e) = std::fs::create_dir_all(storage_path) {
                    eprintln!("Failed to create storage dir: {}", e);
                }

                let safe_keyword = keyword.replace(" ", "_").replace("/", "-");
                let filename_base = format!("{}/{}_{}_{}", storage_path, safe_keyword, engine, task_id_clone);

                // Decode Bing/Google redirect URLs to get actual website list
                let websites: Vec<String> = results.iter().map(|r| {
                    crawler::decode_search_url(&r.link)
                }).collect();

                // Create structured response with keyword (business requirement)
                let structured_response = serde_json::json!({
                    "keyword": &keyword,
                    "engine": &engine,
                    "websites": &websites,
                    "results": &results,
                    "first_page_html_file": format!("{}.html", filename_base),
                    "results_count": results.len()
                });

                // Save JSON (pretty-printed for readability)
                let results_json_pretty = serde_json::to_string_pretty(&structured_response).unwrap_or_else(|_| results_json.clone());
                if let Err(e) = std::fs::write(format!("{}.json", filename_base), &results_json_pretty) {
                    eprintln!("Failed to write JSON: {}", e);
                }

                // Save HTML
                if !extracted_content.html.is_empty() {
                    if let Err(e) = std::fs::write(format!("{}.html", filename_base), &extracted_content.html) {
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
                .bind(&extracted_content.text)
                .bind(&extracted_content.html)
                .bind(&extracted_content.meta_description)
                .bind(&extracted_content.meta_author)
                .bind(&extracted_content.meta_date)
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
