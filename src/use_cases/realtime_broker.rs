use mockall::automock;
use thiserror::Error;

use crate::domain::room::Message;

pub type RealTimeBrokerResult<T> = Result<T, RealTimeBrokerError>;

#[automock]
trait RealTimeBroker: Send + Sync {
    async fn broadcast_message(&self, message: Message) -> RealTimeBrokerResult<()>;

    async fn consume_messages(&self) -> RealTimeBrokerResult<Message>;
}

#[derive(Debug, Error)]
pub enum RealTimeBrokerError {
    #[error("Internal broker error: {0}")]
    InternalBrokerError(String),
}
