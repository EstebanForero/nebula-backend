use std::{sync::Arc, time::Duration};

use futures::StreamExt;
use nebula_backend::{
    domain::{
        room::{Message, RoomVisibility},
    },
    infra::redis::RedisPublisher,
    use_cases::{
        auth_service::register,
        room_service::{create_room, get_user_rooms_use, send_message},
        user_database::UserDatabase,
    },
};
use tokio::time::timeout;
use uuid::Uuid;

#[path = "common/mod.rs"]
mod common;

#[tokio::test]
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

    let owner = database
        .get_user_by_username(username.clone())
        .await
        .expect("owner should exist after registration");

    create_room(Arc::new(database.clone()), RoomVisibility::Public, None, "integration-room".to_string(), owner.id)
    .await
    .expect("room creation should persist to postgres");

    let rooms = get_user_rooms_use(Arc::new(database.clone()), owner.id)
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

    send_message(Arc::new(database.clone()), room_id, owner.id, content.to_string(), publisher)
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
    assert_eq!(broadcasted.sender_id, owner.id);
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
