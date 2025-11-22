use std::sync::Arc;

use dashmap::DashMap;
use dotenvy::dotenv;
use serde::Deserialize;
use tracing::info;

use crate::{
    infra::{
        database::PostgresDatabase,
        http_api::start_http_api,
        rabbit_mq::RabbitMQ,
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
    rabbitmq_host: String,
    rabbitmq_port: u16,
    rabbitmq_username: String,
    rabbitmq_password: String,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    tracing_subscriber::fmt::init();
    let _ = dotenv();

    let env_vars = envy::from_env::<EnvVariables>().unwrap();

    let postgres_database = Arc::new(PostgresDatabase::new(&env_vars.database_url).await.unwrap());

    let message_publisher = Arc::new(RedisPublisher::new(&env_vars.redis_url).await);

    let message_consumer = RedisConsumer::new(
        &env_vars.redis_url,
        infra::redis::RedisChannel::ChatMessages,
    )
    .await;

    let rooms_channels = Arc::new(DashMap::new());

    let rabbit_mq = RabbitMQ::new(
        &env_vars.rabbitmq_host,
        env_vars.rabbitmq_port,
        &env_vars.rabbitmq_username,
        &env_vars.rabbitmq_password,
    )
    .await;

    let addr = "0.0.0.0:3838".to_string();

    info!("the addr is: {}", addr);

    let rooms_channels1 = rooms_channels.clone();
    tokio::spawn(async move {
        realtime_messsage_broker(message_consumer, rooms_channels1).await;
    });

    start_http_api(
        addr,
        env_vars.jwt_secret,
        postgres_database,
        rooms_channels.clone(),
        message_publisher,
    )
    .await;
}
