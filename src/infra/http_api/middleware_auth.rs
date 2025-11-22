use std::str::FromStr;

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{DecodingKey, Validation, decode};
use uuid::Uuid;

use crate::{infra::http_api::AppState, use_cases::auth_service::Claims};

pub async fn middleware_fn(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let bearer_token = match request.headers().get("authorization") {
        Some(auth) => auth.to_str(),
        None => {
            return Response::builder()
                .status(401)
                .body("invalid/missing auth token".into())
                .unwrap();
        }
    };

    let bearer_token = if let Ok(bearer_token) = bearer_token {
        bearer_token
    } else {
        return Response::builder()
            .status(401)
            .body("invalid/missing auth token".into())
            .unwrap();
    };

    let jwt_token = match bearer_token.strip_prefix("Bearer ") {
        Some(token) => token.trim().to_string(),
        None => {
            return Response::builder()
                .status(401)
                .body("worng header format".into())
                .unwrap();
        }
    };

    let user_id = match extract_user_id_from_jwt(jwt_token, &state.jwt_secret) {
        Ok(id) => id,
        Err(res) => return res,
    };

    request.extensions_mut().insert(user_id);

    next.run(request).await
}

pub fn extract_user_id_from_jwt(jwt_token: String, jwt_secret: &str) -> Result<Uuid, Response> {
    let my_claims: Claims = match decode(
        jwt_token,
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &Validation::default(),
    ) {
        Ok(clamis) => clamis.claims,
        Err(_err) => {
            return Err(Response::builder()
                .status(401)
                .body("invalid jwt format or expired".into())
                .unwrap());
        }
    };

    let user_id = match Uuid::from_str(&my_claims.sub) {
        Ok(uuid_real) => uuid_real,
        Err(_) => {
            return Err(Response::builder()
                .status(401)
                .body("wrong user id format".into())
                .unwrap());
        }
    };

    Ok(user_id)
}
