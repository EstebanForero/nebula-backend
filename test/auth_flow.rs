use std::sync::Arc;

use jsonwebtoken::{DecodingKey, Validation, decode};
use nebula_backend::use_cases::{
    auth_service::{AuthError, Claims, get_user_by_id_use, login, register},
    user_database::UserDatabase,
};
use uuid::Uuid;

#[path = "common/mod.rs"]
mod common;

#[tokio::test]
#[serial]
async fn register_and_login_against_postgres() {
    let config = common::IntegrationConfig::load();
    let (database, pool) = common::provision_database(&config).await;

    let username = format!("integration-user-{}", Uuid::new_v4().simple());
    let email = format!("{username}@example.com");
    let password = "password123*".to_string();

    register(Arc::new(database.clone()), username.clone(), password.clone(), email.clone())
        .await
        .expect("register should succeed against postgres");

    let token = login(Arc::new(database.clone()), username.clone(), password.clone(), config.jwt_secret.clone())
        .await
        .expect("login should return a signed JWT");

    let claims: Claims = decode(
        &token,
        &DecodingKey::from_secret(config.jwt_secret.as_ref()),
        &Validation::default(),
    )
    .expect("jwt should decode with the provided secret")
    .claims;

    let fetched = get_user_by_id_use(Arc::new(database), Uuid::parse_str(&claims.sub).unwrap())
        .await
        .expect("user lookup by id should succeed");

    assert_eq!(fetched.email, email);
    assert_eq!(fetched.username, username);

    common::reset_tables(&pool).await;
}

#[tokio::test]
#[serial]
async fn login_with_email_succeeds() {
    let config = common::IntegrationConfig::load();
    let (database, pool) = common::provision_database(&config).await;

    let username = format!("integration-email-{}", Uuid::new_v4().simple());
    let email = format!("{username}@example.com");
    let password = "password123*".to_string();

    register(Arc::new(database.clone()), username.clone(), password.clone(), email.clone())
        .await
        .expect("register should succeed");

    let token = login(Arc::new(database.clone()), email.clone(), password.clone(), config.jwt_secret.clone())
        .await
        .expect("login with email should succeed");

    let claims: Claims = decode(
        &token,
        &DecodingKey::from_secret(config.jwt_secret.as_ref()),
        &Validation::default(),
    )
    .expect("jwt should decode")
    .claims;

    let user = get_user_by_id_use(
        Arc::new(database),
        Uuid::parse_str(&claims.sub).expect("sub should be uuid"),
    )
    .await
    .expect("user exists");
    assert_eq!(claims.sub, user.id.to_string());

    common::reset_tables(&pool).await;
}

#[tokio::test]
#[serial]
async fn login_with_wrong_password_fails() {
    let config = common::IntegrationConfig::load();
    let (database, pool) = common::provision_database(&config).await;

    let username = format!("integration-wrong-pass-{}", Uuid::new_v4().simple());
    let email = format!("{username}@example.com");
    let password = "password123*".to_string();

    register(Arc::new(database.clone()), username.clone(), password.clone(), email.clone())
        .await
        .expect("register should succeed");

    let err = login(
        Arc::new(database.clone()),
        username.clone(),
        "nottherightone".to_string(),
        config.jwt_secret.clone(),
    )
    .await
    .expect_err("login should fail with wrong password");

    match err {
        AuthError::InvalidCredentials => {}
        other => panic!("expected invalid credentials, got {other:?}"),
    }

    common::reset_tables(&pool).await;
}

#[tokio::test]
#[serial]
async fn duplicate_username_is_rejected() {
    let config = common::IntegrationConfig::load();
    let (database, pool) = common::provision_database(&config).await;

    let username = format!("dup-user-{}", Uuid::new_v4().simple());
    let email = format!("{username}@example.com");
    let password = "password123*".to_string();

    register(Arc::new(database.clone()), username.clone(), password.clone(), email.clone())
        .await
        .expect("initial register should succeed");

    let second_email = format!("second-{username}@example.com");
    let err = register(
        Arc::new(database.clone()),
        username.clone(),
        password.clone(),
        second_email,
    )
    .await
    .expect_err("duplicate username should fail");

    match err {
        AuthError::AlreadyExisting(_) => {}
        other => panic!("expected AlreadyExisting, got {other:?}"),
    }

    common::reset_tables(&pool).await;
}
use serial_test::serial;
