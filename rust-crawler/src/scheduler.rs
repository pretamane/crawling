use tokio_cron_scheduler::{Job, JobScheduler};
use std::sync::Arc;
use crate::api::AppState;

pub async fn start_scheduler(state: Arc<AppState>) -> anyhow::Result<()> {
    let sched = JobScheduler::new().await?;

    // 1. Heartbeat Job (Every 5 minutes)
    // Proves the scheduler is alive and logging to stdout
    sched.add(
        Job::new_async("0 */5 * * * *", |_uuid, _l| {
            Box::pin(async move {
                println!("⏰ [Scheduler] Heartbeat: Central Control System active.");
            })
        })?
    ).await?;

    // 2. Example: Daily "Heavy" Crawl Trigger (At Midnight)
    // This demonstrates pushing a job to the Redis queue automatically
    let state_clone = state.clone();
    sched.add(
        Job::new_async("0 0 0 * * *", move |_uuid, _l| {
            let state = state_clone.clone();
            Box::pin(async move {
                println!("⏰ [Scheduler] Triggering Daily Crawl Batch...");
                
                // Example: Trigger a crawl for "Rust Programming" daily
                let job = crate::queue::CrawlJob {
                    id: uuid::Uuid::new_v4().to_string(),
                    keyword: "daily trend analysis".to_string(),
                    engine: "bing".to_string(),
                    selectors: None,
                };

                match state.queue.push_job(job).await {
                    Ok(_) => println!("✅ [Scheduler] Daily job queued successfully."),
                    Err(e) => eprintln!("❌ [Scheduler] Failed to queue daily job: {}", e),
                }
            })
        })?
    ).await?;

    // Start the scheduler
    sched.start().await?;
    println!("✅ Central Scheduler Started (Rust Native)");

    Ok(())
}
