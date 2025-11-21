use mockall::automock;
use thiserror::Error;

use crate::domain::room::Message;

pub type RealTimeBrokerResult<T> = Result<T, RealTimeBrokerError>;

#[automock]
pub trait MessagePublisher: Send + Sync {
    async fn broadcast_message(&self, message: Message) -> RealTimeBrokerResult<()>;
}

#[automock]
pub trait MessageSubscriber: Send + Sync {
    async fn consume_message(&mut self) -> RealTimeBrokerResult<Message>;
}

#[derive(Debug, Error)]
pub enum RealTimeBrokerError {
    #[error("Internal broker error: {0}")]
    InternalBrokerError(String),

    #[error("Broker connection closed")]
    BrokerConnectionClosed,
}
