use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RoomVisibility {
    Public,
    Private,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Room {
    pub id: Uuid,
    pub name: String,
    pub visibility: RoomVisibility,

    /// NULL in DB if room has no password.
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,

    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
pub enum MemberRole {
    Creator,
    Member,
}

impl ToString for MemberRole {
    fn to_string(&self) -> String {
        match self {
            MemberRole::Creator => "creator",
            MemberRole::Member => "member",
        }
        .to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct RoomMember {
    pub room_id: Uuid,
    pub user_id: Uuid,
    pub role: MemberRole,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: Uuid,
    pub room_id: Uuid,
    pub sender_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct RoomReadState {
    pub room_id: Uuid,
    pub user_id: Uuid,
    pub last_read_message_id: Option<Uuid>,
    pub last_read_at: DateTime<Utc>,
}
