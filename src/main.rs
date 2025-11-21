use std::sync::Arc;

use dashmap::DashMap;
use dotenvy::dotenv;
use serde::Deserialize;
use tracing::info;

use crate::{
    infra::{
        database::PostgresDatabase,
        http_api::start_http_api,
        redis::{RedisConsumer, RedisPublisher},
    },
    use_cases::{realtime_service::realtime_messsage_broker, user_database::UserDatabase},
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

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    tracing_subscriber::fmt::init();
    let _ = dotenv();

    let env_vars = envy::from_env::<EnvVariables>().unwrap();

    let postgres_database = Arc::new(PostgresDatabase::new(&env_vars.database_url).await.unwrap());

    let message_publisher = Arc::new(RedisPublisher::new(&env_vars.redis_url));

    let message_consumer = RedisConsumer::new(
        &env_vars.redis_url,
        infra::redis::RedisChannel::ChatMessages,
    );

    let rooms_channels = Arc::new(DashMap::new());

    let addr = "0.0.0.0:3838".to_string();

    info!("the addr is: {}", addr);

    start_http_api(addr, env_vars.jwt_secret, postgres_database, rooms_channels).await;

    realtime_messsage_broker(message_consumer, rooms_channels)
}
