use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use thiserror::Error;

/// Represents the current stock of beans and milk from the Inventory Service
#[derive(Debug, Serialize, Deserialize)]
pub struct Stock {
    pub beans: u32, // amount of coffee beans available
    pub milk: u32,  // amount of milk available
}

/// Errors that can occur when communicating with the Inventory Service
#[derive(Debug, Error)]
pub enum InventoryError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error), // network or protocol errors
    #[error("Unexpected response status: {0}")]
    Status(reqwest::StatusCode), // non-success HTTP status codes
}

/// Helper function to determine the base URL for the Inventory Service from environment variables
fn base_url() -> String {
    env::var("INVENTORY_SERVICE_URL").unwrap_or_else(|_| "http://localhost:8081".to_string())
}

/// Fetches the current stock levels from the Inventory Service via GET /fill
pub async fn get_stock() -> Stock {
    let url = format!("{}/fill", base_url());
    let client = Client::new();
    let resp = client
        .get(&url)
        .send()
        .await
        .expect("Failed to call inventory GET /fill");

    if !resp.status().is_success() {
        panic!("Inventory Service returned error status: {}", resp.status());
    }

    resp.json::<Stock>()
        .await
        .expect("Failed to deserialize inventory response")
}

/// Deducts the specified amounts of beans and milk from the Inventory Service via DELETE /fill
pub async fn deduct_stock(beans: u32, milk: u32) -> Result<(), InventoryError> {
    let url = format!("{}/fill", base_url());
    let client = Client::new();
    let payload = Stock { beans, milk };

    let resp = client.delete(&url).json(&payload).send().await?;

    if resp.status().is_success() {
        Ok(())
    } else {
        Err(InventoryError::Status(resp.status()))
    }
}
