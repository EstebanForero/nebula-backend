use std::sync::Arc;

use thiserror::Error;
use uuid::Uuid;

use crate::use_cases::room_database::RoomDatabase;

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

#[derive(Error, Debug)]
pub enum RoomError {
    #[error("database Error")]
    DatabaseError(String),
}
