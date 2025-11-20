use thiserror::Error;

use crate::domain::user::User;

pub type RoomDatabaseResult<T> = Result<T, UserDatabaseError>;

pub trait UserDatabase: Clone + Send + Sync {
    async fn create_user(&self, user: User) -> RoomDatabaseResult<()>;
}

#[derive(Debug, Error)]
pub enum UserDatabaseError {
    #[error("Internal DB error: {0}")]
    InternalDBError(String),
}
