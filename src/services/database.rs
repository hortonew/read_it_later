use chrono;
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::{Error, FromRow, PgPool};

pub async fn initialize_pool(postgres_url: &str) -> Result<PgPool, Error> {
    PgPool::connect(postgres_url).await
}

pub async fn check_health(db_pool: &PgPool) -> &'static str {
    match sqlx::query("SELECT 1").execute(db_pool).await {
        Ok(_) => "ok",
        Err(_) => "error",
    }
}

pub async fn create_urls_table(db_pool: &PgPool) -> Result<(), Error> {
    let query = r#"
        CREATE TABLE IF NOT EXISTS urls (
            id SERIAL PRIMARY KEY,
            datetime TIMESTAMP NOT NULL DEFAULT NOW(),
            url TEXT NOT NULL,
            url_hash CHAR(64) NOT NULL UNIQUE
        )
    "#;

    sqlx::query(query).execute(db_pool).await?;
    Ok(())
}

fn calculate_url_hash(url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(url);
    format!("{:x}", hasher.finalize()) // Convert to a hexadecimal string
}

pub async fn insert_url(db_pool: &PgPool, url: &str) -> Result<(), Error> {
    let url_hash = calculate_url_hash(url);
    let query = r#"
        INSERT INTO urls (url, url_hash)
        VALUES ($1, $2)
        ON CONFLICT (url_hash) DO NOTHING
    "#;

    sqlx::query(query).bind(url).bind(url_hash).execute(db_pool).await?;
    Ok(())
}

#[derive(FromRow, Serialize)]
pub struct Url {
    pub id: i32,
    pub datetime: chrono::NaiveDateTime,
    pub url: String,
    pub url_hash: String,
}

pub async fn get_all_urls(db_pool: &PgPool) -> Result<Vec<Url>, Error> {
    let query = r#"
        SELECT id, datetime, url, url_hash
        FROM urls
        ORDER BY datetime DESC
    "#;

    let urls = sqlx::query_as::<_, Url>(query).fetch_all(db_pool).await?;

    Ok(urls)
}

pub async fn delete_url(db_pool: &PgPool, id: i32) -> Result<(), Error> {
    let query = "DELETE FROM urls WHERE id = $1";
    sqlx::query(query).bind(id).execute(db_pool).await?;
    Ok(())
}
