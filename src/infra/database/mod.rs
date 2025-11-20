use sqlx::PgPool;

use crate::{
    domain::user::User,
    use_cases::database::{Database, DatabaseResult},
};

struct PostgresDatabase {
    pool: PgPool,
}

impl Database for PostgresDatabase {
    async fn create_user(user: User) -> DatabaseResult<()> {}
}
