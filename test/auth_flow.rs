use std::sync::Arc;

use jsonwebtoken::{DecodingKey, Validation, decode};
use nebula_backend::use_cases::{
    auth_service::{Claims, get_user_by_id_use, login, register},
    user_database::UserDatabase,
};
use uuid::Uuid;

#[path = "common/mod.rs"]
mod common;

#[tokio::test]
async fn register_and_login_against_postgres() {
    let config = common::IntegrationConfig::load();
    let (database, pool) = common::provision_database(&config).await;

    // Seed with a known user
    let username = format!("integration-user-{}", Uuid::new_v4().simple());
    let email = format!("{username}@example.com");
    let password = "password123*".to_string();

    register(Arc::new(database.clone()), username.clone(), password.clone(), email.clone())
        .await
        .expect("register should succeed against postgres");

    let stored_user = database
        .get_user_by_username(username.clone())
        .await
        .expect("user should be persisted");

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

    assert_eq!(
        claims.sub,
        stored_user.id.to_string(),
        "subject should match persisted user id"
    );

    let fetched = get_user_by_id_use(Arc::new(database), stored_user.id)
        .await
        .expect("user lookup by id should succeed");

    assert_eq!(fetched.email, email);
    assert_eq!(fetched.username, username);

    // Clean up to isolate from other tests
    common::reset_tables(&pool).await;
}
