use bcrypt::{hash, DEFAULT_COST};
use chrono::{DateTime, Utc};
use thiserror::Error;
use uuid::Uuid;

use crate::{domain::user::User, use_cases::user_database::UserDatabase};

type AuthResult<T> = Result<T, AuthError>;

pub fn login(database: impl UserDatabase, username: String, password: String, jwt_secret: String) -> AuthResult<String> {}

pub fn register(database: impl UserDatabase, username: String, password: String, email: String) -> AuthResult<()> {
    let encrypted_password = hash(password, DEFAULT_COST).map_err(|err| AuthError::PasswordHashingFailed(err.to_string()))?;

    let user = User {
        id: Uuid::new_v4(),
        username: username,
        email: email,
        password_hash: encrypted_password,
        created_at: Utc::now();

    }
    database.create_user(user);

    Ok(())
}


#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Failed password hashing: {0}")]
    PasswordHashingFailed(String)
}
