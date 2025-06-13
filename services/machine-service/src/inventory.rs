use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::env;
use thiserror::Error;

/// Repräsentiert den aktuellen Lagerbestand an Bohnen und Milch des Inventory Service
#[derive(Debug, Serialize, Deserialize)]
pub struct Stock {
    pub beans: u32,
    pub milk: u32,
}

/// Fehlerarten, die beim Zugriff auf den Inventory Service auftreten können
#[derive(Debug, Error)]
pub enum InventoryError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Unexpected response status: {0}")]
    Status(reqwest::StatusCode),
}

/// Hilfsfunktion zum Ermitteln der Basis-URL aus Umgebungsvariablen
fn base_url() -> String {
    env::var("INVENTORY_SERVICE_URL")
        .unwrap_or_else(|_| "http://localhost:8081".to_string())
}

/// Gibt den aktuellen Lagerbestand zurück
pub async fn get_stock() -> Stock {
    let url = format!("{}/fill", base_url());
    let client = Client::new();
    let resp = client
        .get(&url)
        .send()
        .await
        .expect("Failed to call inventory GET /fill");
    if !resp.status().is_success() {
        panic!("Inventory Service returned error: {}", resp.status());
    }
    resp.json::<Stock>()
        .await
        .expect("Failed to deserialize inventory response")
}

/// Zieht die angegebene Menge an Bohnen und Milch vom Lager ab
pub async fn deduct_stock(beans: u32, milk: u32) -> Result<(), InventoryError> {
    let url = format!("{}/fill", base_url());
    let client = Client::new();
    let payload = Stock { beans, milk };
    let resp = client
        .delete(&url)
        .json(&payload)
        .send()
        .await?;
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(InventoryError::Status(resp.status()))
    }
}