use async_trait::async_trait;
use thiserror::Error;

use crate::domain::user::User;

type DatabaseResult<T> = Result<T, DatabaseError>;

#[async_trait]
trait Database {
    async fn create_user(user: User) -> DatabaseResult<()>;
}

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("UnknownError")]
    UnknownError,
}
