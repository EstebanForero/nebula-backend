use deadpool_redis::{Config, Pool};
use futures::{Stream, StreamExt};

use redis::{AsyncCommands, Msg, aio::PubSubStream};

use crate::{
    domain::room::Message,
    use_cases::realtime_broker::{
        MessagePublisher, MessageSubscriber, RealTimeBrokerError, RealTimeBrokerResult,
    },
};

pub struct RedisPublisher {
    pool: Pool,
}

impl RedisPublisher {
    pub async fn new(redis_url: &str) -> RedisPublisher {
        let cfg = Config::from_url(redis_url);

        let pool = cfg
            .create_pool(Some(deadpool_redis::Runtime::Tokio1))
            .unwrap();

        RedisPublisher { pool }
    }
}

impl MessagePublisher for RedisPublisher {
    async fn broadcast_message(&self, message: Message) -> RealTimeBrokerResult<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|err| RealTimeBrokerError::InternalBrokerError(err.to_string()))?;

        let message_str = serde_json::to_string(&message)
            .map_err(|err| RealTimeBrokerError::InternalBrokerError(err.to_string()))?;

        let _: i64 = conn
            .publish("chat:messages", message_str)
            .await
            .map_err(|err| RealTimeBrokerError::InternalBrokerError(err.to_string()))?;

        Ok(())
    }
}

pub struct RedisConsumer {
    pubsubstream: PubSubStream,
}

pub enum RedisChannel {
    ChatMessages,
}

impl RedisConsumer {
    pub async fn new(redis_url: &str, redis_channel: RedisChannel) -> RedisConsumer {
        let client = redis::Client::open(redis_url).expect("Error, redis connection failed");

        let mut pubsub = client
            .get_async_pubsub()
            .await
            .expect("Error creating pubsub async for redis");

        match redis_channel {
            RedisChannel::ChatMessages => pubsub.subscribe("chat:messages"),
        };

        let msgs: PubSubStream = pubsub.into_on_message();

        RedisConsumer { pubsubstream: msgs }
    }
}

impl MessageSubscriber for RedisConsumer {
    async fn consume_message(&mut self) -> RealTimeBrokerResult<Message> {
        let msg = self
            .pubsubstream
            .next()
            .await
            .ok_or(RealTimeBrokerError::BrokerConnectionClosed)?;

        let message_str: String = msg
            .get_payload()
            .map_err(|err| RealTimeBrokerError::InternalBrokerError(err.to_string()))?;

        let message: Message = serde_json::from_str(&message_str)
            .map_err(|err| RealTimeBrokerError::InternalBrokerError(err.to_string()))?;

        Ok(message)
    }
}
