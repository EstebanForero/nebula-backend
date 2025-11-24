use std::env;

use deadpool_redis::redis::cmd;
use deadpool_redis::{Config as RedisConfig, Runtime};
use nebula_backend::infra::database::PostgresDatabase;
use sqlx::{PgPool, postgres::PgPoolOptions};

#[derive(Clone)]
pub struct IntegrationConfig {
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
}

impl IntegrationConfig {
    pub fn load() -> Self {
        Self {
            database_url: env::var("TEST_DATABASE_URL")
                .unwrap_or_else(|_| "postgres://nebula:nebula123@localhost:55432/nebula".into()),
            redis_url: env::var("TEST_REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:36379/0".into()),
            jwt_secret: env::var("TEST_JWT_SECRET")
                .unwrap_or_else(|_| "integration-test-secret".into()),
        }
    }
}

pub async fn provision_database(config: &IntegrationConfig) -> (PostgresDatabase, PgPool) {
    let database = PostgresDatabase::new(&config.database_url)
        .await
        .expect("failed to connect to postgres for integration tests");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .expect("failed to create cleanup pool for integration tests");

    reset_tables(&pool).await;

    (database, pool)
}

pub async fn reset_tables(pool: &PgPool) {
    sqlx::query("TRUNCATE TABLE messages, room_members, rooms, users RESTART IDENTITY CASCADE;")
        .execute(pool)
        .await
        .expect("failed to truncate tables for test isolation");
}

pub async fn flush_redis(redis_url: &str) {
    let cfg = RedisConfig::from_url(redis_url);
    let pool = cfg
        .create_pool(Some(Runtime::Tokio1))
        .expect("failed to build redis pool for integration tests");

    let mut conn = pool
        .get()
        .await
        .expect("failed to get redis connection for integration tests");

    cmd("FLUSHDB")
        .query_async::<()>(&mut conn)
        .await
        .expect("failed to flush redis state before tests");
}
