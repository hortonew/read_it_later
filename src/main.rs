use actix_web::{get, middleware::Logger, web, App, HttpResponse, HttpServer, Responder};
use dotenv::dotenv;
use redis::Client as RedisClient;
use serde_json::json;
use sqlx::PgPool;
use std::env;

async fn index() -> impl Responder {
    // Use the INDEX_RESPONSE environment variable if it exists, otherwise use "Welcome"
    let response = env::var("INDEX_RESPONSE").unwrap_or_else(|_| "Welcome".to_string());
    HttpResponse::Ok().body(response)
}

#[get("/health")]
async fn health(
    db_pool: web::Data<PgPool>,
    redis_client: web::Data<RedisClient>,
) -> impl Responder {
    // Check PostgreSQL connection
    let db_status = match sqlx::query("SELECT 1").execute(db_pool.get_ref()).await {
        Ok(_) => {
            println!("PostgreSQL is healthy");
            "ok"
        }
        Err(err) => {
            eprintln!("PostgreSQL health check failed: {:?}", err);
            "error"
        }
    };

    // Check Redis connection
    let redis_status = match redis_client.get_multiplexed_async_connection().await {
        Ok(mut con) => {
            // Attempt a simple PING command
            match redis::cmd("PING").query_async::<String>(&mut con).await {
                Ok(_) => {
                    println!("Redis is healthy");
                    "ok"
                }
                Err(err) => {
                    eprintln!("Redis PING failed: {:?}", err);
                    "error"
                }
            }
        }
        Err(err) => {
            eprintln!("Redis health check failed: {:?}", err);
            "error"
        }
    };

    let health_response = json!({
        "status": "ok",
        "postgres": db_status,
        "redis": redis_status
    });

    HttpResponse::Ok().json(health_response)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load the .env file
    dotenv().ok();

    // Read configuration from environment variables
    let port = env::var("WEB_PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_address = format!("0.0.0.0:{}", port);
    let postgres_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set");

    // Create a PostgreSQL connection pool
    let db_pool = PgPool::connect(&postgres_url)
        .await
        .expect("Failed to connect to PostgreSQL");

    // Create a Redis client
    let redis_client = RedisClient::open(redis_url).expect("Failed to connect to Redis");

    // Start the Actix Web server
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(redis_client.clone()))
            .route("/", web::get().to(index))
            .route("/index.html", web::get().to(index))
            .service(health)
    })
    .bind(&bind_address)?
    .run()
    .await
}
