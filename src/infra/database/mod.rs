use anyhow::{Context, Result};
use sqlx::{PgPool, migrate::Migrator, postgres::PgPoolOptions};

use crate::{
    domain::user::User,
    use_cases::user_database::{UserDatabase, UserDatabaseError, UserDatabaseResult},
};

#[derive(Clone)]
pub struct PostgresDatabase {
    pool: PgPool,
}

impl PostgresDatabase {
    pub async fn new(db_url: &str) -> Result<PostgresDatabase> {
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
        sqlx::query!(
            "INSERT INTO users (id, username, email, password_hash) VALUES ($1, $2, $3, $4)",
            user.id,
            user.username,
            user.email,
            user.password_hash
        )
        .execute(&self.pool)
        .await
        .map_err(|err| UserDatabaseError::InternalDBError(err.to_string()))
        .map(|_| ())
    }

    async fn get_user_by_username(&self, username: String) -> UserDatabaseResult<User> {
        sqlx::query_as!(User, "SELECT * FROM users WHERE username = $1", username)
            .fetch_one(&self.pool)
            .await
            .map_err(|err| UserDatabaseError::InternalDBError(err.to_string()))
    }

    async fn get_user_by_email(&self, email: String) -> UserDatabaseResult<User> {
        sqlx::query_as!(User, "SELECT * FROM users WHERE email = $1", email)
            .fetch_one(&self.pool)
            .await
            .map_err(|err| UserDatabaseError::InternalDBError(err.to_string()))
    }
}
