use redis::{Client, AsyncCommands};
use anyhow::Result;
use std::env;

#[derive(Clone)]
pub struct QueueManager {
    client: Client,
}

use serde::{Deserialize, Serialize};
use crate::api::CrawlRequest;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CrawlJob {
    pub id: String,
    pub keyword: String,
    pub engine: String,
    pub selectors: Option<std::collections::HashMap<String, String>>,
}

impl QueueManager {
    pub async fn new() -> Result<Self> {
        let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
        let client = Client::open(redis_url)?;
        
        // Test connection
        let mut conn = client.get_async_connection().await?;
        let _: String = redis::cmd("PING").query_async(&mut conn).await?;
        println!("✅ Redis Connected successfully");

        Ok(Self { client })
    }

    pub async fn push_job(&self, job: CrawlJob) -> Result<()> {
        let mut conn = self.client.get_async_connection().await?;
        let job_json = serde_json::to_string(&job)?;
        conn.lpush::<_, _, ()>("crawl_queue", job_json).await?;
        Ok(())
    }

    pub async fn pop_job(&self) -> Result<Option<CrawlJob>> {
        let mut conn = self.client.get_async_connection().await?;
        let result: Option<String> = conn.rpop("crawl_queue", None).await?;
        
        match result {
            Some(json) => {
                let job: CrawlJob = serde_json::from_str(&json)?;
                Ok(Some(job))
            }
            None => Ok(None)
        }
    }
}
