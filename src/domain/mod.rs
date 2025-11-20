use serde::{Deserialize, Serialize};
use sqlx::Type;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "room_visibility", rename_all = "lowercase")]
pub enum RoomVisibility {
    Public,
    Private,
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,

    #[serde(skip_serializing)] // never send password hash in API responses
    pub password_hash: String,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
