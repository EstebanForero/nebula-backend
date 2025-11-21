use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    connection::{Connection, OpenConnectionArguments},
};

struct RabbitMQ {}

impl RabbitMQ {
    pub async fn new(host: &str, port: u16, username: &str, password: &str) -> RabbitMQ {
        let mut args = OpenConnectionArguments::new("", port, username, password);
        args.heartbeat(60);

        let connection = Connection::open(&args)
            .await
            .expect("Error connecting to RabbitMQ");

        connection
            .register_callback(DefaultConnectionCallback)
            .await
            .unwrap();

        // let channel = connection.open_channel(None).await?;
        // channel.register_callback(DefaultChannelCallback).await?;

        // channel.confirm_select(args)

        todo!()
    }
}
