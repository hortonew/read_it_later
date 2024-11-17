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
            // Render the HTML with an ordered list of URLs and their tags
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

#[get("/saves")]
async fn saves(db_pool: web::Data<PgPool>) -> impl Responder {
    let result = database::get_all_urls(db_pool.get_ref()).await;

    match result {
        Ok(urls) => {
            // Render the HTML with an ordered list
            let html = render_html(&urls);
            HttpResponse::Ok().content_type("text/html").body(html)
        }
        Err(err) => {
            eprintln!("Failed to fetch URLs: {:?}", err);
            HttpResponse::InternalServerError().body("Failed to fetch URLs")
        }
    }
}

fn render_html(urls: &[database::Url]) -> String {
    let mut html = String::from(
        r#"<!DOCTYPE html>
        <html>
        <head>
            <title>Saved URLs</title>
            <meta http-equiv="refresh" content="3">
        </head>
        <body>
        "#,
    );
    html.push_str("<h1>Saved URLs</h1>");
    html.push_str("<ol>");
    for url in urls {
        html.push_str(&format!(
            "<li><a href=\"{url}\" target=\"_blank\">{url}</a> 
             <form method=\"POST\" action=\"/urls/delete\" style=\"display:inline;\">
                 <input type=\"hidden\" name=\"id\" value=\"{id}\" />
                 <button type=\"submit\">Remove</button>
             </form>
             </li>",
            url = url.url,
            id = url.id
        ));
    }
    html.push_str("</ol>");
    html.push_str("</body></html>");
    html
}

fn render_html_with_tags(urls_with_tags: &[(String, Vec<String>)]) -> String {
    let mut html = String::from(
        r#"<!DOCTYPE html>
        <html>
        <head>
            <title>Read it Later</title>
            <meta http-equiv="refresh" content="3">
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
        "#,
    );
    html.push_str("<h1>Read it Later</h1>");
    html.push_str("<ol>");
    for (url, tags) in urls_with_tags {
        html.push_str(&format!(
            r#"<li>
                <a href="{url}" target="_blank">{url}</a> - Tags: {tags}
                <button onclick="submitDeleteUrl(event, '{url}')">Remove</button>
            </li>"#,
            url = url,
            tags = tags.join(", ")
        ));
    }
    html.push_str("</ol>");
    html.push_str("</body></html>");
    html
}

#[derive(Deserialize)]
pub struct DeleteUrl {
    id: i32,
}

#[post("/urls/delete")]
async fn delete_record(db_pool: web::Data<PgPool>, form: web::Form<DeleteUrl>) -> impl Responder {
    let result = database::delete_url(db_pool.get_ref(), form.id).await;

    match result {
        Ok(_) => HttpResponse::Found().append_header(("Location", "/saves")).finish(),
        Err(err) => {
            eprintln!("Failed to delete record: {:?}", err);
            HttpResponse::InternalServerError().body("Failed to delete record")
        }
    }
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
        Ok(_) => HttpResponse::Ok().json("URL deleted successfully"),
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

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(index)
        .service(health)
        .service(list_urls)
        .service(insert_record)
        .service(insert_tags)
        .service(list_urls_with_tags)
        .service(saves)
        .service(delete_record)
        .service(delete_record_by_url);
}
