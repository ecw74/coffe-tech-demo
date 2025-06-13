use axum::{Extension, Json};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use chrono::Utc;
use utoipa::ToSchema;

/// Struktur für die letzte Bestellung im Status-Response
#[derive(Serialize, ToSchema)]
pub struct LastOrder {
    pub order_id: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub status: String,
    pub finished_at: chrono::DateTime<Utc>,
}

/// Struktur für den Status-Response
#[derive(Serialize, ToSchema)]
pub struct StatusResponse {
    pub ready: bool,
    pub last_order: LastOrder,
}

/// Interner Shared State der Maschine
pub struct StatusState {
    pub ready: bool,
    pub last_order_id: String,
    pub last_type: String,
    pub last_status: String,
    pub last_finished: chrono::DateTime<Utc>,
}

impl StatusState {
    /// Initialisiert den State mit Defaults
    pub fn new() -> Self {
        Self {
            ready: true,
            last_order_id: String::new(),
            last_type: String::new(),
            last_status: String::new(),
            last_finished: Utc::now(),
        }
    }
}

#[utoipa::path(
    get,
    path = "/status",
    tag = "Status",
    responses(
        (status = 200, description = "Current status of the machine", body = StatusResponse, content_type = "application/json")
    )
)]
pub async fn get_status(
    Extension(state): Extension<Arc<Mutex<StatusState>>>,
) -> Json<StatusResponse> {
    let st = state.lock().unwrap();
    let last_order = LastOrder {
        order_id: st.last_order_id.clone(),
        r#type: st.last_type.clone(),
        status: st.last_status.clone(),
        finished_at: st.last_finished,
    };
    let resp = StatusResponse {
        ready: st.ready,
        last_order,
    };
    Json(resp)
}