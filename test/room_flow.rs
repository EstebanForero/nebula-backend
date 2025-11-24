use std::{sync::Arc, time::Duration};

use futures::StreamExt;
use jsonwebtoken::{DecodingKey, Validation, decode};
use nebula_backend::{
    domain::{
        room::{Message, RoomVisibility},
    },
    infra::redis::RedisPublisher,
    use_cases::{
        auth_service::{Claims, login, register},
        room_service::{create_room, get_all_public_rooms, get_user_rooms_use, join_room, leave_room, obtain_messages, send_message},
        user_database::UserDatabase,
    },
};
use tokio::time::timeout;
use uuid::Uuid;
use nebula_backend::use_cases::{
    notification_service::MockNotificationService,
    realtime_broker::MockMessagePublisher,
};
use nebula_backend::use_cases::room_service::RoomError;

#[path = "common/mod.rs"]
mod common;

async fn login_and_get_id<T: UserDatabase + Send + Sync + 'static>(
    db: Arc<T>,
    identifier: String,
    password: String,
    jwt_secret: &str,
) -> Uuid {
    let token = login(db, identifier, password, jwt_secret.to_string())
        .await
        .expect("login should succeed");

    let claims: Claims = decode(
        &token,
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &Validation::default(),
    )
    .expect("jwt should decode")
    .claims;

    Uuid::parse_str(&claims.sub).expect("sub should be uuid")
}

#[tokio::test]
#[serial]
async fn room_lifecycle_persists_and_broadcasts_messages() {
    let config = common::IntegrationConfig::load();
    let (database, pool) = common::provision_database(&config).await;
    common::flush_redis(&config.redis_url).await;

    let username = format!("integration-room-owner-{}", Uuid::new_v4().simple());
    let email = format!("{username}@example.com");
    let password = "Password123*".to_string();

    register(Arc::new(database.clone()), username.clone(), password.clone(), email.clone())
    .await
    .expect("user registration should succeed");

    let owner_id =
        login_and_get_id(Arc::new(database.clone()), username.clone(), password.clone(), &config.jwt_secret).await;

    create_room(Arc::new(database.clone()), RoomVisibility::Public, None, "integration-room".to_string(), owner_id)
    .await
    .expect("room creation should persist to postgres");

    let rooms = get_user_rooms_use(Arc::new(database.clone()), owner_id)
        .await
        .expect("owner should have one room");

    let room_id = rooms
        .first()
        .map(|room| room.id)
        .expect("room id should be available");

    // Prepare Redis subscriber before sending the message to avoid missing the publish.
    let client = redis::Client::open(config.redis_url.clone())
        .expect("failed to create redis client for subscription");
    let mut pubsub = client
        .get_async_pubsub()
        .await
        .expect("failed to open redis pubsub");
    pubsub
        .subscribe("chat:messages")
        .await
        .expect("failed to subscribe to chat messages channel");
    let mut message_stream = pubsub.into_on_message();

    let publisher = Arc::new(RedisPublisher::new(&config.redis_url).await);
    let content = "hello from integration";

    let listener = tokio::spawn(async move {
        message_stream
            .next()
            .await
            .and_then(|msg| msg.get_payload::<String>().ok())
    });

    send_message(Arc::new(database.clone()), room_id, owner_id, content.to_string(), publisher)
    .await
    .expect("message should be stored and published");

    let payload = timeout(Duration::from_secs(5), listener)
        .await
        .expect("redis publish should arrive in time")
        .expect("listener task should complete")
        .expect("redis payload should parse into string");

    let broadcasted: Message =
        serde_json::from_str(&payload).expect("redis payload should deserialize into Message");
    assert_eq!(broadcasted.room_id, room_id);
    assert_eq!(broadcasted.sender_id, owner_id);
    assert_eq!(broadcasted.content, content);

    let stored: Vec<Message> = sqlx::query_as::<_, Message>(
        "SELECT id, room_id, sender_id, content, created_at FROM messages WHERE room_id = $1",
    )
    .bind(room_id)
    .fetch_all(&pool)
    .await
    .expect("message should be stored in postgres");

    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].content, content);

    common::reset_tables(&pool).await;
}

#[tokio::test]
#[serial]
async fn private_room_requires_password() {
    let config = common::IntegrationConfig::load();
    let (database, pool) = common::provision_database(&config).await;

    let username = format!("private-no-pass-{}", Uuid::new_v4().simple());
    let email = format!("{username}@example.com");
    let password = "Password123*".to_string();

    register(Arc::new(database.clone()), username.clone(), password.clone(), email.clone())
        .await
        .expect("user registration should succeed");

    let owner_id =
        login_and_get_id(Arc::new(database.clone()), username.clone(), password.clone(), &config.jwt_secret).await;

    let res = create_room(
        Arc::new(database.clone()),
        RoomVisibility::Private,
        None,
        "no-pass-room".to_string(),
        owner_id,
    )
    .await;

    assert!(matches!(res, Err(RoomError::PasswordNotGiven)));

    common::reset_tables(&pool).await;
}

#[tokio::test]
#[serial]
async fn join_private_room_with_wrong_password_fails() {
    let config = common::IntegrationConfig::load();
    let (database, pool) = common::provision_database(&config).await;

    let owner_name = format!("private-owner-{}", Uuid::new_v4().simple());
    let owner_email = format!("{owner_name}@example.com");
    let password = "Password123*".to_string();
    register(Arc::new(database.clone()), owner_name.clone(), password.clone(), owner_email.clone())
        .await
        .expect("owner registration should succeed");
    let owner_id =
        login_and_get_id(Arc::new(database.clone()), owner_name.clone(), password.clone(), &config.jwt_secret).await;

    create_room(
        Arc::new(database.clone()),
        RoomVisibility::Private,
        Some("roomsecret".to_string()),
        "private-room".to_string(),
        owner_id,
    )
    .await
    .expect("room creation should succeed");

    let joiner_name = format!("joiner-{}", Uuid::new_v4().simple());
    let joiner_email = format!("{joiner_name}@example.com");
    register(
        Arc::new(database.clone()),
        joiner_name.clone(),
        password.clone(),
        joiner_email.clone(),
    )
    .await
    .expect("joiner registration should succeed");
    let joiner_id =
        login_and_get_id(Arc::new(database.clone()), joiner_name.clone(), password.clone(), &config.jwt_secret).await;

    let notif = MockNotificationService::new();
    let room_id = get_user_rooms_use(Arc::new(database.clone()), owner_id)
        .await
        .unwrap()
        .first()
        .unwrap()
        .id;

    let result = join_room(
        Arc::new(database.clone()),
        room_id,
        joiner_id,
        Some("wrongpass".into()),
        Arc::new(notif),
    )
    .await;

    assert!(matches!(result, Err(RoomError::InvalidRoomPassword)));

    common::reset_tables(&pool).await;
}

#[tokio::test]
#[serial]
async fn join_and_leave_private_room_succeeds() {
    let config = common::IntegrationConfig::load();
    let (database, pool) = common::provision_database(&config).await;

    let owner_name = format!("private-owner-{}", Uuid::new_v4().simple());
    let owner_email = format!("{owner_name}@example.com");
    let password = "Password123*".to_string();
    register(Arc::new(database.clone()), owner_name.clone(), password.clone(), owner_email.clone())
        .await
        .expect("owner registration should succeed");
    let owner_id =
        login_and_get_id(Arc::new(database.clone()), owner_name.clone(), password.clone(), &config.jwt_secret).await;

    create_room(
        Arc::new(database.clone()),
        RoomVisibility::Private,
        Some("roomsecret".to_string()),
        "private-room".to_string(),
        owner_id,
    )
    .await
    .expect("room creation should succeed");

    let joiner_name = format!("joiner-{}", Uuid::new_v4().simple());
    let joiner_email = format!("{joiner_name}@example.com");
    register(
        Arc::new(database.clone()),
        joiner_name.clone(),
        password.clone(),
        joiner_email.clone(),
    )
    .await
    .expect("joiner registration should succeed");
    let joiner_id =
        login_and_get_id(Arc::new(database.clone()), joiner_name.clone(), password.clone(), &config.jwt_secret).await;

    let mut notif = MockNotificationService::new();
    notif
        .expect_send_room_member_notification()
        .returning(|_| Ok(()));

    let room_id = get_user_rooms_use(Arc::new(database.clone()), owner_id)
        .await
        .unwrap()
        .first()
        .unwrap()
        .id;

    join_room(
        Arc::new(database.clone()),
        room_id,
        joiner_id,
        Some("roomsecret".into()),
        Arc::new(notif),
    )
    .await
    .expect("join should work");

    let mut notif_leave = MockNotificationService::new();
    notif_leave
        .expect_send_room_member_notification()
        .returning(|_| Ok(()));

    leave_room(
        Arc::new(database.clone()),
        Arc::new(notif_leave),
        room_id,
        joiner_id,
    )
    .await
    .expect("leave should work");

    common::reset_tables(&pool).await;
}

#[tokio::test]
#[serial]
async fn get_all_public_rooms_lists_created_room() {
    let config = common::IntegrationConfig::load();
    let (database, pool) = common::provision_database(&config).await;

    let username = format!("public-owner-{}", Uuid::new_v4().simple());
    let email = format!("{username}@example.com");
    let password = "Password123*".to_string();
    register(Arc::new(database.clone()), username.clone(), password.clone(), email.clone())
        .await
        .expect("user registration should succeed");
    let owner_id =
        login_and_get_id(Arc::new(database.clone()), username.clone(), password.clone(), &config.jwt_secret).await;

    create_room(
        Arc::new(database.clone()),
        RoomVisibility::Public,
        None,
        "public-room".to_string(),
        owner_id,
    )
    .await
    .expect("room creation should succeed");

    let rooms = get_all_public_rooms(Arc::new(database.clone()))
        .await
        .expect("should list public rooms");

    assert_eq!(rooms.len(), 1);
    assert_eq!(rooms[0].name, "public-room");

    common::reset_tables(&pool).await;
}

#[tokio::test]
#[serial]
async fn obtain_messages_respects_pagination() {
    let config = common::IntegrationConfig::load();
    let (database, pool) = common::provision_database(&config).await;

    let username = format!("pagination-owner-{}", Uuid::new_v4().simple());
    let email = format!("{username}@example.com");
    let password = "Password123*".to_string();
    register(Arc::new(database.clone()), username.clone(), password.clone(), email.clone())
        .await
        .expect("user registration should succeed");
    let owner_id =
        login_and_get_id(Arc::new(database.clone()), username.clone(), password.clone(), &config.jwt_secret).await;

    create_room(
        Arc::new(database.clone()),
        RoomVisibility::Public,
        None,
        "pagination-room".to_string(),
        owner_id,
    )
    .await
    .expect("room creation should succeed");

    let room_id = get_user_rooms_use(Arc::new(database.clone()), owner_id)
        .await
        .unwrap()
        .first()
        .unwrap()
        .id;

    let mut publisher = MockMessagePublisher::new();
    publisher
        .expect_broadcast_message()
        .times(15)
        .returning(|_| Ok(()));
    let publisher = Arc::new(publisher);

    for idx in 0..15 {
        send_message(
            Arc::new(database.clone()),
            room_id,
            owner_id,
            format!("msg-{idx}"),
            publisher.clone(),
        )
        .await
        .expect("message should be stored");
    }

    let page_one = obtain_messages(Arc::new(database.clone()), 1, 10, room_id)
        .await
        .expect("page one should succeed");
    let page_two = obtain_messages(Arc::new(database.clone()), 2, 10, room_id)
        .await
        .expect("page two should succeed");

    assert_eq!(page_one.len(), 10);
    assert_eq!(page_two.len(), 5);

    common::reset_tables(&pool).await;
}
use serial_test::serial;
