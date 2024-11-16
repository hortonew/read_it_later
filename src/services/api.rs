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

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(index)
        .service(health)
        .service(list_urls)
        .service(insert_record);
}
