use std::sync::Arc;

use bcrypt::{DEFAULT_COST, hash, verify};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::{domain::user::User, use_cases::user_database::UserDatabase};

type AuthResult<T> = Result<T, AuthError>;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub exp: usize,
    pub sub: String,
}

pub async fn login(
    database: Arc<impl UserDatabase>,
    identificator: String,
    password: String,
    jwt_secret: String,
) -> AuthResult<String> {
    let user = match database.get_user_by_username(identificator.clone()).await {
        Ok(user) => user,
        Err(_) => match database.get_user_by_email(identificator.clone()).await {
            Ok(user) => user,
            Err(err) => return Err(AuthError::DatabaseError(err.to_string())),
        },
    };

    let succesful = verify(password, &user.password_hash)
        .map_err(|err| AuthError::ErrorVerifying(err.to_string()))?;

    if succesful {
        let my_claims = Claims {
            exp: (Utc::now() + Duration::hours(3)).timestamp() as usize,
            sub: format!("{}", user.id),
        };

        let token = encode(
            &Header::default(),
            &my_claims,
            &EncodingKey::from_secret(jwt_secret.as_ref()),
        )
        .map_err(|_| AuthError::EncodingTokenError)?;

        Ok(token)
    } else {
        Err(AuthError::InvalidCredentials)
    }
}

pub async fn register(
    database: Arc<impl UserDatabase>,
    username: String,
    password: String,
    email: String,
) -> AuthResult<()> {
    let encrypted_password = hash(password, DEFAULT_COST)
        .map_err(|err| AuthError::PasswordHashingFailed(err.to_string()))?;

    let user = User {
        id: Uuid::new_v4(),
        username,
        email,
        password_hash: encrypted_password,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    database
        .create_user(user)
        .await
        .map_err(|err| AuthError::DatabaseError(err.to_string()))?;

    Ok(())
}

pub async fn get_user_by_id_use(db: Arc<impl UserDatabase>, user_id: Uuid) -> AuthResult<User> {
    let user = db
        .get_user_by_id(user_id)
        .await
        .map_err(|err| AuthError::DatabaseError(err.to_string()))?;

    Ok(user)
}

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Failed password hashing: {0}")]
    PasswordHashingFailed(String),
    #[error("database error")]
    DatabaseError(String),
    #[error("error verifying")]
    ErrorVerifying(String),
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("encode token Error")]
    EncodingTokenError,
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use bcrypt::{DEFAULT_COST, hash, verify};
    use chrono::Utc;
    use jsonwebtoken::{DecodingKey, Validation, decode};
    use mockall::predicate;
    use uuid::Uuid;

    use crate::{
        domain::user::User,
        use_cases::{
            auth_service::{AuthError, Claims, get_user_by_id_use, login, register},
            user_database::{MockUserDatabase, UserDatabaseError},
        },
    };

    #[tokio::test]
    async fn register_test() {
        let mut db = MockUserDatabase::new();

        db.expect_create_user()
            .withf(|user| verify("password123", &user.password_hash).unwrap_or(false))
            .once()
            .returning(|_| Ok(()));

        register(
            Arc::new(db),
            "juan".to_string(),
            "password123".to_string(),
            "juan@juan.juan".to_string(),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn login_test_failed() {
        let mut db = MockUserDatabase::new();

        db.expect_get_user_by_username().returning(|_| {
            Ok(User {
                id: Uuid::new_v4(),
                username: "juan".to_string(),
                email: "juan@juan.juan".to_string(),
                password_hash: hash("pasword", DEFAULT_COST).unwrap(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
        });

        let token = login(
            Arc::new(db),
            "juan".to_string(),
            "pasword2".to_string(),
            "swNItsMArrAbN2ueHZBWBA5Nk6N8zKWoybXhMK0EuhHso2IvCiFyQAIb6m_8SmicCRZ2x2nEHkxXgCYAoN3-XA".to_string(),
        ).await;

        assert!(token.is_err())
    }

    #[tokio::test]
    async fn login_test() {
        let mut db = MockUserDatabase::new();

        let user_id = Uuid::new_v4();

        db.expect_get_user_by_username().returning(move |_| {
            Ok(User {
                id: user_id.clone(),
                username: "juan".to_string(),
                email: "juan@juan.juan".to_string(),
                password_hash: hash("pasword", DEFAULT_COST).unwrap(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
        });

        let token = login(
            Arc::new(db),
            "juan".to_string(),
            "pasword".to_string(),
            "swNItsMArrAbN2ueHZBWBA5Nk6N8zKWoybXhMK0EuhHso2IvCiFyQAIb6m_8SmicCRZ2x2nEHkxXgCYAoN3-XA".to_string(),
        ).await.unwrap();

        let clamis: Claims = decode(token, &DecodingKey::from_secret("swNItsMArrAbN2ueHZBWBA5Nk6N8zKWoybXhMK0EuhHso2IvCiFyQAIb6m_8SmicCRZ2x2nEHkxXgCYAoN3-XA".as_ref()), &Validation::default()).unwrap().claims;
        assert_eq!(clamis.sub, format!("{}", user_id))
    }

    #[tokio::test]
    async fn login_test_1() {
        let mut db = MockUserDatabase::new();

        let user_id = Uuid::new_v4();

        db.expect_get_user_by_username().returning(move |_| {
            Ok(User {
                id: user_id.clone(),
                username: "juan".to_string(),
                email: "juan@juan.juan".to_string(),
                password_hash: hash("pasword", DEFAULT_COST).unwrap(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
        });

        let token = login(
            Arc::new(db),
            "juan@juan.juan".to_string(),
            "pasword".to_string(),
            "swNItsMArrAbN2ueHZBWBA5Nk6N8zKWoybXhMK0EuhHso2IvCiFyQAIb6m_8SmicCRZ2x2nEHkxXgCYAoN3-XA".to_string(),
        ).await.unwrap();

        let clamis: Claims = decode(token, &DecodingKey::from_secret("swNItsMArrAbN2ueHZBWBA5Nk6N8zKWoybXhMK0EuhHso2IvCiFyQAIb6m_8SmicCRZ2x2nEHkxXgCYAoN3-XA".as_ref()), &Validation::default()).unwrap().claims;
        assert_eq!(clamis.sub, format!("{}", user_id))
    }

    #[tokio::test]
    async fn test_get_user_by_id_success() {
        let mut db = MockUserDatabase::new();

        let user_id = Uuid::new_v4();

        let expected_user = User {
            id: user_id,
            username: "john".into(),
            email: "john@example.com".into(),
            password_hash: "hashed".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let copy = expected_user.clone();

        db.expect_get_user_by_id()
            .returning(move |_| Ok(copy.clone()));

        let result = get_user_by_id_use(Arc::new(db), user_id).await.unwrap();

        assert_eq!(result.id, expected_user.clone().id);
        assert_eq!(result.username, expected_user.clone().username);
    }

    #[tokio::test]
    async fn test_get_user_by_id_database_error() {
        let mut db = MockUserDatabase::new();

        db.expect_get_user_by_id()
            .returning(|_| Err(UserDatabaseError::InternalDBError("db failure".to_string())));

        let result = get_user_by_id_use(Arc::new(db), Uuid::new_v4()).await;

        assert!(matches!(result, Err(AuthError::DatabaseError(_))));
    }
}
