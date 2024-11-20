use actix_cors::Cors;
use actix_web::{middleware::Logger, App, HttpServer};
use dotenv::dotenv;
use std::env;
use tera::Tera;
mod services;
use services::{api, models, postgres_database, sqlite_database};
use std::sync::Arc;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("read_it_later has started");

    // Load the .env file
    dotenv().ok();
    println!("environment variables loaded");

    // Initialize the logger
    // env_logger::init_from_env(Env::default().default_filter_or("info"));

    // Read configuration from environment variables
    let port = env::var("WEB_PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_address = format!("0.0.0.0:{}", port);

    let database_type = env::var("DATABASE_TYPE").unwrap_or_else(|_| "sqlite".to_string());
    let database_url = match database_type.as_str() {
        "sqlite" => env::var("SQLITE_URL").expect("SQLITE_URL must be set for SQLite"),
        _ => env::var("POSTGRES_URL").expect("POSTGRES_URL must be set for PostgreSQL"),
    };

    let database: Arc<dyn models::Database> = match database_type.as_str() {
        "sqlite" => Arc::new(sqlite_database::SqliteDatabase::new(&database_url).await.unwrap()),
        _ => Arc::new(postgres_database::PostgresDatabase::new(&database_url).await.unwrap()),
    };

    println!("Database: {}, {}", database_type, database_url);
    println!("Listening on: http://localhost:{}", port);

    // Initialize DB pool
    database.initialize().await.expect("Failed to initialize database");

    // Initialize Tera template engine
    let tera = Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*")).expect("Failed to initialize Tera");

    // Start the Actix Web server
    HttpServer::new(move || {
        let app = App::new()
            .wrap(Logger::default())
            .wrap(Cors::default().allow_any_origin().allow_any_method().allow_any_header())
            .app_data(actix_web::web::Data::new(database.clone()))
            .app_data(actix_web::web::Data::new(tera.clone()))
            .app_data(actix_web::web::Data::new(database_type.clone()));

        app.configure(api::configure_routes) // API routes
    })
    .bind(&bind_address)?
    .run()
    .await
}
