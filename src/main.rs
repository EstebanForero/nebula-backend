use std::sync::Arc;

use dotenvy::dotenv;
use serde::Deserialize;

use crate::{
    infra::{database::PostgresDatabase, http_api::start_http_api, redis::RedisPublisher},
    use_cases::user_database::UserDatabase,
};

mod domain;
mod infra;
mod use_cases;

#[derive(Deserialize, Debug)]
struct EnvVariables {
    database_url: String,
    jwt_secret: String,
    redis_url: String
}

#[tokio::main]
async fn main() {
    let _ = dotenv();

    let env_vars = envy::from_env::<EnvVariables>().unwrap();

    let postgres_database = Arc::new(PostgresDatabase::new(&env_vars.database_url).await.unwrap());
    let message_publisher = Arc::new(RedisPublisher::new(redis_url))

    start_http_api();
}
