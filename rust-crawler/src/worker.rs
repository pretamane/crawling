use std::sync::Arc;
use tokio::time::{sleep, Duration};
use crate::api::AppState;
use crate::crawler;
use crate::queue::CrawlJob;

pub async fn start_worker(state: Arc<AppState>) {
    println!("👷 Worker started, polling Redis...");

    loop {
        // Poll for 1 job
        match state.queue.pop_job().await {
            Ok(Some(job)) => {
                println!("👷 [Worker] Picked up job: {} ({})", job.id, job.keyword);
                if let Err(e) = process_job(state.clone(), job).await {
                    eprintln!("❌ [Worker] Job failed: {}", e);
                    // TODO: Implement DLQ or Retry here
                }
            },
            Ok(None) => {
                // Queue empty, sleep backoff
                sleep(Duration::from_millis(1000)).await;
            },
            Err(e) => {
                eprintln!("🔥 [Worker] Redis error: {}", e);
                sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

async fn process_job(state: Arc<AppState>, job: CrawlJob) -> anyhow::Result<()> {
    println!("🚀 [Worker] Processing: {}", job.keyword);
    let pool = state.pool.clone();
    let engine_clone = job.engine.clone();

    // 1. Search (Google/Bing/Generic)
    let search_results = if job.engine == "google" {
        crawler::search_google(&job.keyword).await
    } else if job.engine == "generic" {
        crawler::generic_crawl(&job.keyword, job.selectors).await
    } else {
        crawler::search_bing(&job.keyword).await
    };

    let serp_data = match search_results {
        Ok(data) => data,
        Err(e) => {
             // Log failure to DB?
             return Err(e);
        }
    };

    // 2. Extract Content (Deep Crawl)
    let first_result_data = if let Some(first_result) = serp_data.results.first() {
        println!("🔍 [Worker] Deep extracting: {}", first_result.link);
        crawler::extract_website_data(&first_result.link).await.ok()
    } else {
        None
    };

    let results_json = serde_json::to_string(&serp_data).unwrap_or_default();

    // 3. Save to MinIO (Raw HTML)
    // Example: Store first page HTML if exists
    if let Some(ref data) = first_result_data {
        if !data.html.is_empty() {
            let s3_key = format!("{}/{}.html", job.engine, job.id);
            if let Err(e) = state.storage.store_html(&s3_key, &data.html).await {
                eprintln!("⚠️ [Worker] MinIO upload failed: {}", e);
            } else {
                println!("💾 [Worker] HTML saved to MinIO: {}", s3_key);
            }
        }
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

    // 4. Save to DB
    sqlx::query(
        "INSERT INTO tasks (id, keyword, engine, status, results_json, extracted_text, first_page_html, meta_description, meta_author, meta_date) VALUES ($1, $2, $3, 'completed', $4, $5, $6, $7, $8, $9)"
    )
    .bind(&job.id)
    .bind(&job.keyword)
    .bind(&job.engine)
    .bind(&results_json)
    .bind(&extracted_text)
    .bind(&extracted_html)
    .bind(&md)
    .bind(&ma)
    .bind(&mdate)
    .execute(&pool)
    .await?;

    println!("✅ [Worker] Job {} completed successfully!", job.id);
    Ok(())
}
