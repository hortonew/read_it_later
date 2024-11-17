use chrono;
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::{Error, FromRow, PgPool, Row};

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

pub async fn create_tags_table(db_pool: &PgPool) -> Result<(), Error> {
    let query = r#"
        CREATE TABLE IF NOT EXISTS tags (
            id SERIAL PRIMARY KEY,
            url_id INTEGER NOT NULL REFERENCES urls(id) ON DELETE CASCADE,
            tag TEXT NOT NULL
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

pub async fn insert_tags(db_pool: &PgPool, url: &str, tags: &str) -> Result<(), Error> {
    let url_hash = calculate_url_hash(url);
    let url_id_query = r#"
        SELECT id FROM urls WHERE url_hash = $1
    "#;

    let url_id: i32 = sqlx::query_scalar(url_id_query)
        .bind(url_hash)
        .fetch_one(db_pool)
        .await?;

    let tags: Vec<&str> = tags.split(',').map(|tag| tag.trim()).collect();
    for tag in tags {
        let query = r#"
            INSERT INTO tags (url_id, tag)
            VALUES ($1, $2)
        "#;
        sqlx::query(query).bind(url_id).bind(tag).execute(db_pool).await?;
    }
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

pub async fn get_urls_with_tags(db_pool: &PgPool) -> Result<Vec<(Url, Vec<String>)>, Error> {
    let query = r#"
        SELECT urls.id,
               urls.datetime,
               urls.url,
               urls.url_hash,
               COALESCE(array_remove(array_agg(tags.tag), NULL), ARRAY[]::TEXT[]) AS tags
        FROM urls
        LEFT JOIN tags ON urls.id = tags.url_id
        GROUP BY urls.id
        ORDER BY urls.datetime DESC
    "#;

    let rows = sqlx::query(query).fetch_all(db_pool).await?;
    let mut urls_with_tags = Vec::new();

    for row in rows {
        let url = Url {
            id: row.get("id"),
            datetime: row.get("datetime"),
            url: row.get("url"),
            url_hash: row.get("url_hash"),
        };
        // Decode tags safely
        let tags: Vec<String> = row.try_get("tags").unwrap_or_default();
        urls_with_tags.push((url, tags));
    }

    Ok(urls_with_tags)
}

pub async fn delete_url(db_pool: &PgPool, id: i32) -> Result<(), Error> {
    let query = "DELETE FROM urls WHERE id = $1";
    sqlx::query(query).bind(id).execute(db_pool).await?;
    Ok(())
}

pub async fn delete_url_by_url(db_pool: &PgPool, url: &str) -> Result<(), Error> {
    let url_hash = calculate_url_hash(url);
    let query = "DELETE FROM urls WHERE url_hash = $1";
    sqlx::query(query).bind(url_hash).execute(db_pool).await?;
    Ok(())
}
