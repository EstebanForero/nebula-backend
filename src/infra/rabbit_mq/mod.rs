use crate::{
    domain::room::Message,
    use_cases::message_processing::{
        MessageProcessing, MessageProcessingError, MessageProcessingResult,
    },
};
use amqprs::{
    BasicProperties,
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{BasicPublishArguments, Channel, ConfirmSelectArguments},
    connection::{self, Connection, OpenConnectionArguments},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::error;

pub struct RabbitMQ {
    channel: Arc<Mutex<Channel>>,
    _connection: connection,
}

impl RabbitMQ {
    pub async fn new(
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        vhost: &str,
    ) -> RabbitMQ {
        loop {
            if let Ok(rabbit) = Self::try_connect(host, port, username, password, vhost).await {
                return rabbit;
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        }
    }

    async fn try_connect(
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        vhost: &str,
    ) -> Result<RabbitMQ, ()> {
        let mut args = OpenConnectionArguments::new(host, port, username, password);
        args.virtual_host(vhost).heartbeat(60);

        let connection = Connection::open(&args)
            .await
            .map_err(|err| error!("Error connecting rabbit mq: {err}"))?;
        connection
            .register_callback(DefaultConnectionCallback)
            .await
            .ok();

        let channel = connection.open_channel(None).await.map_err(|_| ())?;
        channel.register_callback(DefaultChannelCallback).await.ok();

        let _ = channel
            .confirm_select(ConfirmSelectArguments::default())
            .await;

        Ok(RabbitMQ {
            channel: Arc::new(Mutex::new(channel)),
        })
    }
}

impl MessageProcessing for RabbitMQ {
    async fn enqueue_message(&self, message: Message) -> MessageProcessingResult<()> {
        let payload = serde_json::to_vec(&message)
            .map_err(|e| MessageProcessingError::MessageProcessingError(e.to_string()))?;

        let publish_args = BasicPublishArguments::new("", "chat_messages");

        let properties = BasicProperties::default().with_delivery_mode(2).finish();

        self.channel
            .lock()
            .await
            .basic_publish(properties, payload, publish_args)
            .await
            .map_err(|e| MessageProcessingError::MessageProcessingError(e.to_string()))
    }
}
