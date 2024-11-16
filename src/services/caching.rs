use redis::Client;
use std::error::Error;

pub fn initialize_client(redis_url: &str) -> Result<Client, Box<dyn Error>> {
    Ok(Client::open(redis_url)?)
}

pub async fn check_health(redis_client: &Client) -> &'static str {
    match redis_client.get_multiplexed_async_connection().await {
        Ok(mut con) => match redis::cmd("PING").query_async::<String>(&mut con).await {
            Ok(_) => "ok",
            Err(_) => "error",
        },
        Err(_) => "error",
    }
}
