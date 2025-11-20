use mockall::automock;
use thiserror::Error;

use crate::domain::user::User;

pub type UserDatabaseResult<T> = Result<T, UserDatabaseError>;

#[automock]
pub trait UserDatabase: Send + Sync {
    async fn create_user(&self, user: User) -> UserDatabaseResult<()>;

    async fn get_user_by_username(&self, user_name: String) -> UserDatabaseResult<User>;
}

#[derive(Debug, Error)]
pub enum UserDatabaseError {
    #[error("Internal DB error: {0}")]
    InternalDBError(String),
}
