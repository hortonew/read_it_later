use chrono;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Deserialize)]
pub struct UrlTags {
    pub url: String,
    pub tags: String,
}

/// Struct representing a URL
#[derive(FromRow, Serialize)]
pub struct Url {
    pub id: i32,
    pub datetime: chrono::NaiveDateTime,
    pub url: String,
    pub url_hash: String,
}

#[derive(Serialize, Debug)]
pub struct UrlWithTags {
    pub url: String,
    pub tags: Vec<String>,
    pub display_url: String,
}

#[derive(Deserialize)]
pub struct NewUrl {
    pub url: String,
}

#[derive(Deserialize)]
pub struct NewSnippet {
    pub url: String,
    pub snippet: String,
    pub tags: String,
}

#[derive(Deserialize, Debug)]
pub struct DeleteSnippet {
    pub id: i32,
}

#[derive(Deserialize, Debug)]
pub struct DeleteUrlByUrl {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SnippetWithTags {
    pub id: i32,
    pub snippet: String,
    pub url: String,
    pub tags: Vec<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct TagWithUrlsAndSnippets {
    pub tag: String,
    pub urls: Vec<String>,
    pub snippets: Vec<SnippetWithTags>,
}

#[async_trait::async_trait]
pub trait Database: Send + Sync {
    async fn initialize(&self) -> Result<(), sqlx::Error>;
    async fn check_health(&self) -> &'static str;

    // URL-related operations
    async fn insert_url(&self, url: &str) -> Result<i32, sqlx::Error>;
    async fn get_urls_with_tags(&self) -> Result<Vec<UrlWithTags>, sqlx::Error>;
    async fn get_all_urls(&self) -> Result<Vec<Url>, sqlx::Error>;
    async fn delete_url_by_url(&self, url: &str) -> Result<(), sqlx::Error>;
    async fn insert_tags(&self, url: &str, tags: &[&str]) -> Result<(), sqlx::Error>;
    async fn remove_unused_tags(&self) -> Result<(), sqlx::Error>;

    // Snippet-related operations
    async fn insert_snippet(&self, url: &str, snippet: &str, tags: &[&str]) -> Result<i32, sqlx::Error>;
    async fn delete_snippet(&self, snippet_id: i32) -> Result<(), sqlx::Error>;
    async fn get_snippets_with_tags(&self) -> Result<Vec<SnippetWithTags>, sqlx::Error>;

    // Tags-related operations
    async fn get_tags_with_urls_and_snippets(&self) -> Result<Vec<TagWithUrlsAndSnippets>, sqlx::Error>;
}
