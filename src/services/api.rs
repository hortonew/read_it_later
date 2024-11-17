use crate::services::{caching, database};
use actix_web::{get, post, web, HttpResponse, Responder};
use redis::Client as RedisClient;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;

#[get("/")]
async fn index(db_pool: web::Data<PgPool>) -> impl Responder {
    let result = database::get_urls_with_tags(db_pool.get_ref()).await;

    match result {
        Ok(urls_with_tags) => {
            // Render the HTML with the structured list of URLs and their tags
            let html = render_html_with_tags(&urls_with_tags);
            HttpResponse::Ok().content_type("text/html").body(html)
        }
        Err(err) => {
            eprintln!("Failed to fetch URLs with tags: {:?}", err);
            HttpResponse::InternalServerError().body("Failed to fetch URLs with tags")
        }
    }
}

#[get("/health")]
async fn health(db_pool: web::Data<PgPool>, redis_client: web::Data<RedisClient>) -> impl Responder {
    let db_status = database::check_health(db_pool.get_ref()).await;
    let redis_status = caching::check_health(redis_client.get_ref()).await;

    let health_response = json!({
        "status": "ok",
        "postgres": db_status,
        "redis": redis_status
    });

    HttpResponse::Ok().json(health_response)
}

#[derive(Deserialize)]
pub struct NewUrl {
    url: String,
}

#[post("/urls/url")]
async fn insert_record(db_pool: web::Data<PgPool>, req: web::Json<NewUrl>) -> impl Responder {
    match database::insert_url(db_pool.get_ref(), &req.url).await {
        Ok(_) => HttpResponse::Ok().json("Record inserted successfully"),
        Err(sqlx::Error::RowNotFound) => HttpResponse::Conflict().json("Record already exists"),
        Err(err) => {
            eprintln!("Failed to insert record: {:?}", err);
            HttpResponse::InternalServerError().json("Failed to insert record")
        }
    }
}

#[get("/urls")]
async fn list_urls(db_pool: web::Data<PgPool>) -> impl Responder {
    let result = database::get_all_urls(db_pool.get_ref()).await;

    match result {
        Ok(urls) => HttpResponse::Ok().json(urls), // Serialize and return the list of URLs
        Err(err) => {
            eprintln!("Failed to fetch URLs: {:?}", err);
            HttpResponse::InternalServerError().json("Failed to fetch URLs")
        }
    }
}

fn render_html_with_tags(urls_with_tags: &[database::UrlWithTags]) -> String {
    let mut html = String::from(
        r#"<!DOCTYPE html>
        <html>
        <head>
            <title>Read it Later</title>
            <meta http-equiv="refresh" content="3">
            <meta charset="UTF-8">
            <script>
                async function submitDeleteUrl(event, url) {
                    event.preventDefault();
                    try {
                        const response = await fetch('/urls/delete/by-url', {
                            method: 'POST',
                            headers: { 'Content-Type': 'application/json' },
                            body: JSON.stringify({ url })
                        });
                        if (response.ok) {
                            location.reload();
                        } else {
                            alert('Failed to delete URL');
                        }
                    } catch (error) {
                        console.error('Error:', error);
                        alert('An error occurred while deleting the URL');
                    }
                }
            </script>
        </head>
        <body>
        <p><a href="/">Home</a></p>
        <p><a href="/tags">Tags</a></p>
        <p><a href="/snippets">Snippets</a></p>
        "#,
    );
    html.push_str("<h1>Read it Later</h1>");
    html.push_str("<ol>");
    for url_with_tags in urls_with_tags {
        html.push_str(&format!(
            r#"<li>
                <button onclick="submitDeleteUrl(event, '{url}')">X</button>
                <a href="{url}" target="_blank">{url}</a>
                <div>Tags: {tags}</div>
            </li>"#,
            url = url_with_tags.url,
            tags = url_with_tags.tags.join(", ")
        ));
    }
    html.push_str("</ol>");
    html.push_str("</body></html>");
    html
}

#[derive(Deserialize, Debug)]
pub struct DeleteUrlByUrl {
    url: String,
}

#[post("/urls/delete/by-url")]
async fn delete_record_by_url(db_pool: web::Data<PgPool>, req: web::Json<DeleteUrlByUrl>) -> impl Responder {
    println!("Body: {:?}", req);

    let result = database::delete_url_by_url(db_pool.get_ref(), &req.url).await;

    match result {
        Ok(_) => {
            // Call the background job to remove unused tags
            if let Err(err) = database::remove_unused_tags(db_pool.get_ref()).await {
                eprintln!("Failed to remove unused tags: {:?}", err);
            }
            HttpResponse::Ok().json("URL deleted successfully")
        }
        Err(err) => {
            eprintln!("Failed to delete URL: {:?}", err);
            HttpResponse::InternalServerError().json("Failed to delete URL")
        }
    }
}

#[derive(Deserialize)]
pub struct UrlTags {
    url: String,
    tags: String,
}

#[post("/urls/tags")]
async fn insert_tags(db_pool: web::Data<PgPool>, req: web::Json<UrlTags>) -> impl Responder {
    let tags: Vec<&str> = req.tags.split(',').map(|tag| tag.trim()).collect();

    match database::insert_tags(db_pool.get_ref(), &req.url, &tags).await {
        Ok(_) => HttpResponse::Ok().json("Tags inserted successfully"),
        Err(sqlx::Error::RowNotFound) => HttpResponse::Conflict().json("One or more tags already exist for this URL"),
        Err(err) => {
            eprintln!("Failed to insert tags: {:?}", err);
            HttpResponse::InternalServerError().json("Failed to insert tags")
        }
    }
}

#[get("/urls_with_tags")]
async fn list_urls_with_tags(db_pool: web::Data<PgPool>) -> impl Responder {
    match database::get_urls_with_tags(db_pool.get_ref()).await {
        Ok(urls_with_tags) => HttpResponse::Ok().json(urls_with_tags),
        Err(err) => {
            eprintln!("Failed to fetch URLs with tags: {:?}", err);
            HttpResponse::InternalServerError().json("Failed to fetch URLs with tags")
        }
    }
}

#[get("/tags")]
async fn tags_page(db_pool: web::Data<PgPool>) -> impl Responder {
    let result = database::get_tags_with_urls(db_pool.get_ref()).await;

    match result {
        Ok(tags_with_urls) => {
            // Render the HTML with the structured list of tags and their URLs
            let html = render_html_with_tags_and_urls(&tags_with_urls);
            HttpResponse::Ok().content_type("text/html").body(html)
        }
        Err(err) => {
            eprintln!("Failed to fetch tags with URLs: {:?}", err);
            HttpResponse::InternalServerError().body("Failed to fetch tags with URLs")
        }
    }
}

fn render_html_with_tags_and_urls(tags_with_urls: &[(String, Vec<String>)]) -> String {
    let mut html = String::from(
        r#"<!DOCTYPE html>
        <html>
        <head>
            <title>Tags</title>
            <meta http-equiv="refresh" content="3">
            <meta charset="UTF-8">
            <script>
                async function submitDeleteUrl(event, url) {
                    event.preventDefault();
                    try {
                        const response = await fetch('/urls/delete/by-url', {
                            method: 'POST',
                            headers: { 'Content-Type': 'application/json' },
                            body: JSON.stringify({ url })
                        });
                        if (response.ok) {
                            location.reload();
                        } else {
                            alert('Failed to delete URL');
                        }
                    } catch (error) {
                        console.error('Error:', error);
                        alert('An error occurred while deleting the URL');
                    }
                }
            </script>
        </head>
        <body>
        <p><a href="/">Home</a></p>
        <p><a href="/tags">Tags</a></p>
        <p><a href="/snippets">Snippets</a></p>
        "#,
    );
    html.push_str("<h1>Tags</h1>");
    for (tag, urls) in tags_with_urls {
        html.push_str(&format!("<h2>{}</h2>", tag));
        html.push_str("<ul>");
        for url in urls {
            html.push_str(&format!(
                r#"<li>
                    <button onclick="submitDeleteUrl(event, '{url}')">X</button>
                    <a href="{url}" target="_blank">{url}</a>
                </li>"#,
                url = url
            ));
        }
        html.push_str("</ul>");
    }
    html.push_str("</body></html>");
    html
}

#[derive(Deserialize)]
pub struct NewSnippet {
    url: String,
    snippet: String,
    tags: String,
}

#[post("/snippets")]
async fn insert_snippet(db_pool: web::Data<PgPool>, req: web::Json<NewSnippet>) -> impl Responder {
    let tags: Vec<&str> = req.tags.split(',').map(|tag| tag.trim()).collect();

    match database::insert_snippet(db_pool.get_ref(), &req.url, &req.snippet, &tags).await {
        Ok(_) => HttpResponse::Ok().json("Snippet inserted successfully"),
        Err(err) => {
            eprintln!("Failed to insert snippet: {:?}", err);
            HttpResponse::InternalServerError().json("Failed to insert snippet")
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct DeleteSnippet {
    id: i32,
}

#[get("/snippets")]
async fn snippets_page(db_pool: web::Data<PgPool>) -> impl Responder {
    let result = database::get_snippets_with_tags(db_pool.get_ref()).await;

    match result {
        Ok(snippets_with_tags) => {
            // Render the HTML with the structured list of snippets and their tags
            let html = render_html_with_snippets(&snippets_with_tags);
            HttpResponse::Ok().content_type("text/html").body(html)
        }
        Err(err) => {
            eprintln!("Failed to fetch snippets with tags: {:?}", err);
            HttpResponse::InternalServerError().body("Failed to fetch snippets with tags")
        }
    }
}

#[post("/snippets/delete")]
async fn delete_snippet(db_pool: web::Data<PgPool>, req: web::Json<DeleteSnippet>) -> impl Responder {
    println!("Body: {:?}", req);

    let result = database::delete_snippet(db_pool.get_ref(), req.id).await;

    match result {
        Ok(_) => HttpResponse::Ok().json("Snippet deleted successfully"),
        Err(err) => {
            eprintln!("Failed to delete snippet: {:?}", err);
            HttpResponse::InternalServerError().json("Failed to delete snippet")
        }
    }
}

fn render_html_with_snippets(snippets_with_tags: &[database::SnippetWithTags]) -> String {
    let mut html = String::from(
        r#"<!DOCTYPE html>
        <html>
        <head>
            <title>Snippets</title>
            <meta http-equiv="refresh" content="3">
            <meta charset="UTF-8">
            <script>
                async function submitDeleteSnippet(event, id) {
                    event.preventDefault();
                    console.log('Snippet ID to delete:', id); // Debugging output
                    try {
                        const response = await fetch('/snippets/delete', {
                            method: 'POST',
                            headers: { 'Content-Type': 'application/json' },
                            body: JSON.stringify({ id })
                        });
                        if (response.ok) {
                            console.log('Snippet deleted successfully'); // Debugging output
                            location.reload();
                        } else {
                            alert('Failed to delete snippet');
                            console.error('Response error:', response.statusText); // Debugging output
                        }
                    } catch (error) {
                        console.error('Error:', error);
                        alert('An error occurred while deleting the snippet');
                    }
                }
            </script>
        </head>
        <body>
        <p><a href="/">Home</a></p>
        <p><a href="/tags">Tags</a></p>
        <p><a href="/snippets">Snippets</a></p>
        "#,
    );
    html.push_str("<h1>Snippets</h1>");
    html.push_str("<ol>");
    for snippet_with_tags in snippets_with_tags {
        html.push_str(&format!(
            r#"<li>
                <button onclick="submitDeleteSnippet(event, {id})">X</button>
                <div>{snippet}</div>
                <div>URL: <a href="{url}" target="_blank">{url}</a></div>
                <div>Tags: {tags}</div>
            </li>"#,
            id = snippet_with_tags.id, // Include the `id` in the format string
            snippet = snippet_with_tags.snippet.replace('"', "&quot;").replace("'", "&#39;"), // Escape quotes
            url = snippet_with_tags.url,
            tags = snippet_with_tags.tags.join(", ")
        ));
    }
    html.push_str("</ol>");
    html.push_str("</body></html>");
    html
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(index)
        .service(tags_page)
        .service(health)
        .service(list_urls)
        .service(insert_record)
        .service(insert_tags)
        .service(list_urls_with_tags)
        .service(delete_record_by_url)
        .service(insert_snippet)
        .service(snippets_page)
        .service(delete_snippet); // Add the new route here
}
