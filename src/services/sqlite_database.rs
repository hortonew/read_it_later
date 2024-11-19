use crate::services::models;
use sha2::{Digest, Sha256};
use sqlx::{Error, Row, SqlitePool};
use std::fs;
use std::path::Path;

pub struct SqliteDatabase {
    pool: SqlitePool,
}

impl SqliteDatabase {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        if database_url.starts_with("sqlite://") {
            let path = database_url.strip_prefix("sqlite://").unwrap_or(database_url);

            // Ensure the parent directory exists
            if let Some(parent) = Path::new(path).parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent).map_err(|e| sqlx::Error::Configuration(Box::new(e)))?;
                }
            }

            // Create the SQLite file if it doesn't exist
            if !Path::new(path).exists() {
                fs::File::create(path).map_err(|e| sqlx::Error::Configuration(Box::new(e)))?;
            }
        }

        // Connect to the SQLite database
        let pool = sqlx::SqlitePool::connect(database_url).await?;
        Ok(Self { pool })
    }
}

#[async_trait::async_trait]
impl models::Database for SqliteDatabase {
    async fn initialize(&self) -> Result<(), sqlx::Error> {
        initialize_tables(&self.pool).await
    }

    async fn check_health(&self) -> &'static str {
        check_health(&self.pool).await
    }

    async fn insert_url(&self, url: &str) -> Result<i32, sqlx::Error> {
        insert_url(&self.pool, url).await
    }

    async fn get_urls_with_tags(&self) -> Result<Vec<models::UrlWithTags>, sqlx::Error> {
        get_urls_with_tags(&self.pool).await
    }

    async fn insert_snippet(&self, url: &str, snippet: &str, tags: &[&str]) -> Result<i32, sqlx::Error> {
        insert_snippet(&self.pool, url, snippet, tags).await
    }

    async fn get_all_urls(&self) -> Result<Vec<models::Url>, sqlx::Error> {
        get_all_urls(&self.pool).await
    }

    async fn delete_url_by_url(&self, url: &str) -> Result<(), sqlx::Error> {
        delete_url_by_url(&self.pool, url).await
    }

    async fn insert_tags(&self, url: &str, tags: &[&str]) -> Result<(), sqlx::Error> {
        insert_tags(&self.pool, url, tags).await
    }

    async fn remove_unused_tags(&self) -> Result<(), sqlx::Error> {
        remove_unused_tags(&self.pool).await
    }

    async fn delete_snippet(&self, snippet_id: i32) -> Result<(), sqlx::Error> {
        delete_snippet(&self.pool, snippet_id).await
    }

    async fn get_snippets_with_tags(&self) -> Result<Vec<models::SnippetWithTags>, sqlx::Error> {
        get_snippets_with_tags(&self.pool).await
    }

    async fn get_tags_with_urls_and_snippets(&self) -> Result<Vec<models::TagWithUrlsAndSnippets>, sqlx::Error> {
        get_tags_with_urls_and_snippets(&self.pool).await
    }
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

/// Helper: Insert or fetch a tag ID
async fn get_or_create_tag(db_pool: &SqlitePool, tag: &str) -> Result<i32, Error> {
    match sqlx::query_scalar::<_, i32>(
        r#"
        INSERT INTO tags (tag)
        VALUES (?)
        ON CONFLICT(tag) DO NOTHING
        RETURNING id
        "#,
    )
    .bind(tag)
    .fetch_optional(db_pool)
    .await?
    {
        Some(id) => Ok(id),
        None => {
            // If the tag exists, fetch its ID
            sqlx::query_scalar("SELECT id FROM tags WHERE tag = ?")
                .bind(tag)
                .fetch_one(db_pool)
                .await
        }
    }
}

/// Helper: Link a tag to a snippet or URL
async fn link_to_tag(
    db_pool: &SqlitePool,
    tag_id: i32,
    target_id: i32,
    table: &str,
    column: &str,
) -> Result<(), Error> {
    let query = format!(
        r#"
        INSERT INTO {table} ({column}, tag_id)
        VALUES (?, ?)
        ON CONFLICT({column}, tag_id) DO NOTHING
        "#,
        table = table,
        column = column
    );

    sqlx::query(&query)
        .bind(target_id)
        .bind(tag_id)
        .execute(db_pool)
        .await?;
    Ok(())
}

/// Insert a snippet into the database
pub async fn insert_snippet(db_pool: &SqlitePool, url: &str, snippet: &str, tags: &[&str]) -> Result<i32, Error> {
    let tags_json = serde_json::to_string(tags).unwrap_or("[]".to_string());

    // Insert the snippet
    let snippet_id: i32 = sqlx::query_scalar(
        r#"
        INSERT INTO snippets (url, snippet, tags)
        VALUES (?, ?, ?)
        RETURNING id
        "#,
    )
    .bind(url)
    .bind(snippet)
    .bind(tags_json)
    .fetch_one(db_pool)
    .await?;

    // Link tags to the snippet
    for tag in tags {
        let tag_id = get_or_create_tag(db_pool, tag).await?;
        link_to_tag(db_pool, tag_id, snippet_id, "snippet_tags", "snippet_id").await?;
    }

    Ok(snippet_id)
}

/// Insert tags for a URL
pub async fn insert_tags(db_pool: &SqlitePool, url: &str, tags: &[&str]) -> Result<(), Error> {
    if tags.is_empty() {
        return Ok(()); // Nothing to insert
    }

    // Insert or retrieve the URL ID
    let url_id = insert_url(db_pool, url).await?;

    // Link tags to the URL
    for tag in tags {
        let tag_id = get_or_create_tag(db_pool, tag).await?;
        link_to_tag(db_pool, tag_id, url_id, "url_tags", "url_id").await?;
    }

    Ok(())
}

/// Fetch all snippets with their associated tags
pub async fn get_snippets_with_tags(db_pool: &SqlitePool) -> Result<Vec<models::SnippetWithTags>, Error> {
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
        results.push(models::SnippetWithTags {
            id,
            snippet,
            url,
            tags: tags_vec,
        });
    }

    Ok(results)
}

pub async fn get_all_urls(db_pool: &SqlitePool) -> Result<Vec<models::Url>, sqlx::Error> {
    let query = r#"
        SELECT id, datetime, url, url_hash
        FROM urls
        ORDER BY datetime DESC
    "#;

    // Use the `query_as` method to map rows to the `Url` struct.
    let urls = sqlx::query_as::<_, models::Url>(query).fetch_all(db_pool).await?;

    Ok(urls)
}

pub async fn get_urls_with_tags(db_pool: &SqlitePool) -> Result<Vec<models::UrlWithTags>, sqlx::Error> {
    let query = r#"
        SELECT urls.url, 
               COALESCE(GROUP_CONCAT(tags.tag, ','), '') AS tags
        FROM urls
        LEFT JOIN url_tags ON urls.id = url_tags.url_id
        LEFT JOIN tags ON url_tags.tag_id = tags.id
        GROUP BY urls.id, urls.datetime, urls.url
        ORDER BY urls.datetime DESC
    "#;

    let rows = sqlx::query(query).fetch_all(db_pool).await?;
    let mut results = Vec::new();

    for row in rows {
        let url: String = row.get("url");
        let tags_string: String = row.try_get("tags").unwrap_or_default(); // Ensure tags string is never null
        let tags: Vec<String> = if tags_string.is_empty() {
            Vec::new()
        } else {
            tags_string.split(',').map(String::from).collect()
        };
        let display_url = url.split('?').next().unwrap_or(&url).to_string();

        results.push(models::UrlWithTags { url, tags, display_url });
    }

    Ok(results)
}

pub async fn delete_url_by_url(db_pool: &SqlitePool, url: &str) -> Result<(), Error> {
    let url_hash = calculate_url_hash(url);
    let query = "DELETE FROM urls WHERE url_hash = ?";
    sqlx::query(query).bind(url_hash).execute(db_pool).await?;
    Ok(())
}

pub async fn remove_unused_tags(db_pool: &SqlitePool) -> Result<(), Error> {
    let query = r#"
        DELETE FROM tags
        WHERE id NOT IN (SELECT tag_id FROM url_tags)
          AND id NOT IN (SELECT tag_id FROM snippet_tags)
    "#;
    sqlx::query(query).execute(db_pool).await?;
    Ok(())
}

pub async fn delete_snippet(db_pool: &SqlitePool, snippet_id: i32) -> Result<(), Error> {
    let query = "DELETE FROM snippets WHERE id = ?";
    sqlx::query(query).bind(snippet_id).execute(db_pool).await?;
    Ok(())
}

pub async fn get_tags_with_urls_and_snippets(
    db_pool: &SqlitePool,
) -> Result<Vec<models::TagWithUrlsAndSnippets>, Error> {
    let query = r#"
        SELECT 
            tags.tag, 
            GROUP_CONCAT(DISTINCT urls.url) AS urls,
            GROUP_CONCAT(DISTINCT snippets.id) AS snippet_ids
        FROM tags
        LEFT JOIN url_tags ON tags.id = url_tags.tag_id
        LEFT JOIN urls ON url_tags.url_id = urls.id
        LEFT JOIN snippet_tags ON tags.id = snippet_tags.tag_id
        LEFT JOIN snippets ON snippet_tags.snippet_id = snippets.id
        GROUP BY tags.id, tags.tag
        UNION
        SELECT 
            snippet_tags.tag AS tag,
            NULL AS urls,
            GROUP_CONCAT(DISTINCT snippets.id) AS snippet_ids
        FROM snippets,
        json_each(snippets.tags) AS snippet_tags
        WHERE snippet_tags.tag NOT IN (
            SELECT tag FROM tags
        )
        GROUP BY snippet_tags.tag
        ORDER BY tag
    "#;

    let rows = sqlx::query(query).fetch_all(db_pool).await?;
    let mut results = Vec::new();

    for row in rows {
        let tag: String = row.get("tag");
        let urls: String = row.try_get("urls").unwrap_or_default();
        let snippet_ids: String = row.try_get("snippet_ids").unwrap_or_default();

        // Parse URLs and snippet IDs into vectors
        let urls_vec: Vec<String> = if urls.is_empty() {
            Vec::new()
        } else {
            urls.split(',').map(String::from).collect()
        };

        let snippet_ids_vec: Vec<i32> = if snippet_ids.is_empty() {
            Vec::new()
        } else {
            snippet_ids.split(',').filter_map(|id| id.parse::<i32>().ok()).collect()
        };

        // Fetch snippets based on IDs
        let snippets = if !snippet_ids_vec.is_empty() {
            let placeholders = snippet_ids_vec.iter().map(|_| "?").collect::<Vec<&str>>().join(",");

            let snippet_query = format!(
                "SELECT id, snippet, url, tags FROM snippets WHERE id IN ({})",
                placeholders
            );

            let mut query = sqlx::query(&snippet_query);

            for snippet_id in &snippet_ids_vec {
                query = query.bind(snippet_id);
            }

            let snippet_rows = query.fetch_all(db_pool).await?;

            snippet_rows
                .into_iter()
                .map(|row| {
                    let id: i32 = row.get("id");
                    let snippet: String = row.get("snippet");
                    let url: String = row.get("url");
                    let tags: String = row.get("tags");
                    let tags_vec: Vec<String> = serde_json::from_str(&tags).unwrap_or_default();

                    Ok(models::SnippetWithTags {
                        id,
                        snippet,
                        url,
                        tags: tags_vec,
                    })
                })
                .collect::<Result<Vec<models::SnippetWithTags>, sqlx::Error>>()?
        } else {
            Vec::new()
        };

        results.push(models::TagWithUrlsAndSnippets {
            tag,
            urls: urls_vec,
            snippets,
        });
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
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
    #[tokio::test]
    async fn test_get_all_urls() {
        let db_pool = setup_test_db().await;

        let url1 = "https://example1.com";
        let url2 = "https://example2.com";

        insert_url(&db_pool, url1).await.unwrap();
        insert_url(&db_pool, url2).await.unwrap();

        let urls = get_all_urls(&db_pool).await.unwrap();
        assert_eq!(urls.len(), 2);
        assert!(urls.iter().any(|u| u.url == url1));
        assert!(urls.iter().any(|u| u.url == url2));
    }

    #[tokio::test]
    async fn test_get_urls_with_tags() {
        let db_pool = setup_test_db().await;

        let url = "https://example.com";
        let tags = vec!["tag1", "tag2"];
        insert_tags(&db_pool, url, &tags).await.unwrap();

        let urls_with_tags = get_urls_with_tags(&db_pool).await.unwrap();
        assert_eq!(urls_with_tags.len(), 1);
        let retrieved = &urls_with_tags[0];
        assert_eq!(retrieved.url, url);
        assert_eq!(retrieved.tags, tags);
    }

    #[tokio::test]
    async fn test_delete_url_by_url() {
        let db_pool = setup_test_db().await;

        let url = "https://example.com";
        insert_url(&db_pool, url).await.unwrap();
        delete_url_by_url(&db_pool, url).await.unwrap();

        let urls = get_all_urls(&db_pool).await.unwrap();
        assert!(urls.is_empty());
    }

    #[tokio::test]
    async fn test_insert_tags() {
        let db_pool = setup_test_db().await;

        let url = "https://example.com";
        let tags = vec!["tag1", "tag2"];
        insert_tags(&db_pool, url, &tags).await.unwrap();

        let urls_with_tags = get_urls_with_tags(&db_pool).await.unwrap();
        assert_eq!(urls_with_tags.len(), 1);
        assert_eq!(urls_with_tags[0].tags, tags);
    }

    #[tokio::test]
    async fn test_remove_unused_tags() {
        let db_pool = setup_test_db().await;

        let url = "https://example.com";
        let tags = vec!["tag1", "tag2"];
        insert_tags(&db_pool, url, &tags).await.unwrap();
        delete_url_by_url(&db_pool, url).await.unwrap();
        remove_unused_tags(&db_pool).await.unwrap();

        let remaining_tags: Vec<String> = sqlx::query_scalar("SELECT tag FROM tags")
            .fetch_all(&db_pool)
            .await
            .unwrap();
        assert!(remaining_tags.is_empty());
    }

    #[tokio::test]
    async fn test_delete_snippet() {
        let db_pool = setup_test_db().await;

        let url = "https://example.com";
        let snippet = "This is a test snippet.";
        let tags = vec!["tag1", "tag2"];
        let snippet_id = insert_snippet(&db_pool, url, snippet, &tags).await.unwrap();

        delete_snippet(&db_pool, snippet_id).await.unwrap();
        let snippets = get_snippets_with_tags(&db_pool).await.unwrap();
        assert!(snippets.is_empty());
    }

    #[tokio::test]
    async fn test_get_tags_with_urls_and_snippets() {
        let db_pool = setup_test_db().await;

        let url = "https://example.com";
        let snippet = "Test snippet content.";
        let tags = vec!["tag1", "tag2"];

        // Insert data
        println!("Inserting snippet...");
        insert_snippet(&db_pool, url, snippet, &tags).await.unwrap();

        // Fetch all data to validate setup
        println!("Fetching all snippets...");
        let snippets = get_snippets_with_tags(&db_pool).await.unwrap();
        println!("Snippets: {:?}", snippets);

        println!("Fetching all tags...");
        let tags_in_db = sqlx::query_scalar::<_, String>("SELECT tag FROM tags")
            .fetch_all(&db_pool)
            .await
            .unwrap();
        println!("Tags: {:?}", tags_in_db);

        println!("Fetching tags with URLs and snippets...");
        let tags_with_urls_and_snippets = get_tags_with_urls_and_snippets(&db_pool).await.unwrap();

        // Assertions
        assert_eq!(
            tags_with_urls_and_snippets.len(),
            2,
            "Expected 2 tags, got {}",
            tags_with_urls_and_snippets.len()
        );
        assert!(
            tags_with_urls_and_snippets.iter().any(|t| t.tag == "tag1"),
            "Tag 'tag1' not found"
        );
        assert!(
            tags_with_urls_and_snippets.iter().any(|t| t.tag == "tag2"),
            "Tag 'tag2' not found"
        );
    }
}
