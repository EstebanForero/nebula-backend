use std::sync::Arc;

use dotenvy::dotenv;
use serde::Deserialize;

use crate::{infra::database::PostgresDatabase, use_cases::user_database::UserDatabase};

mod domain;
mod infra;
mod use_cases;

#[derive(Deserialize, Debug)]
struct EnvVariables {
    database_url: String,
}

#[tokio::main]
async fn main() {
    let _ = dotenv();

    let env_vars = envy::from_env::<EnvVariables>().unwrap();

    let user_database = PostgresDatabase::new(&env_vars.database_url).await.unwrap();
}
