use mockall::automock;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub type NotificationServiceResult<T> = Result<T, NotificationServiceError>;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RoomAction {
    JoinedRoom,
    LeftRoom,
}

#[derive(Deserialize, Serialize)]
pub struct RoomMemberNotification {
    pub user_id: Uuid,
    pub room_id: Uuid,
    pub action: RoomAction,
}

#[automock]
pub trait NotificationService: Send + Sync {
    async fn send_room_member_notification(
        &self,
        message: RoomMemberNotification,
    ) -> NotificationServiceResult<()>;
}

#[derive(Debug, Error)]
pub enum NotificationServiceError {
    #[error("Internal broker error: {0}")]
    MessageProcessingError(String),
}
