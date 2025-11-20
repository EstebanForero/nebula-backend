use thiserror::Error;

use crate::domain::user::User;

pub type UserDatabaseResult<T> = Result<T, UserDatabaseError>;

pub trait UserDatabase {
    async fn create_user(user: User) -> UserDatabaseResult<()>;
}

#[derive(Debug, Error)]
pub enum UserDatabaseError {
    #[error("Internal DB error: {0}")]
    InternalDBError(String),
}
