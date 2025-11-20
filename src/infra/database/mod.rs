use sqlx::PgPool;

use crate::{
    domain::user::User,
    use_cases::user_database::{UserDatabase, UserDatabaseResult},
};

struct PostgresDatabase {
    pool: PgPool,
}

impl UserDatabase for PostgresDatabase {
    async fn create_user(user: User) -> UserDatabaseResult<()> {}
}
