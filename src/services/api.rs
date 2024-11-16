use crate::services::{caching, database};
use actix_web::{get, post, web, HttpResponse, Responder};
use redis::Client as RedisClient;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;

#[get("/")]
async fn index() -> impl Responder {
    let response = std::env::var("INDEX_RESPONSE").unwrap_or_else(|_| "Welcome".to_string());
    HttpResponse::Ok().body(response)
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
    let result = database::insert_url(db_pool.get_ref(), &req.url).await;

    match result {
        Ok(_) => HttpResponse::Ok().json("Record inserted successfully"),
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
    let mut html = String::from("<!DOCTYPE html><html><head><title>Saved URLs</title></head><body>");
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

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(index)
        .service(health)
        .service(list_urls)
        .service(insert_record)
        .service(saves)
        .service(delete_record);
}
