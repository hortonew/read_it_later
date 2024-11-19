use crate::services::models;
use sha2::{Digest, Sha256};
use sqlx::{Error, PgPool, Row};

pub struct PostgresDatabase {
    pool: PgPool,
}

impl PostgresDatabase {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = sqlx::PgPool::connect(database_url).await?;
        Ok(Self { pool })
    }
}

#[async_trait::async_trait]
impl models::Database for PostgresDatabase {
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

/// Create the `snippets` table
pub async fn create_snippets_table(db_pool: &PgPool) -> Result<(), Error> {
    let query = r#"
        CREATE TABLE IF NOT EXISTS snippets (
            id SERIAL PRIMARY KEY,
            url TEXT NOT NULL,
            snippet TEXT NOT NULL,
            tags TEXT[]
        )
    "#;

    sqlx::query(query).execute(db_pool).await?;
    Ok(())
}

/// Create the `snippet_tags` join table
pub async fn create_snippet_tags_table(db_pool: &PgPool) -> Result<(), Error> {
    let query = r#"
        CREATE TABLE IF NOT EXISTS snippet_tags (
            id SERIAL PRIMARY KEY,
            snippet_id INTEGER NOT NULL REFERENCES snippets(id) ON DELETE CASCADE,
            tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            UNIQUE (snippet_id, tag_id)
        )
    "#;

    sqlx::query(query).execute(db_pool).await?;
    Ok(())
}

/// Initialize all database tables
pub async fn initialize_tables(db_pool: &PgPool) -> Result<(), Error> {
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

/// Insert a snippet into the database
pub async fn insert_snippet(db_pool: &PgPool, url: &str, snippet: &str, tags: &[&str]) -> Result<i32, Error> {
    let query = r#"
        INSERT INTO snippets (url, snippet, tags)
        VALUES ($1, $2, $3)
        RETURNING id
    "#;

    let snippet_id: i32 = sqlx::query_scalar(query)
        .bind(url)
        .bind(snippet)
        .bind(tags)
        .fetch_one(db_pool)
        .await?;

    // Ensure tags are added to the tags table and linked to the snippet
    for tag in tags {
        let tag_query = r#"
            INSERT INTO tags (tag)
            VALUES ($1)
            ON CONFLICT (tag) DO NOTHING
            RETURNING id
        "#;

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

        // Link the snippet and tag in the `snippet_tags` table
        let snippet_tag_query = r#"
            INSERT INTO snippet_tags (snippet_id, tag_id)
            VALUES ($1, $2)
            ON CONFLICT (snippet_id, tag_id) DO NOTHING
        "#;

        sqlx::query(snippet_tag_query)
            .bind(snippet_id)
            .bind(tag_id)
            .execute(db_pool)
            .await?;
    }

    Ok(snippet_id)
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

/// Delete a URL by its string value
pub async fn delete_url_by_url(db_pool: &PgPool, url: &str) -> Result<(), Error> {
    let url_hash = calculate_url_hash(url);
    let query = "DELETE FROM urls WHERE url_hash = $1";
    sqlx::query(query).bind(url_hash).execute(db_pool).await?;
    Ok(())
}

/// Delete a snippet by its string value
pub async fn delete_snippet(db_pool: &PgPool, id: i32) -> Result<(), Error> {
    let query = "DELETE FROM snippets WHERE id = $1";
    sqlx::query(query).bind(id).execute(db_pool).await?;
    Ok(())
}

/// Remove unused tags from the database
pub async fn remove_unused_tags(db_pool: &PgPool) -> Result<(), Error> {
    let query = r#"
        DELETE FROM tags
        WHERE id NOT IN (SELECT tag_id FROM url_tags)
          AND id NOT IN (SELECT tag_id FROM snippet_tags)
    "#;

    sqlx::query(query).execute(db_pool).await?;
    Ok(())
}

/// Fetch all URLs from the database
pub async fn get_all_urls(db_pool: &PgPool) -> Result<Vec<models::Url>, Error> {
    let query = r#"
        SELECT id, datetime, url, url_hash
        FROM urls
        ORDER BY datetime DESC
    "#;

    let urls = sqlx::query_as::<_, models::Url>(query).fetch_all(db_pool).await?;

    Ok(urls)
}

/// Fetch all URLs with their associated tags
/// Fetch all URLs with their associated tags
pub async fn get_urls_with_tags(db_pool: &PgPool) -> Result<Vec<models::UrlWithTags>, sqlx::Error> {
    let query = r#"
        SELECT urls.url, COALESCE(ARRAY_AGG(tags.tag), ARRAY[]::TEXT[]) AS tags
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
        let tags: Vec<String> = row.try_get("tags").unwrap_or_default(); // Ensure tags is never null
        let display_url = url.split('?').next().unwrap_or(url.as_str()).to_string();
        results.push(models::UrlWithTags { url, tags, display_url });
    }

    Ok(results)
}

/// Fetch all snippets with their associated tags
pub async fn get_snippets_with_tags(db_pool: &PgPool) -> Result<Vec<models::SnippetWithTags>, Error> {
    let query = r#"
        SELECT id, snippet, url, COALESCE(tags, ARRAY[]::TEXT[]) AS tags
        FROM snippets
        ORDER BY id DESC
    "#;

    let rows = sqlx::query(query).fetch_all(db_pool).await?;
    let mut results = Vec::new();

    for row in rows {
        let id: i32 = row.get("id");
        let snippet: String = row.get("snippet");
        let url: String = row.get("url");
        let tags: Vec<String> = row.try_get("tags").unwrap_or_default();
        results.push(models::SnippetWithTags { id, snippet, url, tags });
    }

    Ok(results)
}

pub async fn get_tags_with_urls_and_snippets(db_pool: &PgPool) -> Result<Vec<models::TagWithUrlsAndSnippets>, Error> {
    let query = r#"
        SELECT tags.tag, 
               COALESCE(ARRAY_AGG(DISTINCT urls.url), ARRAY[]::TEXT[]) AS urls,
               COALESCE(ARRAY_AGG(DISTINCT snippets.id), ARRAY[]::INTEGER[]) AS snippet_ids
        FROM tags
        LEFT JOIN url_tags ON tags.id = url_tags.tag_id
        LEFT JOIN urls ON url_tags.url_id = urls.id
        LEFT JOIN snippet_tags ON tags.id = snippet_tags.tag_id
        LEFT JOIN snippets ON snippet_tags.snippet_id = snippets.id
        GROUP BY tags.tag
        UNION
        SELECT unnest(snippets.tags) AS tag,
               ARRAY[]::TEXT[] AS urls,
               ARRAY_AGG(snippets.id) AS snippet_ids
        FROM snippets
        WHERE NOT EXISTS (
            SELECT 1
            FROM tags
            WHERE tags.tag = ANY(snippets.tags)
        )
        GROUP BY tag
        ORDER BY tag
    "#;

    let rows = sqlx::query(query).fetch_all(db_pool).await?;
    let mut results = Vec::new();

    for row in rows {
        let tag: String = row.get("tag");
        let urls: Vec<String> = row.try_get("urls").unwrap_or_default();
        let snippet_ids: Vec<i32> = row.try_get("snippet_ids").unwrap_or_default();

        let snippets = sqlx::query_as::<_, models::SnippetWithTags>(
            "SELECT id, snippet, url, COALESCE(tags, ARRAY[]::TEXT[]) AS tags FROM snippets WHERE id = ANY($1)",
        )
        .bind(&snippet_ids)
        .fetch_all(db_pool)
        .await?;

        results.push(models::TagWithUrlsAndSnippets { tag, urls, snippets });
    }

    Ok(results)
}
