use anyhow::Result;
use lapin::{
    BasicProperties, Channel, Connection, ConnectionProperties,
    options::{BasicPublishOptions, QueueDeclareOptions},
    types::FieldTable,
};
use serde::Serialize;
use utoipa::{IntoParams, ToSchema};

/// Producer encapsulates a RabbitMQ Queue producer instance using lapin
pub struct Producer {
    channel: Channel,
    queue_name: String,
}

/// OrderMessage defines the payload structure for publishing orders
#[derive(Serialize, IntoParams)]
pub struct OrderMessage {
    pub order_id: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Producer {
    /// Initialize the AMQP connection, open a channel, and declare the queue
    pub async fn init() -> Result<Self> {
        let host = std::env::var("RABBITMQ_HOST").unwrap_or_else(|_| "localhost".into());
        let port: u16 = std::env::var("RABBITMQ_PORT")
            .unwrap_or_else(|_| "5672".into())
            .parse()?;
        let user = std::env::var("RABBITMQ_USER").unwrap_or_else(|_| "user".into());
        let pass = std::env::var("RABBITMQ_PASS").unwrap_or_else(|_| "pass".into());

        let addr = format!("amqp://{}:{}@{}:{}/%2f", user, pass, host, port);
        // Establish connection
        let conn = Connection::connect(&addr, ConnectionProperties::default()).await?;
        // Open a channel
        let channel = conn.create_channel().await?;

        // Enable publisher confirms
        channel.confirm_select(Default::default()).await?;

        // Declare a durable queue named "order.placed"
        let queue = "order.placed";
        channel
            .queue_declare(
                queue,
                QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await?;

        Ok(Producer {
            channel,
            queue_name: queue.to_string(),
        })
    }

    /// Publish an OrderMessage to the RabbitMQ queue, awaiting confirmation
    pub async fn publish(&self, order: OrderMessage) -> Result<()> {
        let payload = serde_json::to_vec(&order)?;
        // Publish to default exchange with routing key = queue name
        let confirm = self
            .channel
            .basic_publish(
                "",
                &self.queue_name,
                BasicPublishOptions::default(),
                &payload,
                BasicProperties::default(),
            )
            .await?;
        // Wait for confirmation
        confirm.await?;
        Ok(())
    }
}

/// QueueLength represents the JSON response for queue length API
#[derive(serde::Serialize, ToSchema)]
pub struct QueueLength {
    pub pending_coffee_orders: u32,
}

/// Fetch the current number of pending messages in the 'order.placed' queue via the RabbitMQ Management API
pub async fn fetch_queue_length() -> Result<u32> {
    let protocol = std::env::var("RABBITMQ_MGMT_PROTOCOL").unwrap_or_else(|_| "http".into());
    let host = std::env::var("RABBITMQ_MGMT_HOST").unwrap_or_else(|_| "localhost".into());
    let port: u16 = std::env::var("RABBITMQ_MGMT_PORT")
        .unwrap_or_else(|_| "15672".into())
        .parse()?;
    let user = std::env::var("RABBITMQ_USER").unwrap_or_else(|_| "user".into());
    let pass = std::env::var("RABBITMQ_PASS").unwrap_or_else(|_| "pass".into());

    let mgmt_url = format!("{}://{}:{}", protocol, host, port);
    let url = format!("{}/api/queues/%2F/order.placed", mgmt_url);

    let resp = reqwest::Client::new()
        .get(&url)
        .basic_auth(user, Some(pass))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    Ok(resp["messages_ready"].as_u64().unwrap_or(0) as u32)
}
