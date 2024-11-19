use crate::services::{caching, models};
use actix_web::{get, post, web, HttpResponse, Responder};
use ammonia::Builder;
use redis::Client as RedisClient;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tera::{Context, Tera};

fn sanitize_with_allowed_tags(input: &str) -> ammonia::Document {
    Builder::default()
        .add_tags(["b", "i", "em", "strong", "a"])
        .add_generic_attributes(["href", "title"])
        .clean(input)
}

#[get("/")]
async fn index(
    database: web::Data<Arc<dyn models::Database>>,
    tmpl: web::Data<Tera>,
    database_type: web::Data<String>,
) -> impl Responder {
    let result = database.get_urls_with_tags().await;

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
            context.insert("database_type", &**database_type);

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
async fn health(
    database: web::Data<Arc<dyn models::Database>>,
    redis_client: web::Data<RedisClient>,
) -> impl Responder {
    let db_status = database.check_health().await;
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
async fn insert_record(database: web::Data<Arc<dyn models::Database>>, req: web::Json<NewUrl>) -> impl Responder {
    match database.insert_url(&req.url).await {
        Ok(_) => HttpResponse::Ok().json("Record inserted successfully"),
        Err(sqlx::Error::RowNotFound) => HttpResponse::Conflict().json("Record already exists"),
        Err(err) => {
            eprintln!("Failed to insert record: {:?}", err);
            HttpResponse::InternalServerError().json("Failed to insert record")
        }
    }
}

#[get("/urls")]
async fn list_urls(database: web::Data<Arc<dyn models::Database>>) -> impl Responder {
    let result = database.get_all_urls().await;

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
async fn delete_record_by_url(
    database: web::Data<Arc<dyn models::Database>>,
    req: web::Json<DeleteUrlByUrl>,
) -> impl Responder {
    println!("Body: {:?}", req);

    let result = database.delete_url_by_url(&req.url).await;

    match result {
        Ok(_) => {
            // Call the background job to remove unused tags
            if let Err(err) = database.remove_unused_tags().await {
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

#[post("/urls/tags")]
async fn insert_tags(
    database: web::Data<Arc<dyn models::Database>>,
    req: web::Json<models::UrlTags>,
) -> impl Responder {
    let tags: Vec<&str> = req.tags.split(',').map(|tag| tag.trim()).collect();

    match database.insert_tags(&req.url, &tags).await {
        Ok(_) => HttpResponse::Ok().json("Tags inserted successfully"),
        Err(sqlx::Error::RowNotFound) => HttpResponse::Conflict().json("One or more tags already exist for this URL"),
        Err(err) => {
            eprintln!("Failed to insert tags: {:?}", err);
            HttpResponse::InternalServerError().json("Failed to insert tags")
        }
    }
}

#[get("/urls_with_tags")]
async fn list_urls_with_tags(database: web::Data<Arc<dyn models::Database>>) -> impl Responder {
    match database.get_urls_with_tags().await {
        Ok(urls_with_tags) => HttpResponse::Ok().json(urls_with_tags),
        Err(err) => {
            eprintln!("Failed to fetch URLs with tags: {:?}", err);
            HttpResponse::InternalServerError().json("Failed to fetch URLs with tags")
        }
    }
}

#[get("/tags")]
async fn tags_page(
    database: web::Data<Arc<dyn models::Database>>,
    tmpl: web::Data<Tera>,
    database_type: web::Data<String>,
) -> impl Responder {
    let result = database.get_tags_with_urls_and_snippets().await;

    match result {
        Ok(tags_with_urls_and_snippets) => {
            let mut context = Context::new();
            context.insert("tags_with_urls_and_snippets", &tags_with_urls_and_snippets);
            context.insert("title", "Tags");
            context.insert("database_type", &**database_type);

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
async fn snippets_page(
    database: web::Data<Arc<dyn models::Database>>,
    tmpl: web::Data<Tera>,
    database_type: web::Data<String>,
) -> impl Responder {
    let result = database.get_snippets_with_tags().await;

    match result {
        Ok(snippets_with_tags) => {
            // Sanitize data
            let sanitized_snippets: Vec<_> = snippets_with_tags
                .into_iter()
                .map(|snippet_with_tags| models::SnippetWithTags {
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
            context.insert("database_type", &**database_type);

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

#[post("/snippets")]
async fn insert_snippet(
    database: web::Data<Arc<dyn models::Database>>,
    req: web::Json<models::NewSnippet>,
) -> impl Responder {
    let tags: Vec<&str> = req.tags.split(',').map(|tag| tag.trim()).collect();

    // Log the received tags for debugging
    println!("Received tags for snippet: {:?}", tags);

    match database.insert_snippet(&req.url, &req.snippet, &tags).await {
        Ok(_) => HttpResponse::Ok().json("Snippet inserted successfully"),
        Err(err) => {
            eprintln!("Failed to insert snippet: {:?}", err);
            HttpResponse::InternalServerError().json("Failed to insert snippet")
        }
    }
}

#[post("/snippets/delete")]
async fn delete_snippet(
    database: web::Data<Arc<dyn models::Database>>,
    req: web::Json<models::DeleteSnippet>,
) -> impl Responder {
    println!("Body: {:?}", req);

    let result = database.delete_snippet(req.id).await;

    match result {
        Ok(_) => {
            // Call the background job to remove unused tags
            if let Err(err) = database.remove_unused_tags().await {
                eprintln!("Failed to remove unused tags: {:?}", err);
            }
            HttpResponse::Ok().json("Snippet deleted successfully")
        }
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
