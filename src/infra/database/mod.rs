use anyhow::{Context, Result};
use sqlx::{PgPool, migrate::Migrator, postgres::PgPoolOptions};

use crate::{
    domain::user::User,
    use_cases::user_database::{UserDatabase, UserDatabaseResult},
};

#[derive(Clone)]
struct PostgresDatabase {
    pool: PgPool,
}

impl PostgresDatabase {
    async fn new(db_url: &str) -> Result<PostgresDatabase> {
        let pool = PgPoolOptions::new().connect(db_url).await?;

        Migrator::new(std::path::Path::new("./migrations"))
            .await?
            .run(&pool)
            .await
            .context("Failed to connect to postgres")?;

        Ok(PostgresDatabase { pool })
    }
}

impl UserDatabase for PostgresDatabase {
    async fn create_user(&self, user: User) -> UserDatabaseResult<()> {
        todo!()
    }
}
