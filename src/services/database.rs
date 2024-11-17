use chrono;
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::{Error, FromRow, PgPool, Row};

/// Initialize the PostgreSQL connection pool
pub async fn initialize_pool(postgres_url: &str) -> Result<PgPool, Error> {
    PgPool::connect(postgres_url).await
}

/// Check if the database connection is healthy
pub async fn check_health(db_pool: &PgPool) -> &'static str {
    match sqlx::query("SELECT 1").execute(db_pool).await {
        Ok(_) => "ok",
        Err(_) => "error",
    }
}

/// Create the `urls` table
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

/// Create the `tags` table
pub async fn create_tags_table(db_pool: &PgPool) -> Result<(), Error> {
    let query = r#"
        CREATE TABLE IF NOT EXISTS tags (
            id SERIAL PRIMARY KEY,
            tag TEXT NOT NULL UNIQUE
        )
    "#;

    sqlx::query(query).execute(db_pool).await?;

    // Add a unique constraint to `tag` if it doesn't exist (idempotent)
    let constraint_query = r#"
        DO $$
        BEGIN
            IF NOT EXISTS (
                SELECT 1
                FROM information_schema.table_constraints
                WHERE table_name = 'tags'
                  AND constraint_type = 'UNIQUE'
                  AND constraint_name = 'unique_tag'
            ) THEN
                ALTER TABLE tags ADD CONSTRAINT unique_tag UNIQUE (tag);
            END IF;
        END $$;
    "#;

    sqlx::query(constraint_query).execute(db_pool).await?;

    Ok(())
}

/// Create the `url_tags` join table
pub async fn create_url_tags_table(db_pool: &PgPool) -> Result<(), Error> {
    let query = r#"
        CREATE TABLE IF NOT EXISTS url_tags (
            id SERIAL PRIMARY KEY,
            url_id INTEGER NOT NULL REFERENCES urls(id) ON DELETE CASCADE,
            tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            UNIQUE (url_id, tag_id)
        )
    "#;

    sqlx::query(query).execute(db_pool).await?;
    Ok(())
}

/// Hash a URL to create a unique identifier
fn calculate_url_hash(url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(url);
    format!("{:x}", hasher.finalize()) // Convert to a hexadecimal string
}

/// Insert a URL into the database
pub async fn insert_url(db_pool: &PgPool, url: &str) -> Result<i32, Error> {
    let url_hash = calculate_url_hash(url);

    // Try to insert the URL and return its ID. If it already exists, fetch the existing ID.
    let query = r#"
        INSERT INTO urls (url, url_hash)
        VALUES ($1, $2)
        ON CONFLICT (url_hash) DO UPDATE SET url_hash = urls.url_hash
        RETURNING id
    "#;

    let url_id: i32 = sqlx::query_scalar(query)
        .bind(url)
        .bind(url_hash)
        .fetch_one(db_pool)
        .await?;

    Ok(url_id)
}

/// Insert tags into the database and associate them with a URL
pub async fn insert_tags(db_pool: &PgPool, url: &str, tags: &[&str]) -> Result<(), Error> {
    if tags.is_empty() {
        return Ok(()); // Nothing to insert
    }

    // Insert or retrieve the URL ID
    let url_id = insert_url(db_pool, url).await?;

    for tag in tags {
        // Check if the tag already exists or insert it
        let tag_query = r#"
            INSERT INTO tags (tag)
            VALUES ($1)
            ON CONFLICT (tag) DO NOTHING
            RETURNING id
        "#;

        // If the tag already exists, fetch its ID
        let tag_id: i32 = match sqlx::query_scalar(tag_query).bind(tag).fetch_one(db_pool).await {
            Ok(id) => id,
            Err(sqlx::Error::RowNotFound) => {
                // If the tag exists but isn't returned, fetch its ID directly
                sqlx::query_scalar("SELECT id FROM tags WHERE tag = $1")
                    .bind(tag)
                    .fetch_one(db_pool)
                    .await?
            }
            Err(err) => return Err(err),
        };

        // Link the URL and tag in the `url_tags` table
        let url_tag_query = r#"
            INSERT INTO url_tags (url_id, tag_id)
            VALUES ($1, $2)
            ON CONFLICT (url_id, tag_id) DO NOTHING
        "#;

        sqlx::query(url_tag_query)
            .bind(url_id)
            .bind(tag_id)
            .execute(db_pool)
            .await?;
    }

    Ok(())
}

/// Delete a URL by its ID
pub async fn delete_url(db_pool: &PgPool, id: i32) -> Result<(), Error> {
    let query = "DELETE FROM urls WHERE id = $1";
    sqlx::query(query).bind(id).execute(db_pool).await?;
    Ok(())
}

/// Delete a URL by its string value
pub async fn delete_url_by_url(db_pool: &PgPool, url: &str) -> Result<(), Error> {
    let url_hash = calculate_url_hash(url);
    let query = "DELETE FROM urls WHERE url_hash = $1";
    sqlx::query(query).bind(url_hash).execute(db_pool).await?;
    Ok(())
}

/// Struct representing a URL
#[derive(FromRow, Serialize)]
pub struct Url {
    pub id: i32,
    pub datetime: chrono::NaiveDateTime,
    pub url: String,
    pub url_hash: String,
}

/// Fetch all URLs from the database
pub async fn get_all_urls(db_pool: &PgPool) -> Result<Vec<Url>, Error> {
    let query = r#"
        SELECT id, datetime, url, url_hash
        FROM urls
        ORDER BY datetime DESC
    "#;

    let urls = sqlx::query_as::<_, Url>(query).fetch_all(db_pool).await?;

    Ok(urls)
}

/// Fetch all URLs with their associated tags
pub async fn get_urls_with_tags(db_pool: &PgPool) -> Result<Vec<(String, Vec<String>)>, Error> {
    let query = r#"
        SELECT urls.url, COALESCE(ARRAY_AGG(tags.tag), ARRAY[]::TEXT[]) AS tags
        FROM urls
        LEFT JOIN url_tags ON urls.id = url_tags.url_id
        LEFT JOIN tags ON url_tags.tag_id = tags.id
        GROUP BY urls.url
    "#;

    let rows = sqlx::query(query).fetch_all(db_pool).await?;
    let mut results = Vec::new();

    for row in rows {
        let url: String = row.get("url");
        let tags: Vec<String> = row.try_get("tags").unwrap_or_default();
        results.push((url, tags));
    }

    Ok(results)
}
