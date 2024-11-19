use chrono;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Error, FromRow, Row, SqlitePool};

/// Initialize the SQLite connection pool
pub async fn initialize_pool(sqlite_url: &str) -> Result<SqlitePool, Error> {
    SqlitePool::connect(sqlite_url).await
}

/// Check if the database connection is healthy
pub async fn check_health(db_pool: &SqlitePool) -> &'static str {
    match sqlx::query("SELECT 1").execute(db_pool).await {
        Ok(_) => "ok",
        Err(_) => "error",
    }
}

/// Create the `urls` table
pub async fn create_urls_table(db_pool: &SqlitePool) -> Result<(), Error> {
    let query = r#"
        CREATE TABLE IF NOT EXISTS urls (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            datetime TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            url TEXT NOT NULL,
            url_hash CHAR(64) NOT NULL UNIQUE
        )
    "#;

    sqlx::query(query).execute(db_pool).await?;
    Ok(())
}

/// Create the `tags` table
pub async fn create_tags_table(db_pool: &SqlitePool) -> Result<(), Error> {
    let query = r#"
        CREATE TABLE IF NOT EXISTS tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tag TEXT NOT NULL UNIQUE
        )
    "#;

    sqlx::query(query).execute(db_pool).await?;
    Ok(())
}

/// Create the `url_tags` join table
pub async fn create_url_tags_table(db_pool: &SqlitePool) -> Result<(), Error> {
    let query = r#"
        CREATE TABLE IF NOT EXISTS url_tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            url_id INTEGER NOT NULL REFERENCES urls(id) ON DELETE CASCADE,
            tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            UNIQUE (url_id, tag_id)
        )
    "#;

    sqlx::query(query).execute(db_pool).await?;
    Ok(())
}

/// Create the `snippets` table
pub async fn create_snippets_table(db_pool: &SqlitePool) -> Result<(), Error> {
    let query = r#"
        CREATE TABLE IF NOT EXISTS snippets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            url TEXT NOT NULL,
            snippet TEXT NOT NULL,
            tags TEXT
        )
    "#;

    sqlx::query(query).execute(db_pool).await?;
    Ok(())
}

/// Create the `snippet_tags` join table
pub async fn create_snippet_tags_table(db_pool: &SqlitePool) -> Result<(), Error> {
    let query = r#"
        CREATE TABLE IF NOT EXISTS snippet_tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            snippet_id INTEGER NOT NULL REFERENCES snippets(id) ON DELETE CASCADE,
            tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            UNIQUE (snippet_id, tag_id)
        )
    "#;

    sqlx::query(query).execute(db_pool).await?;
    Ok(())
}

/// Initialize all database tables
pub async fn initialize_tables(db_pool: &SqlitePool) -> Result<(), Error> {
    create_urls_table(db_pool).await?;
    create_tags_table(db_pool).await?;
    create_url_tags_table(db_pool).await?;
    create_snippets_table(db_pool).await?;
    create_snippet_tags_table(db_pool).await?;
    Ok(())
}

/// Hash a URL to create a unique identifier
fn calculate_url_hash(url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(url);
    format!("{:x}", hasher.finalize()) // Convert to a hexadecimal string
}

/// Insert a URL into the database
pub async fn insert_url(db_pool: &SqlitePool, url: &str) -> Result<i32, Error> {
    let url_hash = calculate_url_hash(url);

    // Try to insert the URL and return its ID. If it already exists, fetch the existing ID.
    let query = r#"
        INSERT INTO urls (url, url_hash)
        VALUES (?, ?)
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

/// Insert a snippet into the database
pub async fn insert_snippet(db_pool: &SqlitePool, url: &str, snippet: &str, tags: &[&str]) -> Result<i32, Error> {
    let tags_json = serde_json::to_string(tags).unwrap_or("[]".to_string());

    let query = r#"
        INSERT INTO snippets (url, snippet, tags)
        VALUES (?, ?, ?)
        RETURNING id
    "#;

    let snippet_id: i32 = sqlx::query_scalar(query)
        .bind(url)
        .bind(snippet)
        .bind(tags_json)
        .fetch_one(db_pool)
        .await?;

    Ok(snippet_id)
}
#[derive(FromRow, Serialize)]
pub struct SnippetWithTags {
    pub id: i32,
    pub snippet: String,
    pub url: String,
    pub tags: Vec<String>,
}

/// Fetch all snippets with their associated tags
pub async fn get_snippets_with_tags(db_pool: &SqlitePool) -> Result<Vec<SnippetWithTags>, Error> {
    let query = r#"
        SELECT id, snippet, url, tags
        FROM snippets
        ORDER BY id DESC
    "#;

    let rows = sqlx::query(query).fetch_all(db_pool).await?;
    let mut results = Vec::new();

    for row in rows {
        let id: i32 = row.get("id");
        let snippet: String = row.get("snippet");
        let url: String = row.get("url");
        let tags: String = row.get("tags");
        let tags_vec: Vec<String> = serde_json::from_str(&tags).unwrap_or_default();
        results.push(SnippetWithTags {
            id,
            snippet,
            url,
            tags: tags_vec,
        });
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn setup_test_db() -> SqlitePool {
        let pool = initialize_pool(":memory:").await.unwrap();
        initialize_tables(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn test_initialize_tables() {
        let db_pool = setup_test_db().await;
        assert_eq!(check_health(&db_pool).await, "ok");
    }

    #[tokio::test]
    async fn test_insert_url() {
        let db_pool = setup_test_db().await;
        let url = "https://example.com";

        let url_id = insert_url(&db_pool, url).await.unwrap();
        assert!(url_id > 0);

        let inserted_url: (String,) = sqlx::query_as("SELECT url FROM urls WHERE id = ?")
            .bind(url_id)
            .fetch_one(&db_pool)
            .await
            .unwrap();
        assert_eq!(inserted_url.0, url);
    }

    #[tokio::test]
    async fn test_insert_snippet() {
        let db_pool = setup_test_db().await;
        let url = "https://example.com";
        let snippet = "This is a test snippet.";
        let tags = vec!["tag1", "tag2"];

        let snippet_id = insert_snippet(&db_pool, url, snippet, &tags).await.unwrap();
        assert!(snippet_id > 0);

        let inserted_snippet: (String, String, String) =
            sqlx::query_as("SELECT url, snippet, tags FROM snippets WHERE id = ?")
                .bind(snippet_id)
                .fetch_one(&db_pool)
                .await
                .unwrap();
        assert_eq!(inserted_snippet.0, url);
        assert_eq!(inserted_snippet.1, snippet);

        let stored_tags: Vec<String> = serde_json::from_str(&inserted_snippet.2).unwrap_or_default();
        assert_eq!(stored_tags, tags);
    }

    #[tokio::test]
    async fn test_get_snippets_with_tags() {
        let db_pool = setup_test_db().await;
        let url = "https://example.com";
        let snippet = "This is a test snippet.";
        let tags = vec!["tag1", "tag2"];

        insert_snippet(&db_pool, url, snippet, &tags).await.unwrap();

        let snippets = get_snippets_with_tags(&db_pool).await.unwrap();
        assert_eq!(snippets.len(), 1);

        let retrieved_snippet = &snippets[0];
        assert_eq!(retrieved_snippet.url, url);
        assert_eq!(retrieved_snippet.snippet, snippet);
        assert_eq!(retrieved_snippet.tags, tags);
    }

    #[tokio::test]
    async fn test_check_health() {
        let db_pool = setup_test_db().await;
        let health = check_health(&db_pool).await;
        assert_eq!(health, "ok");
    }
}
