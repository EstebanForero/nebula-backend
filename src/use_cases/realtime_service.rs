use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::broadcast;
use tracing::{error, info};
use uuid::Uuid;

use crate::{domain::room::Message, use_cases::realtime_broker::MessageSubscriber};

pub async fn realtime_messsage_broker(
    mut messageSubscriber: impl MessageSubscriber,
    rooms_channels: Arc<DashMap<Uuid, broadcast::Sender<Message>>>,
) {
    while let Ok(message) = messageSubscriber.consume_message().await {
        let channel = if let Some(channel) = rooms_channels.get(&message.room_id) {
            channel
        } else {
            info!("Clients to broadcast messages to, were not found");
            continue;
        };

        match channel.send(message) {
            Ok(n_receivers) => info!("There are {n_receivers}, that will receive the message"),
            Err(err) => error!("{}", err),
        };
    }
}
