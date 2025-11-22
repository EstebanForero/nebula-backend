use std::sync::Arc;

use bcrypt::{DEFAULT_COST, hash};
use chrono::Utc;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::room::{MemberRole, Message, Room, RoomMember, RoomVisibility},
    use_cases::{
        notification_processing::MessageProcessing, realtime_broker::MessagePublisher,
        room_database::RoomDatabase,
    },
};

type RoomResult<T> = Result<T, RoomError>;

pub async fn user_is_in_room(
    db: Arc<impl RoomDatabase>,
    user_id: Uuid,
    room_id: Uuid,
) -> RoomResult<bool> {
    let rooms = db
        .get_user_rooms(user_id)
        .await
        .map_err(|err| RoomError::DatabaseError(err.to_string()))?;

    Ok(rooms.iter().any(|room| room.id == room_id))
}

pub async fn create_room(
    db: Arc<impl RoomDatabase>,
    visibility: RoomVisibility,
    password: Option<String>,
    name: String,
    user_id: Uuid,
) -> RoomResult<()> {
    let room_id = Uuid::new_v4();
    let mut hashed_pasword: Option<String> = None;

    if let RoomVisibility::Private = visibility
        && password.is_some()
    {
        let hash = hash(password.unwrap(), DEFAULT_COST)
            .map_err(|err| RoomError::PasswordHashError(err.to_string()))?;
        hashed_pasword = Some(hash)
    } else if let RoomVisibility::Private = visibility
        && password.is_none()
    {
        return Err(RoomError::PasswordNotGiven);
    };

    let room = crate::domain::room::Room {
        id: room_id,
        name,
        visibility,
        password_hash: hashed_pasword,
        created_by: user_id,
        created_at: Utc::now(),
    };
    db.create_room(room)
        .await
        .map_err(|err| RoomError::DatabaseError(err.to_string()))?;

    let room_membre = RoomMember {
        room_id,
        user_id,
        role: MemberRole::Owner,
        joined_at: Utc::now(),
    };

    db.create_room_membership(room_membre)
        .await
        .map_err(|err| RoomError::DatabaseError(err.to_string()))?;

    Ok(())
}

pub async fn join_room(db: Arc<impl RoomDatabase>, room_id: Uuid, user_id: Uuid) -> RoomResult<()> {
    let room_member = RoomMember {
        room_id,
        user_id,
        role: MemberRole::Member,
        joined_at: Utc::now(),
    };

    db.create_room_membership(room_member)
        .await
        .map_err(|err| RoomError::DatabaseError(err.to_string()))?;

    Ok(())
}

pub async fn get_user_rooms_use(
    db: Arc<impl RoomDatabase>,
    user_id: Uuid,
) -> RoomResult<Vec<Room>> {
    let rooms = db
        .get_user_rooms(user_id)
        .await
        .map_err(|err| RoomError::DatabaseError(err.to_string()))?;

    Ok(rooms)
}

pub async fn get_all_public_rooms(db: Arc<impl RoomDatabase>) -> RoomResult<Vec<Room>> {
    let rooms = db
        .get_public_rooms()
        .await
        .map_err(|err| RoomError::DatabaseError(err.to_string()))?;

    Ok(rooms)
}

pub async fn send_message(
    db: Arc<impl RoomDatabase>,
    room_id: Uuid,
    user_id: Uuid,
    content: String,
    message_publisher: Arc<impl MessagePublisher>,
    message_procceser: Arc<impl MessageProcessing>,
) -> RoomResult<()> {
    let message = Message {
        id: Uuid::new_v4(),
        room_id,
        sender_id: user_id,
        content,
        created_at: Utc::now(),
    };

    db.create_message(message.clone())
        .await
        .map_err(|err| RoomError::DatabaseError(err.to_string()))?;

    message_publisher
        .broadcast_message(message.clone())
        .await
        .map_err(|err| RoomError::BroadcastError(err.to_string()))?;

    message_procceser
        .enqueue_message(message)
        .await
        .map_err(|err| RoomError::EnqueueMessageError(err.to_string()))?;

    Ok(())
}

pub async fn obtain_messages(
    db: Arc<impl RoomDatabase>,
    page: u32,
    page_size: u8,
    room_id: Uuid,
) -> RoomResult<Vec<Message>> {
    let messages = db
        .get_room_messages(room_id, page, page_size)
        .await
        .map_err(|err| RoomError::DatabaseError(err.to_string()))?;

    Ok(messages)
}

#[derive(Error, Debug)]
pub enum RoomError {
    #[error("database Error")]
    DatabaseError(String),
    #[error("hashing error")]
    PasswordHashError(String),
    #[error("pasword not given")]
    PasswordNotGiven,
    #[error("broadcast error")]
    BroadcastError(String),
    #[error("enqueue message error: {0}")]
    EnqueueMessageError(String),
}
