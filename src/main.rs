use std::sync::Arc;

use dotenvy::dotenv;
use serde::Deserialize;
use tracing::info;

use crate::{
    infra::{
        database::PostgresDatabase,
        http_api::start_http_api,
        redis::{RedisConsumer, RedisPublisher},
    },
    use_cases::user_database::UserDatabase,
};

mod domain;
mod infra;
mod use_cases;

#[derive(Deserialize, Debug)]
struct EnvVariables {
    database_url: String,
    jwt_secret: String,
    redis_url: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let _ = dotenv();

    let env_vars = envy::from_env::<EnvVariables>().unwrap();

    let postgres_database = Arc::new(PostgresDatabase::new(&env_vars.database_url).await.unwrap());

    let message_publisher = Arc::new(RedisPublisher::new(&env_vars.redis_url));

    let message_consumer = Arc::new(RedisConsumer::new(
        &env_vars.redis_url,
        infra::redis::RedisChannel::ChatMessages,
    ));

    let addr = "0.0.0.0:3838".to_string();

    info!("the addr is: {}", addr);

    start_http_api(addr, env_vars.jwt_secret, postgres_database).await;
}
