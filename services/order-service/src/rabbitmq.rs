use anyhow::Result;
use rabbitmq_stream_client::{
    Environment, NoDedup,
    error::StreamCreateError,
    types::{ByteCapacity, Message, ResponseCode},
};
use serde::Serialize;
use serde_json;
use utoipa::{IntoParams, ToSchema};

// Producer encapsulates a RabbitMQ Stream producer instance
pub struct Producer {
    inner: rabbitmq_stream_client::Producer<NoDedup>,
}

// OrderMessage defines the payload structure for publishing orders
#[derive(Serialize, IntoParams)]
pub struct OrderMessage {
    pub order_id: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Producer {
    /// Initialize the RabbitMQ Stream environment and create a producer for the "order.placed" stream
    pub async fn init() -> Result<Self> {
        // Read connection settings from environment variables or use defaults
        let host = std::env::var("RABBITMQ_HOST").unwrap_or_else(|_| "localhost".into());
        let port: u16 = std::env::var("RABBITMQ_STREAM_PORT")
            .unwrap_or_else(|_| "5552".into())
            .parse()?;
        let user = std::env::var("RABBITMQ_USER").unwrap_or_else(|_| "user".into());
        let pass = std::env::var("RABBITMQ_PASS").unwrap_or_else(|_| "pass".into());

        // Build the RabbitMQ Stream environment with credentials
        let env = Environment::builder()
            .host(&host)
            .port(port)
            .username(&user)
            .password(&pass)
            .build()
            .await?;

        let stream = "order.placed";
        // Create the stream if it does not already exist, ignoring the "already exists" error
        if let Err(e) = env
            .stream_creator()
            .max_length(ByteCapacity::GB(1))
            .create(stream)
            .await
        {
            if let StreamCreateError::Create { status, .. } = &e {
                if *status != ResponseCode::StreamAlreadyExists {
                    return Err(e.into());
                }
            } else {
                return Err(e.into());
            }
        }

        // Instantiate a producer for the specified stream
        let producer: rabbitmq_stream_client::Producer<NoDedup> =
            env.producer().build(stream).await?;
        Ok(Producer { inner: producer })
    }

    /// Publish an OrderMessage to the RabbitMQ Stream, awaiting confirmation
    pub async fn publish(&mut self, order: OrderMessage) -> Result<()> {
        // Serialize the order payload to JSON bytes
        let payload = serde_json::to_vec(&order)?;
        // Build the message and send with confirmation
        let msg = Message::builder().body(payload).build();
        self.inner.send_with_confirm(msg).await?;
        Ok(())
    }
}

// QueueLength represents the JSON response for queue length API
#[derive(serde::Serialize, ToSchema)]
pub struct QueueLength {
    pub pending_coffee_orders: u32,
}

/// Fetch the current number of pending messages in the 'order.placed' queue via the RabbitMQ Management API
pub async fn fetch_queue_length() -> Result<u32> {
    // Read management API connection info from environment or use defaults
    let protocol = std::env::var("RABBITMQ_MGMT_PROTOCOL").unwrap_or_else(|_| "http".into());
    let host = std::env::var("RABBITMQ_MGMT_HOST").unwrap_or_else(|_| "localhost".into());
    let port: u16 = std::env::var("RABBITMQ_MGMT_PORT")
        .unwrap_or_else(|_| "15672".into())
        .parse()?;
    let user = std::env::var("RABBITMQ_USER").unwrap_or_else(|_| "user".into());
    let pass = std::env::var("RABBITMQ_PASS").unwrap_or_else(|_| "pass".into());

    // Construct the management API URL for the target queue
    let mgmt_url = format!("{}://{}:{}", protocol, host, port);
    let url = format!("{}/api/queues/%2F/order.placed", mgmt_url);

    // Execute HTTP GET request with basic auth and parse JSON response
    let resp = reqwest::Client::new()
        .get(&url)
        .basic_auth(user, Some(pass))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    // Extract the 'messages_ready' field or default to zero
    Ok(resp["messages_ready"].as_u64().unwrap_or(0) as u32)
}
