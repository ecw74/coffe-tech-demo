use axum::{extract::Extension, http::StatusCode, Json, Router};
use serde::{Deserialize, Serialize};
use std::{net::{Ipv4Addr, SocketAddr}, sync::Arc};
use tokio::{net::TcpListener, sync::Mutex};
use tracing::{info, warn};
use utoipa::{OpenApi, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

/// In-memory inventory state
#[derive(Debug, Default)]
struct Inventory {
    beans: u32,
    milk: u32,
}

type SharedInventory = Arc<Mutex<Inventory>>;

/// Response payload for GET /fill
#[derive(Serialize, ToSchema)]
struct InventoryResponse {
    beans: u32,
    milk: u32,
}

/// Request payload for PUT /fill
#[derive(Deserialize, ToSchema)]
struct InventoryUpdate {
    #[serde(default)]
    beans: Option<u32>,
    #[serde(default)]
    milk: Option<u32>,
}

/// Response for successful update
#[derive(Serialize, ToSchema)]
struct UpdateResponse {
    message: String,
    beans: u32,
    milk: u32,
}

/// Error response structure
#[derive(Serialize, ToSchema)]
struct ErrorResponse {
    error: String,
}

/// OpenAPI documentation definition
#[derive(OpenApi)]
#[openapi(
    paths(
        get_fill,
        put_fill
    ),
    components(
        schemas(
            InventoryResponse,
            InventoryUpdate,
            UpdateResponse,
            ErrorResponse
        )
    ),
    tags(
        (name = "Inventory", description = "Inventory management API")
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() {
    // init tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // initialize shared inventory
    let shared_inventory = Arc::new(Mutex::new(Inventory { beans: 20, milk: 10 }));

    // build OpenAPI router
    let (api_router, api_spec) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(utoipa_axum::routes![get_fill])
        .routes(utoipa_axum::routes![put_fill])
        .routes(utoipa_axum::routes![del_fill])
        .split_for_parts();

    // construct application
    let app = Router::new()
        // serve Swagger UI
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api_spec))
        // mount API routes
        .merge(api_router)
        // add shared inventory state
        .layer(Extension(shared_inventory));

    // bind and run
    let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 8081));
    let listener = TcpListener::bind(&addr).await.unwrap();
    info!("Listening on {}", addr);
    axum::serve(listener, app.into_make_service()).await.unwrap();
}

/// Handler for GET /fill
#[utoipa::path(
    get,
    path = "/fill",
    tag = "Inventory",
    responses(
        (status = 200, description = "Current inventory levels", body = InventoryResponse)
    )
)]
async fn get_fill(
    Extension(state): Extension<SharedInventory>
) -> (StatusCode, Json<InventoryResponse>) {
    let inv = state.lock().await;
    (
        StatusCode::OK,
        Json(InventoryResponse { beans: inv.beans, milk: inv.milk })
    )
}

/// Handler for PUT /fill
#[utoipa::path(
    put,
    path = "/fill",
    tag = "Inventory",
    request_body(content = InventoryUpdate, content_type = "application/json"),
    responses(
        (status = 200, description = "Inventory updated", body = UpdateResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse)
    )
)]
async fn put_fill(
    Extension(state): Extension<SharedInventory>,
    Json(payload): Json<InventoryUpdate>,
) -> Result<(StatusCode, Json<UpdateResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Validate and apply update
    if payload.beans.unwrap_or(0) == 0 && payload.milk.unwrap_or(0) == 0 {
        let err = ErrorResponse { error: "No values to update".into() };
        return Err((StatusCode::BAD_REQUEST, Json(err)));
    }

    let mut inv = state.lock().await;
    if let Some(b) = payload.beans {
        inv.beans = inv.beans.checked_add(b).ok_or_else(|| {
            (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "Beans overflow".into() }))
        })?;
    }
    if let Some(m) = payload.milk {
        inv.milk = inv.milk.checked_add(m).ok_or_else(|| {
            (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "Milk overflow".into() }))
        })?;
    }

    // Optional warning if low
    if inv.beans < 2 {
        warn!("Bean levels critically low: {} beans remaining", inv.beans);
    }

    let resp = UpdateResponse {
        message: "Inventory updated".into(),
        beans: inv.beans,
        milk: inv.milk,
    };
    Ok((StatusCode::OK, Json(resp)))
}

/// Handler for DEL /fill
#[utoipa::path(
    delete,
    path = "/fill",
    tag = "Inventory",
    request_body(content = InventoryUpdate, content_type = "application/json"),
    responses(
        (status = 200, description = "Inventory updated", body = UpdateResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse)
    )
)]
async fn del_fill(
    Extension(state): Extension<SharedInventory>,
    Json(payload): Json<InventoryUpdate>,
) -> Result<(StatusCode, Json<UpdateResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Validate and apply update
    if payload.beans.unwrap_or(0) == 0 && payload.milk.unwrap_or(0) == 0 {
        let err = ErrorResponse { error: "No values to update".into() };
        return Err((StatusCode::BAD_REQUEST, Json(err)));
    }

    let mut inv = state.lock().await;
    if let Some(b) = payload.beans {
        inv.beans = inv.beans.checked_sub(b).ok_or_else(|| {
            (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "Beans underflow".into() }))
        })?;
    }
    if let Some(m) = payload.milk {
        inv.milk = inv.milk.checked_sub(m).ok_or_else(|| {
            (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "Milk underflow".into() }))
        })?;
    }

    // Optional warning if low
    if inv.beans < 2 {
        warn!("Bean levels critically low: {} beans remaining", inv.beans);
    }

    // Optional warning if low
    if inv.milk < 2 {
        warn!("Milk levels critically low: {} milk remaining", inv.milk);
    }

    let resp = UpdateResponse {
        message: "Inventory updated".into(),
        beans: inv.beans,
        milk: inv.milk,
    };
    Ok((StatusCode::OK, Json(resp)))
}

