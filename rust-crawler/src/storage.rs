use aws_sdk_s3::{Client, config::Region};
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::primitives::ByteStream;
use anyhow::Result;
use std::env;

#[derive(Clone)]
pub struct StorageManager {
    client: Client,
    bucket: String,
}

impl StorageManager {
    pub async fn new() -> Result<Self> {
        let endpoint = env::var("MINIO_ENDPOINT").unwrap_or_else(|_| "http://localhost:9000".to_string());
        let access_key = env::var("MINIO_ROOT_USER").unwrap_or_else(|_| "minio_user".to_string());
        let secret_key = env::var("MINIO_ROOT_PASSWORD").unwrap_or_else(|_| "minio_password".to_string());
        let bucket = env::var("MINIO_BUCKET").unwrap_or_else(|_| "crawler-data".to_string());

        let region_provider = RegionProviderChain::default_provider().or_else(Region::new("us-east-1"));
        let config = aws_config::from_env()
            .region(region_provider)
            .endpoint_url(&endpoint)
            .credentials_provider(Credentials::new(
                access_key,
                secret_key,
                None,
                None,
                "static",
            ))
            .load()
            .await;

        let client_config = aws_sdk_s3::config::Builder::from(&config)
            .force_path_style(true)
            .build();
        let client = Client::from_conf(client_config);

        // Robust Retry Loop for Bucket Initialization
        let mut attempts = 0;
        loop {
            match client.head_bucket().bucket(&bucket).send().await {
                Ok(_) => {
                    println!("✅ MinIO Bucket '{}' exists", bucket);
                    break;
                },
                Err(e) => {
                    // Check if error is "NotFound" (404) or something else (DNS, Conn)
                    let is_not_found = e.into_service_error().is_not_found();
                    
                    if is_not_found {
                        println!("⚠️ MinIO Bucket '{}' not found, creating...", bucket);
                        match client.create_bucket().bucket(&bucket).send().await {
                            Ok(_) => {
                                println!("✅ Created bucket '{}'", bucket);
                                break; 
                            },
                            Err(create_err) => {
                                eprintln!("🔥 Failed to create bucket: {}", create_err);
                                // Don't break, retry loop (might be transient)
                            }
                        }
                    } else {
                        // DNS/Connection Error
                        attempts += 1;
                        if attempts >= 30 {
                            return Err(anyhow::anyhow!("Failed to connect to MinIO after 30 attempts"));
                        }
                        println!("⚠️ MinIO Connect failed (Attempt {}/30). Retrying in 2s...", attempts);
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    }
                }
            }
        }

        Ok(Self { client, bucket })
    }

    pub async fn store_html(&self, key: &str, content: &str) -> Result<()> {
        let body = ByteStream::from(content.as_bytes().to_vec());
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body)
            .content_type("text/html")
            .send()
            .await?;
        Ok(())
    }
}
