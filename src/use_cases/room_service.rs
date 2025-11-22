use std::sync::Arc;

use bcrypt::{DEFAULT_COST, hash, verify};
use chrono::Utc;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::{
        room::{MemberRole, Message, Room, RoomMember, RoomVisibility},
        user::User,
    },
    use_cases::{
        notification_service::{NotificationService, RoomMemberNotification},
        realtime_broker::MessagePublisher,
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

pub async fn join_room(
    db: Arc<impl RoomDatabase>,
    room_id: Uuid,
    user_id: Uuid,
    password: Option<String>,
    notification_service: Arc<impl NotificationService>,
) -> RoomResult<()> {
    let room_member = RoomMember {
        room_id,
        user_id,
        role: MemberRole::Member,
        joined_at: Utc::now(),
    };

    let notification = RoomMemberNotification {
        user_id,
        room_id,
        action: super::notification_service::RoomAction::JoinedRoom,
    };

    let room = db
        .get_room(room_id)
        .await
        .map_err(|err| RoomError::DatabaseError(err.to_string()))?;

    if room.visibility == RoomVisibility::Private && password.is_none() {
        return Err(RoomError::PasswordNotGiven);
    }

    if room.visibility == RoomVisibility::Private && password.is_some() {
        let ver = verify(password.unwrap(), &room.password_hash.unwrap())
            .map_err(|err| RoomError::BcryptError(err.to_string()))?;
        if !ver {
            return Err(RoomError::InvalidRoomPassword);
        }
    }

    db.create_room_membership(room_member)
        .await
        .map_err(|err| RoomError::DatabaseError(err.to_string()))?;

    notification_service
        .send_room_member_notification(notification)
        .await
        .map_err(|err| RoomError::NotificationError(err.to_string()))?;

    Ok(())
}

pub async fn leave_room(
    db: Arc<impl RoomDatabase>,
    notification_service: Arc<impl NotificationService>,
    room_id: Uuid,
    user_id: Uuid,
) -> RoomResult<()> {
    let notification = RoomMemberNotification {
        user_id,
        room_id,
        action: super::notification_service::RoomAction::LeftRoom,
    };

    db.delete_room_membership(room_id, user_id)
        .await
        .map_err(|err| RoomError::DatabaseError(err.to_string()))?;

    notification_service
        .send_room_member_notification(notification)
        .await
        .map_err(|err| RoomError::NotificationError(err.to_string()))?;

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

pub async fn obtain_room_members(
    db: Arc<impl RoomDatabase>,
    room_id: Uuid,
) -> RoomResult<Vec<User>> {
    let users = db
        .get_room_members(room_id)
        .await
        .map_err(|err| RoomError::DatabaseError(err.to_string()))?;
    Ok(users)
}
//
//
//
//
//
//
//
//

#[derive(Error, Debug)]
pub enum RoomError {
    #[error("database Error {0}")]
    DatabaseError(String),
    #[error("hashing error {0}")]
    PasswordHashError(String),
    #[error("pasword not given")]
    PasswordNotGiven,
    #[error("broadcast error {0}")]
    BroadcastError(String),
    #[error("notification error: {0}")]
    NotificationError(String),
    #[error("invalid room password")]
    InvalidRoomPassword,
    #[error("encryt error: {0}")]
    BcryptError(String),
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use chrono::Utc;
    use uuid::Uuid;

    use crate::{
        domain::{
            room::{Message, Room, RoomVisibility},
            user::User,
        },
        use_cases::{
            notification_service::{MockNotificationService, NotificationServiceError},
            realtime_broker::MockMessagePublisher,
            room_database::{MockRoomDatabase, RoomDatabaseError},
            room_service::{
                RoomError, create_room, get_all_public_rooms, get_user_rooms_use, join_room,
                leave_room, obtain_messages, obtain_room_members, send_message, user_is_in_room,
            },
        },
    };

    #[tokio::test]
    async fn test_user_is_in_room_true() {
        let mut db = MockRoomDatabase::new();

        let user_id = Uuid::new_v4();
        let room_id = Uuid::new_v4();

        db.expect_get_user_rooms().returning(move |_| {
            Ok(vec![Room {
                id: room_id,
                name: "test".into(),
                visibility: RoomVisibility::Public,
                password_hash: None,
                created_by: user_id,
                created_at: Utc::now(),
            }])
        });

        let result = user_is_in_room(Arc::new(db), user_id, room_id)
            .await
            .unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_user_is_in_room_false() {
        let mut db = MockRoomDatabase::new();

        let user_id = Uuid::new_v4();

        db.expect_get_user_rooms().returning(|_| Ok(vec![]));

        let result = user_is_in_room(Arc::new(db), user_id, Uuid::new_v4())
            .await
            .unwrap();

        assert!(!result);
    }

    #[tokio::test]
    async fn join_public_room_ok() {
        let mut db = MockRoomDatabase::new();
        let mut notif = MockNotificationService::new();

        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        db.expect_get_room().returning(move |_| {
            Ok(Room {
                id: room_id,
                name: "Public".into(),
                visibility: RoomVisibility::Public,
                password_hash: None,
                created_by: user_id,
                created_at: Utc::now(),
            })
        });

        db.expect_create_room_membership().returning(|_| Ok(()));

        notif
            .expect_send_room_member_notification()
            .returning(|_| Ok(()));

        let res = join_room(Arc::new(db), room_id, user_id, None, Arc::new(notif)).await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn join_private_room_without_password_fails() {
        let mut db = MockRoomDatabase::new();
        let notif = MockNotificationService::new();

        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        db.expect_get_room().returning(move |_| {
            Ok(Room {
                id: room_id,
                name: "Private".into(),
                visibility: RoomVisibility::Private,
                password_hash: Some("$2b$12$somehashhere".into()),
                created_by: user_id,
                created_at: Utc::now(),
            })
        });

        let res = join_room(Arc::new(db), room_id, user_id, None, Arc::new(notif)).await;

        assert!(matches!(res, Err(RoomError::PasswordNotGiven)));
    }

    #[tokio::test]
    async fn join_private_room_wrong_password_fails() {
        let mut db = MockRoomDatabase::new();
        let notif = MockNotificationService::new();

        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        // Hash of "correct123"
        let valid_hash = bcrypt::hash("correct123", bcrypt::DEFAULT_COST).unwrap();

        db.expect_get_room().returning(move |_| {
            Ok(Room {
                id: room_id,
                name: "Private".into(),
                visibility: RoomVisibility::Private,
                password_hash: Some(valid_hash.clone()),
                created_by: user_id,
                created_at: Utc::now(),
            })
        });

        let res = join_room(
            Arc::new(db),
            room_id,
            user_id,
            Some("wrongpass".into()),
            Arc::new(notif),
        )
        .await;

        assert!(matches!(res, Err(RoomError::InvalidRoomPassword)));
    }

    #[tokio::test]
    async fn join_private_room_correct_password_ok() {
        let mut db = MockRoomDatabase::new();
        let mut notif = MockNotificationService::new();

        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let hash = bcrypt::hash("mypassword", bcrypt::DEFAULT_COST).unwrap();

        db.expect_get_room().returning(move |_| {
            Ok(Room {
                id: room_id,
                name: "Private".into(),
                visibility: RoomVisibility::Private,
                password_hash: Some(hash.clone()),
                created_by: user_id,
                created_at: Utc::now(),
            })
        });

        db.expect_create_room_membership().returning(|_| Ok(()));

        notif
            .expect_send_room_member_notification()
            .returning(|_| Ok(()));

        let res = join_room(
            Arc::new(db),
            room_id,
            user_id,
            Some("mypassword".into()),
            Arc::new(notif),
        )
        .await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn leave_room_ok() {
        let mut db = MockRoomDatabase::new();
        let mut notif = MockNotificationService::new();

        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        db.expect_delete_room_membership().returning(|_, _| Ok(()));

        notif
            .expect_send_room_member_notification()
            .returning(|_| Ok(()));

        let res = leave_room(Arc::new(db), Arc::new(notif), room_id, user_id).await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn leave_room_delete_fails() {
        let mut db = MockRoomDatabase::new();
        let notif = MockNotificationService::new();

        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        db.expect_delete_room_membership()
            .returning(|_, _| Err(RoomDatabaseError::InternalDBError("db error".into())));

        let res = leave_room(Arc::new(db), Arc::new(notif), room_id, user_id).await;

        assert!(matches!(res, Err(RoomError::DatabaseError(_))));
    }

    #[tokio::test]
    async fn leave_room_notification_fails() {
        let mut db = MockRoomDatabase::new();
        let mut notif = MockNotificationService::new();

        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        db.expect_delete_room_membership().returning(|_, _| Ok(()));

        notif.expect_send_room_member_notification().returning(|_| {
            Err(NotificationServiceError::MessageProcessingError(
                "notif error".into(),
            ))
        });

        let res = leave_room(Arc::new(db), Arc::new(notif), room_id, user_id).await;

        assert!(matches!(res, Err(RoomError::NotificationError(_))));
    }

    #[tokio::test]
    async fn test_create_room_private_with_password() {
        let mut db = MockRoomDatabase::new();
        let user_id = Uuid::new_v4();

        db.expect_create_room().returning(|_| Ok(()));

        db.expect_create_room_membership().returning(|_| Ok(()));

        let res = create_room(
            Arc::new(db),
            RoomVisibility::Private,
            Some("1234".into()),
            "My Room".into(),
            user_id,
        )
        .await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_create_room_private_missing_password() {
        let db = MockRoomDatabase::new();

        let user_id = Uuid::new_v4();

        let res = create_room(
            Arc::new(db),
            RoomVisibility::Private,
            None,
            "My Room".into(),
            user_id,
        )
        .await;

        assert!(matches!(res, Err(RoomError::PasswordNotGiven)));
    }

    #[tokio::test]
    async fn test_get_user_rooms() {
        let mut db = MockRoomDatabase::new();
        let user_id = Uuid::new_v4();

        db.expect_get_user_rooms().returning(move |_| {
            Ok(vec![Room {
                id: Uuid::new_v4(),
                name: "Test".into(),
                visibility: RoomVisibility::Public,
                password_hash: None,
                created_by: user_id.clone(),
                created_at: Utc::now(),
            }])
        });

        let res = get_user_rooms_use(Arc::new(db), user_id.clone())
            .await
            .unwrap();

        assert_eq!(res.len(), 1);
    }

    #[tokio::test]
    async fn test_get_all_public_rooms() {
        let mut db = MockRoomDatabase::new();

        db.expect_get_public_rooms().returning(|| {
            Ok(vec![Room {
                id: Uuid::new_v4(),
                name: "Public Room".into(),
                visibility: RoomVisibility::Public,
                password_hash: None,
                created_by: Uuid::new_v4(),
                created_at: Utc::now(),
            }])
        });

        let rooms = get_all_public_rooms(Arc::new(db)).await.unwrap();

        assert_eq!(rooms.len(), 1);
    }

    #[tokio::test]
    async fn test_obtain_messages() {
        let mut db = MockRoomDatabase::new();
        let room_id = Uuid::new_v4();

        db.expect_get_room_messages().returning(move |_, _, _| {
            Ok(vec![Message {
                id: Uuid::new_v4(),
                room_id: room_id.clone(),
                sender_id: Uuid::new_v4(),
                content: "msg".into(),
                created_at: Utc::now(),
            }])
        });

        let msgs = obtain_messages(Arc::new(db), 1, 10, room_id).await.unwrap();

        assert_eq!(msgs.len(), 1);
    }

    #[tokio::test]
    async fn test_obtain_room_members() {
        let mut db = MockRoomDatabase::new();
        let room_id = Uuid::new_v4();

        db.expect_get_room_members().returning(|_| {
            Ok(vec![User {
                id: Uuid::new_v4(),
                username: "john".into(),
                email: "john@example.com".into(),
                password_hash: "hash".into(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }])
        });

        let members = obtain_room_members(Arc::new(db), room_id).await.unwrap();

        assert_eq!(members.len(), 1);
    }

    #[tokio::test]
    async fn test_send_message_success() {
        let mut db = MockRoomDatabase::new();
        let mut publisher = MockMessagePublisher::new();

        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let content = "Hello World!".to_string();

        // Expect DB create_message to succeed
        db.expect_create_message().returning(|_| Ok(()));

        // Expect publisher broadcast_message to succeed
        publisher.expect_broadcast_message().returning(|_| Ok(()));

        let result =
            send_message(Arc::new(db), room_id, user_id, content, Arc::new(publisher)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_message_db_fail() {
        let mut db = MockRoomDatabase::new();
        let publisher = MockMessagePublisher::new();

        db.expect_create_message()
            .returning(|_| Err(RoomDatabaseError::InternalDBError("db error".into())));

        let result = send_message(
            Arc::new(db),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "hi".into(),
            Arc::new(publisher),
        )
        .await;

        assert!(matches!(result, Err(RoomError::DatabaseError(_))));
    }

    #[tokio::test]
    async fn test_send_message_broadcast_fail() {
        let mut db = MockRoomDatabase::new();
        let mut publisher = MockMessagePublisher::new();

        db.expect_create_message().returning(|_| Ok(()));

        publisher.expect_broadcast_message().returning(|_| {
            Err(
                crate::use_cases::realtime_broker::RealTimeBrokerError::InternalBrokerError(
                    "broadcast err".into(),
                ),
            )
        });

        let result = send_message(
            Arc::new(db),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "hello".into(),
            Arc::new(publisher),
        )
        .await;

        assert!(matches!(result, Err(RoomError::BroadcastError(_))));
    }
}
