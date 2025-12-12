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
    pub use_chrome: Option<bool>,
}

#[derive(Serialize)]
pub struct CrawlResponse {
    pub task_id: String,
    pub message: String,
}

pub async fn trigger_crawl(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CrawlRequest>,
) -> Json<CrawlResponse> {
    let task_id = Uuid::new_v4().to_string();
    let pool = state.pool.clone();
    let keyword = payload.keyword.clone();
    let use_chrome = payload.use_chrome.unwrap_or(false);

    let task_id_clone = task_id.clone();
    tokio::spawn(async move {
        // Placeholder for actual search logic
        // For now, let's just pretend we found a URL to crawl
        let url = format!("https://www.bing.com/search?q={}", keyword);
        
        let content_result = if use_chrome {
            crawler::crawl_with_chrome(&url).await
        } else {
            crawler::crawl_fast(&url).await
        };

        match content_result {
            Ok(content) => {
                let _ = sqlx::query(
                    "INSERT INTO tasks (id, keyword, status, extracted_text) VALUES ($1, $2, 'completed', $3)"
                )
                .bind(&task_id_clone)
                .bind(&keyword)
                .bind(&content)
                .execute(&pool)
                .await;
            }
            Err(e) => {
                eprintln!("Crawl failed: {}", e);
            }
        }
    });

    Json(CrawlResponse {
        task_id,
        message: "Crawl started".to_string(),
    })
}
