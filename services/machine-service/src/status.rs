use axum::{Extension, Json};
use chrono::Utc;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use utoipa::ToSchema;

/// Represents details of the most recent processed order in the status response
#[derive(Serialize, ToSchema)]
pub struct LastOrder {
    pub order_id: String, // Unique identifier of the last order
    #[serde(rename = "type")]
    pub r#type: String, // Beverage type: espresso, coffee, cappuccino
    pub status: String,   // Status of the last order (e.g., "done")
    pub finished_at: chrono::DateTime<Utc>, // Timestamp when the last order was completed
}

/// Defines the JSON structure returned by GET /status endpoint
#[derive(Serialize, ToSchema)]
pub struct StatusResponse {
    pub ready: bool,           // Indicates if the machine is ready for a new order
    pub last_order: LastOrder, // Information about the last processed order
}

/// Internal shared state for tracking machine status
pub struct StatusState {
    pub ready: bool,                          // Is the machine ready for a new order?
    pub last_order_id: String,                // ID of the last order processed
    pub last_type: String,                    // Type of the last order processed
    pub last_status: String,                  // Status of the last order (e.g., "done")
    pub last_finished: chrono::DateTime<Utc>, // Completion timestamp of the last order
}

impl StatusState {
    /// Creates a new StatusState with default initial values
    pub fn new() -> Self {
        Self {
            ready: true,                  // Machine starts in a ready state
            last_order_id: String::new(), // No orders processed yet
            last_type: String::new(),     // No type yet
            last_status: String::new(),   // No status yet
            last_finished: Utc::now(),    // Default to current time
        }
    }
}

/// GET /status endpoint returning the current machine status
#[utoipa::path(
    get,
    path = "/status",
    tag = "Status",
    responses(
        (status = 200, description = "Current status of the machine", body = StatusResponse, content_type = "application/json")
    )
)]
pub async fn get_status(
    Extension(state): Extension<Arc<Mutex<StatusState>>>, // Shared state injected by Axum
) -> Json<StatusResponse> {
    // Lock the state to read values
    let st = state.lock().unwrap();
    // Build the LastOrder struct from the shared state
    let last_order = LastOrder {
        order_id: st.last_order_id.clone(),
        r#type: st.last_type.clone(),
        status: st.last_status.clone(),
        finished_at: st.last_finished,
    };
    // Create the response payload
    let resp = StatusResponse {
        ready: st.ready,
        last_order,
    };
    Json(resp)
}
