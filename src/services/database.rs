use sqlx::{Error, PgPool};

pub async fn initialize_pool(postgres_url: &str) -> Result<PgPool, Error> {
    PgPool::connect(postgres_url).await
}

pub async fn check_health(db_pool: &PgPool) -> &'static str {
    match sqlx::query("SELECT 1").execute(db_pool).await {
        Ok(_) => "ok",
        Err(_) => "error",
    }
}
