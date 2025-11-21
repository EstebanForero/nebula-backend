use thiserror::Error;
use uuid::Uuid;

use crate::domain::{
    dto::MessageView,
    room::{Message, Room, RoomMember},
    user::User,
};

pub type RoomDatabaseResult<T> = Result<T, RoomDatabaseError>;

pub trait RoomDatabase: Clone + Send + Sync {
    /// This method returns all the public rooms
    async fn get_public_rooms(&self) -> RoomDatabaseResult<Vec<Room>>;

    /// Returns only the rooms in which the user is already joined
    async fn get_user_rooms(&self, user_id: Uuid) -> RoomDatabaseResult<Vec<Room>>;

    /// Return the specific information about only one room
    async fn get_room(&self, id: Uuid) -> RoomDatabaseResult<Room>;

    /// Creates a room
    async fn create_room(&self, room: Room) -> RoomDatabaseResult<()>;

    /// Joins a specific user from a specific room
    async fn create_room_membership(&self, room_member: RoomMember) -> RoomDatabaseResult<()>;

    /// Removes a specific user from a specific room
    async fn delete_room_membership(&self, room_id: Uuid, user_id: Uuid) -> RoomDatabaseResult<()>;

    /// Get's all of the members for n specific room
    async fn get_room_members(&self, room_id: Uuid) -> RoomDatabaseResult<Vec<User>>;

    /// Get's all of the messages for n specific room
    async fn get_room_messages(&self, room_id: Uuid) -> RoomDatabaseResult<Vec<Message>>;
}

#[derive(Debug, Error)]
pub enum RoomDatabaseError {
    #[error("Internal DB error: {0}")]
    InternalDBError(String),
}
