use sqlx::PgPool;

use crate::{
    domain::user::User,
    use_cases::user_database::{UserDatabase, UserDatabaseResult},
};

#[derive(Clone)]
struct PostgresDatabase {
    pool: PgPool,
}

impl PostgresDatabase {
    fn new() -> PostgresDatabase {}
}

impl UserDatabase for PostgresDatabase {
    async fn create_user(&self, user: User) -> UserDatabaseResult<()> {
        todo!()
    }
}
