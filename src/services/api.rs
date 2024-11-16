use actix_web::{get, web, HttpResponse, Responder};
use redis::Client as RedisClient;
use serde_json::json;
use sqlx::PgPool;

use crate::services::{caching, database};

#[get("/")]
async fn index() -> impl Responder {
    let response = std::env::var("INDEX_RESPONSE").unwrap_or_else(|_| "Welcome".to_string());
    HttpResponse::Ok().body(response)
}

#[get("/health")]
async fn health(
    db_pool: web::Data<PgPool>,
    redis_client: web::Data<RedisClient>,
) -> impl Responder {
    let db_status = database::check_health(db_pool.get_ref()).await;
    let redis_status = caching::check_health(redis_client.get_ref()).await;

    let health_response = json!({
        "status": "ok",
        "postgres": db_status,
        "redis": redis_status
    });

    HttpResponse::Ok().json(health_response)
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(index).service(health);
}
