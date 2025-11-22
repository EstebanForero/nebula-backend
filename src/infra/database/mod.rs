use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, migrate::Migrator, postgres::PgPoolOptions};
use uuid::Uuid;

use crate::{
    domain::{
        room::{Message, Room, RoomMember, RoomVisibility},
        user::User,
    },
    use_cases::{
        room_database::{RoomDatabase, RoomDatabaseError, RoomDatabaseResult},
        user_database::{UserDatabase, UserDatabaseError, UserDatabaseResult},
    },
};

#[derive(Clone)]
pub struct PostgresDatabase {
    pool: PgPool,
}

impl PostgresDatabase {
    pub async fn new(db_url: &str) -> Result<PostgresDatabase> {
        let pool = PgPoolOptions::new().connect(db_url).await?;

        Migrator::new(std::path::Path::new("./migrations"))
            .await?
            .run(&pool)
            .await
            .context("Failed to connect to postgres")?;

        Ok(PostgresDatabase { pool })
    }
}

impl UserDatabase for PostgresDatabase {
    async fn create_user(&self, user: User) -> UserDatabaseResult<()> {
        sqlx::query!(
            "INSERT INTO users (id, username, email, password_hash) VALUES ($1, $2, $3, $4)",
            user.id,
            user.username,
            user.email,
            user.password_hash
        )
        .execute(&self.pool)
        .await
        .map_err(|err| UserDatabaseError::InternalDBError(err.to_string()))
        .map(|_| ())
    }

    async fn get_user_by_username(&self, username: String) -> UserDatabaseResult<User> {
        sqlx::query_as!(User, "SELECT * FROM users WHERE username = $1", username)
            .fetch_one(&self.pool)
            .await
            .map_err(|err| UserDatabaseError::InternalDBError(err.to_string()))
    }

    async fn get_user_by_email(&self, email: String) -> UserDatabaseResult<User> {
        sqlx::query_as!(User, "SELECT * FROM users WHERE email = $1", email)
            .fetch_one(&self.pool)
            .await
            .map_err(|err| UserDatabaseError::InternalDBError(err.to_string()))
    }

    async fn get_user_by_id(&self, id: Uuid) -> UserDatabaseResult<User> {
        sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", id)
            .fetch_one(&self.pool)
            .await
            .map_err(|err| UserDatabaseError::InternalDBError(err.to_string()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbRoom {
    pub id: Uuid,
    pub name: String,
    pub visibility: Option<String>,
    pub password_hash: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

impl TryInto<Room> for DbRoom {
    type Error = RoomDatabaseError;

    fn try_into(self) -> std::result::Result<Room, Self::Error> {
        let visibility = match self.visibility {
            Some(visibility_string) => match visibility_string.as_str() {
                "public" => Ok(RoomVisibility::Public),
                "private" => Ok(RoomVisibility::Private),
                _ => Err(RoomDatabaseError::InternalDBError(format!(
                    "{visibility_string}: is not public nor private, error deserializing in the db"
                ))),
            },
            None => Err(RoomDatabaseError::InternalDBError(
                "visibility doesn't contain any string, database error".to_string(),
            )),
        }?;

        Ok(Room {
            id: self.id,
            name: self.name,
            visibility,
            password_hash: self.password_hash,
            created_by: self.created_by,
            created_at: self.created_at,
        })
    }
}

fn rooms_db_to_rooms(rooms_db: Vec<DbRoom>) -> RoomDatabaseResult<Vec<Room>> {
    let mut rooms = Vec::new();

    for room_db in rooms_db {
        let room = room_db.try_into()?;
        rooms.push(room);
    }

    Ok(rooms)
}

impl RoomDatabase for PostgresDatabase {
    async fn get_public_rooms(&self) -> RoomDatabaseResult<Vec<Room>> {
        let rooms_db = sqlx::query_as!(
            DbRoom,
            "SELECT id, name, visibility::text, password_hash, created_by, created_at FROM rooms WHERE visibility = 'public' ORDER BY created_at DESC"
        ).fetch_all(&self.pool).await
            .map_err(|err| RoomDatabaseError::InternalDBError(err.to_string()))?;

        rooms_db_to_rooms(rooms_db)
    }

    async fn get_user_rooms(&self, user_id: Uuid) -> RoomDatabaseResult<Vec<Room>> {
        let rooms_db = sqlx::query_as!(
            DbRoom,
            "SELECT id, name, visibility::text, password_hash, created_by, created_at FROM rooms WHERE id = (SELECT room_id FROM room_members WHERE user_id = $1) ORDER BY created_at DESC",
            user_id
        ).fetch_all(&self.pool).await
            .map_err(|err| RoomDatabaseError::InternalDBError(err.to_string()))?;

        rooms_db_to_rooms(rooms_db)
    }

    async fn get_room(&self, id: Uuid) -> RoomDatabaseResult<Room> {
        let room_db = sqlx::query_as!(
            DbRoom,
            "SELECT id, name, visibility::text, password_hash, created_by, created_at FROM rooms WHERE id = $1",
            id
        ).fetch_one(&self.pool).await
            .map_err(|err| RoomDatabaseError::InternalDBError(err.to_string()))?;

        room_db.try_into()
    }

    async fn create_room(&self, room: Room) -> RoomDatabaseResult<()> {
        sqlx::query!(
            "INSERT INTO rooms (id, name, visibility, password_hash, created_by, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
            room.id,
            room.name,
            room.visibility.to_string(),
            room.password_hash,
            room.created_by,
            room.created_at
        )
        .execute(&self.pool)
        .await
        .map_err(|err| RoomDatabaseError::InternalDBError(err.to_string()))?;

        Ok(())
    }

    async fn create_room_membership(&self, room_member: RoomMember) -> RoomDatabaseResult<()> {
        sqlx::query!(
            "INSERT INTO room_members (room_id, user_id, role, joined_at) VALUES ($1, $2, $3, $4)",
            room_member.room_id,
            room_member.user_id,
            room_member.role.to_string(),
            room_member.joined_at
        )
        .execute(&self.pool)
        .await
        .map_err(|err| RoomDatabaseError::InternalDBError(err.to_string()))?;

        Ok(())
    }

    async fn delete_room_membership(&self, room_id: Uuid, user_id: Uuid) -> RoomDatabaseResult<()> {
        sqlx::query!(
            "DELETE FROM room_members WHERE room_id = $1 AND user_id = $2",
            room_id,
            user_id
        )
        .execute(&self.pool)
        .await
        .map_err(|err| RoomDatabaseError::InternalDBError(err.to_string()))?;

        Ok(())
    }

    async fn get_room_members(&self, room_id: Uuid) -> RoomDatabaseResult<Vec<User>> {
        let users = sqlx::query_as!(
            User,
            "SELECT * FROM users WHERE id = (SELECT user_id FROM room_members WHERE room_id = $1)",
            room_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|err| RoomDatabaseError::InternalDBError(err.to_string()))?;

        Ok(users)
    }

    async fn create_message(&self, message: Message) -> RoomDatabaseResult<()> {
        sqlx::query!(
            "INSERT INTO messages (id, room_id, sender_id, content) VALUES ($1, $2, $3, $4)",
            message.id,
            message.room_id,
            message.sender_id,
            message.content
        )
        .execute(&self.pool)
        .await
        .map_err(|err| RoomDatabaseError::InternalDBError(err.to_string()))?;

        Ok(())
    }

    async fn get_room_messages(&self, room_id: Uuid) -> RoomDatabaseResult<Vec<Message>> {
        let messages = sqlx::query_as!(
            Message,
            "SELECT * FROM messages WHERE room_id = $1 ORDER BY created_at DESC",
            room_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|err| RoomDatabaseError::InternalDBError(err.to_string()))?;

        Ok(messages)
    }
}
