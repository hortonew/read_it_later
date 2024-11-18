use crate::services::{caching, database};
use actix_web::{get, post, web, HttpResponse, Responder};
use ammonia::Builder;
use redis::Client as RedisClient;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use tera::{Context, Tera};

fn sanitize_with_allowed_tags(input: &str) -> ammonia::Document {
    Builder::default()
        .add_tags(["b", "i", "em", "strong", "a"])
        .add_generic_attributes(["href", "title"])
        .clean(input)
}

#[get("/")]
async fn index(db_pool: web::Data<PgPool>, tmpl: web::Data<Tera>) -> impl Responder {
    let result = database::get_urls_with_tags(db_pool.get_ref()).await;

    match result {
        Ok(urls_with_tags) => {
            // Enrich the data to include display_url
            let enriched_urls_with_tags: Vec<_> = urls_with_tags
                .into_iter()
                .map(|mut url_with_tags| {
                    url_with_tags.display_url = url_with_tags
                        .url
                        .split('?')
                        .next()
                        .unwrap_or(&url_with_tags.url)
                        .to_string();
                    url_with_tags
                })
                .collect();

            // Insert enriched data into the context
            let mut context = Context::new();
            context.insert("urls_with_tags", &enriched_urls_with_tags);
            context.insert("title", "Read it Later");

            // Render the template
            match tmpl.render("index.html", &context) {
                Ok(rendered) => HttpResponse::Ok().content_type("text/html").body(rendered),
                Err(e) => {
                    eprintln!("Template error: {:?}", e);
                    HttpResponse::InternalServerError().body("Template error")
                }
            }
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
async fn tags_page(db_pool: web::Data<PgPool>, tmpl: web::Data<Tera>) -> impl Responder {
    let result = database::get_tags_with_urls_and_snippets(db_pool.get_ref()).await;

    match result {
        Ok(tags_with_urls_and_snippets) => {
            let mut context = Context::new();
            context.insert("tags_with_urls_and_snippets", &tags_with_urls_and_snippets);
            context.insert("title", "Tags");

            match tmpl.render("tags.html", &context) {
                Ok(rendered) => HttpResponse::Ok().content_type("text/html").body(rendered),
                Err(e) => {
                    eprintln!("Template error: {:?}", e);
                    HttpResponse::InternalServerError().body("Template error")
                }
            }
        }
        Err(err) => {
            eprintln!("Failed to fetch tags with URLs and snippets: {:?}", err);
            HttpResponse::InternalServerError().body("Failed to fetch tags with URLs and snippets")
        }
    }
}

#[get("/snippets")]
async fn snippets_page(db_pool: web::Data<PgPool>, tmpl: web::Data<Tera>) -> impl Responder {
    let result = database::get_snippets_with_tags(db_pool.get_ref()).await;

    match result {
        Ok(snippets_with_tags) => {
            // Sanitize data
            let sanitized_snippets: Vec<_> = snippets_with_tags
                .into_iter()
                .map(|snippet_with_tags| database::SnippetWithTags {
                    id: snippet_with_tags.id,
                    snippet: sanitize_with_allowed_tags(&snippet_with_tags.snippet).to_string(),
                    url: sanitize_with_allowed_tags(&snippet_with_tags.url).to_string(),
                    tags: snippet_with_tags
                        .tags
                        .into_iter()
                        .map(|tag| sanitize_with_allowed_tags(&tag).to_string())
                        .collect(),
                })
                .collect();

            let mut context = Context::new();
            context.insert("snippets_with_tags", &sanitized_snippets);
            context.insert("title", "Snippets");

            match tmpl.render("snippets.html", &context) {
                Ok(rendered) => HttpResponse::Ok().content_type("text/html").body(rendered),
                Err(e) => {
                    eprintln!("Template error: {:?}", e);
                    HttpResponse::InternalServerError().body("Template error")
                }
            }
        }
        Err(err) => {
            eprintln!("Failed to fetch snippets with tags: {:?}", err);
            HttpResponse::InternalServerError().body("Failed to fetch snippets with tags")
        }
    }
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

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(index)
        .service(tags_page)
        .service(snippets_page)
        .service(health)
        .service(list_urls)
        .service(insert_record)
        .service(insert_tags)
        .service(list_urls_with_tags)
        .service(delete_record_by_url)
        .service(insert_snippet)
        .service(delete_snippet);
}
