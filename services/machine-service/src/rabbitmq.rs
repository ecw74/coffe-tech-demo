use crate::inventory;
use crate::status::StatusState;
use chrono::Utc;
use lapin::{
    options::{BasicAckOptions, BasicConsumeOptions, QueueDeclareOptions},
    types::FieldTable,
    Connection, ConnectionProperties,
};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use futures_util::StreamExt;


// Producer encapsulates a RabbitMQ Stream producer instance
pub struct Consumer;

#[derive(Deserialize)]
pub struct OrderMessage {
    pub order_id: String,
    pub r#type: String,
    pub timestamp: chrono::DateTime<Utc>,
}

impl Consumer {
    pub async fn run(state: Arc<Mutex<StatusState>>) -> anyhow::Result<()> {
        // Read connection settings from environment variables or use defaults
        let host = std::env::var("RABBITMQ_HOST").unwrap_or_else(|_| "localhost".into());
        let port: u16 = std::env::var("RABBITMQ_PORT")
            .unwrap_or_else(|_| "5672".into())
            .parse()?;
        let user = std::env::var("RABBITMQ_USER").unwrap_or_else(|_| "user".into());
        let pass = std::env::var("RABBITMQ_PASS").unwrap_or_else(|_| "pass".into());

        let addr = format!("amqp://{}:{}@{}:{}/%2f", user, pass, host, port);
        let conn = loop {
            let retry_delay = Duration::from_secs(1);
            match Connection::connect(&addr, ConnectionProperties::default()).await { 
                Ok(c) => break c,
                Err(err) => {
                    tracing::error!(error=%err, "Init of RabbitMQ consumer failed. Retrying in {:?}.", retry_delay);
                    sleep(retry_delay);
                }
            }
        };
        let channel = conn.create_channel().await.expect("Failed to create a channel");

        // Queue deklarieren (idempotent)
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

        // Consumer starten
        let mut consumer = channel
            .basic_consume(
                queue.name().as_str(),
                "consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await.expect("Failed to start consuming messages");

        tracing::info!("Waiting for messages on queue '{}'", queue.name().as_str());

        // Nachrichten-Schleife
        while let Some(delivery) = consumer.next().await {
            let delivery = delivery?;
            let data = &delivery.data;
            match serde_json::from_slice::<OrderMessage>(data) {
                Ok(order) => {
                    // Verarbeitung
                    Self::process_order(order, &state).await;
                    // Nachricht acken
                    channel
                        .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                        .await?;
                }
                Err(e) => {
                    tracing::error!("Ungültige Nachricht: {e}");
                    // ggf. nack mit Requeue=false
                    channel
                        .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                        .await?;
                }
            }
        }

        Ok(())
    }

    async fn process_order(order: OrderMessage, state: &Arc<Mutex<StatusState>>) {
        tracing::info!("Processing order {}: {}", order.order_id, order.r#type);

        // Zutatenbedarf ermitteln
        let (beans, milk) = match order.r#type.as_str() {
            "espresso" => (1, 0),
            "coffee" => (2, 1),
            "cappuccino" => (1, 2),
            _ => {
                tracing::error!("Unbekannter Getränketyp: {}", order.r#type);
                return;
            }
        };

        // Beständen abfragen
        let available = inventory::get_stock().await;
        if available.beans < beans || available.milk < milk {
            tracing::error!("Nicht genug Zutaten für {} (order_id: {})", order.r#type, order.order_id);
            // TODO: Optional in order.failed Queue
            return;
        }

        // Zutaten abziehen
        if inventory::deduct_stock(beans, milk).await.is_err() {
            tracing::error!("Fehler beim Abziehen der Zutaten für {}", order.order_id);
            return;
        }

        tracing::info!("Es stehen noch {} Bohnen und {} Milch zur Verfügung", available.beans, available.milk);
        tracing::info!("Received {} order with id {} at {}", order.r#type, order.order_id, order.timestamp);

        // Zubereitung simulieren
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // Status aktualisieren
        let mut st = state.lock().unwrap();
        st.last_order_id = order.order_id;
        st.last_type = order.r#type;
        st.last_status = "done".to_string();
        st.last_finished = Utc::now();
        st.ready = true;

        tracing::info!("Order {} fertig", st.last_order_id);
    }
}