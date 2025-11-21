use mockall::automock;
use thiserror::Error;

use crate::domain::room::Message;

pub type MessageProcessingResult<T> = Result<T, MessageProcessingError>;

#[automock]
pub trait MessageProcessing: Send + Sync {
    /// enqueues message for any kind of future processing or analitics
    async fn enqueue_message(&self, message: Message) -> MessageProcessingResult<()>;
}

#[derive(Debug, Error)]
pub enum MessageProcessingError {
    #[error("Internal broker error: {0}")]
    MessageProcessingError(String),
}
