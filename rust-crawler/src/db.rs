use sqlx::{postgres::PgPool, Row};
use anyhow::Result;

pub async fn init_db(pool: &PgPool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tasks (
            id VARCHAR PRIMARY KEY,
            keyword VARCHAR NOT NULL,
            engine VARCHAR NOT NULL DEFAULT 'bing',
            status VARCHAR NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            results_json TEXT,
            extracted_text TEXT,
            first_page_html TEXT,
            meta_description TEXT,
            meta_author TEXT,
            meta_date TEXT
        );
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}
