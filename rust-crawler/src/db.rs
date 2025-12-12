use sqlx::{postgres::PgPool, Row};
use anyhow::Result;

pub async fn init_db(pool: &PgPool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tasks (
            id VARCHAR PRIMARY KEY,
            keyword VARCHAR NOT NULL,
            status VARCHAR NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            results_json TEXT,
            extracted_text TEXT,
            meta_description TEXT,
            meta_author VARCHAR,
            meta_date VARCHAR
        );
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}
