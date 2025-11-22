use amqprs::{
    BasicProperties,
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{BasicPublishArguments, Channel, ConfirmSelectArguments, QueueDeclareArguments},
    connection::{Connection, OpenConnectionArguments},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::use_cases::notification_service::{
    NotificationService, NotificationServiceError, NotificationServiceResult,
    RoomMemberNotification,
};

pub struct RabbitMQ {
    channel: Arc<Mutex<Channel>>,
    _connection: Connection,
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
            match Self::try_connect(host, port, username, password, vhost).await {
                Ok(rabbit) => {
                    info!("RabbitMQ connected and queue 'room_member_notifications' ready");
                    return rabbit;
                }
                Err(_) => {
                    error!("Failed to connect to RabbitMQ, retrying in 3s...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                }
            }
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
            .map_err(|err| error!("Error connecting to RabbitMQ: {err}"))?;

        connection
            .register_callback(DefaultConnectionCallback)
            .await
            .ok();

        let channel = connection.open_channel(None).await.map_err(|_| ())?;
        channel.register_callback(DefaultChannelCallback).await.ok();

        let queue_args = QueueDeclareArguments::durable_client_named("room_member_notifications")
            .durable(true)
            .auto_delete(false)
            .finish();

        channel
            .queue_declare(queue_args)
            .await
            .map_err(|e| error!("Failed to declare queue: {e}"))?;

        channel
            .confirm_select(ConfirmSelectArguments::default())
            .await
            .map_err(|e| error!("Failed to enable confirms: {e}"))?;

        Ok(RabbitMQ {
            channel: Arc::new(Mutex::new(channel)),
            _connection: connection,
        })
    }
}

impl NotificationService for RabbitMQ {
    async fn send_room_member_notification(
        &self,
        message: RoomMemberNotification,
    ) -> NotificationServiceResult<()> {
        let payload = serde_json::to_vec(&message)
            .map_err(|e| NotificationServiceError::MessageProcessingError(e.to_string()))?;

        let args = BasicPublishArguments::new("", "room_member_notifications");
        let props = BasicProperties::default().with_delivery_mode(2).finish();

        self.channel
            .lock()
            .await
            .basic_publish(props, payload, args)
            .await
            .map_err(|e| NotificationServiceError::MessageProcessingError(e.to_string()))?;

        Ok(())
    }
}
