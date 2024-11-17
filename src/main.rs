use actix_cors::Cors;
use actix_web::{middleware::Logger, App, HttpServer};
use dotenv::dotenv;
// use env_logger::Env;
use std::env;
mod services;
use services::{api, caching, database};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load the .env file
    dotenv().ok();

    // Initialize the logger
    // env_logger::init_from_env(Env::default().default_filter_or("info"));

    // Read configuration from environment variables
    let port = env::var("WEB_PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_address = format!("0.0.0.0:{}", port);
    let postgres_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set");

    // Initialize PostgreSQL pool
    let db_pool = database::initialize_pool(&postgres_url)
        .await
        .expect("Failed to initialize PostgreSQL pool");

    services::database::create_urls_table(&db_pool)
        .await
        .expect("Failed to create `urls` table");

    services::database::create_tags_table(&db_pool)
        .await
        .expect("Failed to create `tags` table");

    // Initialize Redis client
    let redis_client = caching::initialize_client(&redis_url).expect("Failed to initialize Redis client");

    // Start the Actix Web server
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(Cors::default().allow_any_origin().allow_any_method().allow_any_header())
            .app_data(actix_web::web::Data::new(db_pool.clone()))
            .app_data(actix_web::web::Data::new(redis_client.clone()))
            .configure(api::configure_routes) // API routes
    })
    .bind(&bind_address)?
    .run()
    .await
}
