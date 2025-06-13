use crate::inventory;
use crate::status::StatusState;
use chrono::Utc;
use futures_util::StreamExt;
use lapin::{
    Connection, ConnectionProperties,
    options::{BasicAckOptions, BasicConsumeOptions, QueueDeclareOptions},
    types::FieldTable,
};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

/// Consumer handles incoming order messages from RabbitMQ and processes them
pub struct Consumer;

/// Structure representing the incoming order message fields
#[derive(Deserialize)]
pub struct OrderMessage {
    pub order_id: String,                 // Unique identifier for the order
    pub r#type: String,                   // Beverage type: espresso, coffee, cappuccino
    pub timestamp: chrono::DateTime<Utc>, // Time the order was placed
}

impl Consumer {
    /// Starts the RabbitMQ consumer loop using the provided shared status state
    pub async fn run(state: Arc<Mutex<StatusState>>) -> anyhow::Result<()> {
        // Load RabbitMQ connection settings from environment or use defaults
        let host = std::env::var("RABBITMQ_HOST").unwrap_or_else(|_| "localhost".into());
        let port: u16 = std::env::var("RABBITMQ_PORT")
            .unwrap_or_else(|_| "5672".into())
            .parse()?;
        let user = std::env::var("RABBITMQ_USER").unwrap_or_else(|_| "user".into());
        let pass = std::env::var("RABBITMQ_PASS").unwrap_or_else(|_| "pass".into());

        // Build AMQP URI and attempt to connect with retry logic
        let addr = format!("amqp://{}:{}@{}:{}/%2f", user, pass, host, port);
        let conn = loop {
            let retry_delay = Duration::from_secs(1);
            match Connection::connect(&addr, ConnectionProperties::default()).await {
                Ok(c) => break c,
                Err(err) => {
                    tracing::error!(error=%err, "Failed to initialize RabbitMQ consumer. Retrying in {:?}...", retry_delay);
                    sleep(retry_delay);
                }
            }
        };

        // Create a channel on the established connection
        let channel = conn
            .create_channel()
            .await
            .expect("Failed to create RabbitMQ channel");

        // Declare the 'order.placed' queue idempotently
        let queue = channel
            .queue_declare(
                "order.placed",
                QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await?;

        // Start consuming messages from the queue
        let mut consumer = channel
            .basic_consume(
                queue.name().as_str(),
                "consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .expect("Failed to start RabbitMQ consumer");

        tracing::info!("Waiting for messages on queue '{}'", queue.name().as_str());

        // Process each delivery as it arrives
        while let Some(delivery) = consumer.next().await {
            let delivery = delivery?;
            let data = &delivery.data;
            match serde_json::from_slice::<OrderMessage>(data) {
                Ok(order) => {
                    // Process the valid order message
                    Self::process_order(order, &state).await;
                    // Acknowledge the message on success
                    channel
                        .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                        .await?;
                }
                Err(e) => {
                    tracing::error!(error=%e, "Invalid message received, discarding");
                    // Acknowledge to remove it from the queue (no requeue)
                    channel
                        .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                        .await?;
                }
            }
        }

        Ok(())
    }

    /// Handles the business logic for preparing an order
    async fn process_order(order: OrderMessage, state: &Arc<Mutex<StatusState>>) {
        tracing::info!(
            "Processing order {} of type {}",
            order.order_id,
            order.r#type
        );

        // Determine ingredient requirements based on beverage type
        let (beans, milk) = match order.r#type.as_str() {
            "espresso" => (1, 0),
            "coffee" => (2, 1),
            "cappuccino" => (1, 2),
            _ => {
                tracing::error!("Unknown beverage type: {}", order.r#type);
                return;
            }
        };

        // Query current stock levels
        let available = inventory::get_stock().await;
        if available.beans < beans || available.milk < milk {
            tracing::error!(
                "Insufficient ingredients for {} (order_id: {})",
                order.r#type,
                order.order_id
            );
            // TODO: Optionally publish to an order.failed queue
            return;
        }

        // Deduct the required ingredients
        if inventory::deduct_stock(beans, milk).await.is_err() {
            tracing::error!("Failed to deduct ingredients for order {}", order.order_id);
            return;
        }

        tracing::info!(
            "Stock after deduction: {} beans, {} milk",
            available.beans,
            available.milk
        );
        tracing::info!(
            "Received order {} (type {}) at {}",
            order.order_id,
            order.r#type,
            order.timestamp
        );

        // Simulate preparation delay
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Update shared status state upon completion
        let mut st = state.lock().unwrap();
        st.last_order_id = order.order_id;
        st.last_type = order.r#type;
        st.last_status = "done".to_string();
        st.last_finished = Utc::now();
        st.ready = true;

        tracing::info!("Order {} completed", st.last_order_id);
    }
}
