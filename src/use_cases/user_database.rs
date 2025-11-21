use mockall::automock;
use thiserror::Error;
use uuid::Uuid;

use crate::domain::user::User;

pub type UserDatabaseResult<T> = Result<T, UserDatabaseError>;

pub trait UserDatabaseClone: UserDatabase + Clone {}

#[automock]
pub trait UserDatabase: Send + Sync {
    async fn create_user(&self, user: User) -> UserDatabaseResult<()>;

    async fn get_user_by_username(&self, username: String) -> UserDatabaseResult<User>;

    async fn get_user_by_email(&self, email: String) -> UserDatabaseResult<User>;

    async fn get_user_by_id(&self, id: Uuid) -> UserDatabaseResult<User>;
}

#[derive(Debug, Error)]
pub enum UserDatabaseError {
    #[error("Internal DB error: {0}")]
    InternalDBError(String),
}
