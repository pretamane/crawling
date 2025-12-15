mod api;
mod crawler;
mod db;
mod proxy;
mod storage;
mod queue;
mod worker;
mod scheduler;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use dotenv::dotenv;
use std::env;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use tower_http::services::ServeDir;

#[derive(OpenApi)]
#[openapi(
    paths(
        api::trigger_crawl,
        api::get_crawl_status,
        api::list_tasks,
        api::list_proxies,
        api::add_proxy,
        api::remove_proxy,
        api::enable_proxy,
        api::proxy_stats
    ),
    components(
        schemas(
            api::CrawlRequest, 
            api::CrawlResponse, 
            api::TaskResult, 
            api::TaskSummary,
            api::AddProxyRequest,
            api::AddProxyResponse,
            api::RemoveProxyResponse,
            crate::proxy::ProxyInfo,
            crate::proxy::ProxyStats,
            crate::proxy::ProxyProtocol
        )
    ),
    tags(
        (name = "crawler", description = "Crawler Management API"),
        (name = "proxy", description = "Proxy Management API")
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    
    // Robust Connection Retry Loop
    println!("🔌 Connecting to Database...");
    let pool = {
        let mut attempts = 0;
        loop {
            match PgPoolOptions::new()
                .max_connections(5)
                .connect(&db_url)
                .await 
            {
                Ok(p) => {
                    println!("✅ Database Connected!");
                    break p;
                },
                Err(e) => {
                    attempts += 1;
                    if attempts >= 15 {
                        eprintln!("🔥 CRITICAL: Failed to connect to DB after 15 attempts.");
                        return Err(e.into());
                    }
                    println!("⚠️ DB Connect failed ({}), retrying in 2s... (Attempt {}/15)", e, attempts);
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
            }
        }
    };

    db::init_db(&pool).await?;

    let storage = storage::StorageManager::new().await.expect("Failed to init MinIO");
    let queue = queue::QueueManager::new().await.expect("Failed to init Redis");

    let state = Arc::new(api::AppState { pool, storage, queue });

    // Start Background Worker
    let worker_state = state.clone();
    tokio::spawn(async move {
        worker::start_worker(worker_state).await;
    });

    // Start Central Scheduler (Rust)
    let scheduler_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = scheduler::start_scheduler(scheduler_state).await {
            eprintln!("🔥 Scheduler Error: {}", e);
        }
    });

    let app = Router::new()
        .merge(SwaggerUi::new("/rust-crawler-swagger").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/crawl", post(api::trigger_crawl))
        .route("/crawl/:task_id", get(api::get_crawl_status))
        .route("/tasks", get(api::list_tasks))
        // Proxy management endpoints
        .route("/proxies", get(api::list_proxies))
        .route("/proxies", post(api::add_proxy))
        .route("/proxies/:proxy_id", axum::routing::delete(api::remove_proxy))
        .route("/proxies/:proxy_id/enable", post(api::enable_proxy))
        .route("/proxies/stats", get(api::proxy_stats))
        .nest_service("/", ServeDir::new("static")) // Serve Dashboard
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
