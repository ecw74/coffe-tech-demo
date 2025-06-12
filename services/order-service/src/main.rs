use axum::{Json, Router, extract::Extension, http::StatusCode};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;
use std::time::Duration;
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{error, info};
use utoipa_axum::router::OpenApiRouter;
use uuid::Uuid;

use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

mod rabbitmq;

// Type alias for shared, thread-safe access to the RabbitMQ producer
type SharedProducer = Arc<Mutex<rabbitmq::Producer>>;

// Request payload for placing an order
#[derive(Deserialize, ToSchema)]
struct OrderRequest {
    #[serde(rename = "type")]
    drink_type: String,
}

// Successful order response structure
#[derive(Serialize, ToSchema)]
struct OrderResponse {
    message: String,
    order_id: String,
}

// Error response structure
#[derive(Serialize, ToSchema)]
struct ErrorResponse {
    error: String,
}

// Define OpenAPI documentation for the API
#[derive(OpenApi)]
#[openapi(
    paths(post_order),
    components(schemas(OrderRequest, OrderResponse, ErrorResponse)),
    tags(
        (name = "Orders", description = "Order APIs")
    )
)]
struct ApiDoc;

// Main entry point of the application
#[tokio::main]
async fn main() {
    // Initialize tracing subscriber for logging with environment-based level filter
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Initialize the RabbitMQ producer, retrying until successful
    let producer = loop {
        let retry_delay = Duration::from_secs(1);
        match rabbitmq::Producer::init().await {
            Ok(p) => break p,
            Err(err) => {
                error!(error=%err, "Init of RabbitMQ producer failed. Retrying in {:?}.", retry_delay);
                sleep(retry_delay).await;
            }
        }
    };
    // Wrap the producer in an Arc<Mutex<>> for shared, async-safe usage in handlers
    let shared_producer = Arc::new(Mutex::new(producer));

    // Build OpenAPI router and extract the spec for Swagger UI
    let (api_router, api_spec) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(utoipa_axum::routes![post_order])
        .routes(utoipa_axum::routes![get_queue_length])
        .split_for_parts();

    // Construct the full application router
    let app = Router::new()
        // Serve Swagger UI at /swagger-ui
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api_spec.clone()))
        // Mount the API routes
        .merge(api_router)
        // Add shared producer as an extension for handlers to access
        .layer(Extension(shared_producer));

    // Bind to 0.0.0.0:8080 and start serving
    let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 8080));
    let listener = TcpListener::bind(&addr).await.unwrap();
    info!("Listening on {}", addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

/// Handler for placing a new coffee order
#[utoipa::path(
    post,
    path = "/order",
    request_body(
            content = OrderRequest,
            description = "Details of the drink order",
            content_type = "application/json"
    ),
    responses(
            (status = 202, description = "Order accepted", body = OrderResponse, content_type = "application/json"),
            (status = 400, description = "Invalid drink type", body = ErrorResponse, content_type = "application/json"),
            (status = 500, description = "Internal server error", body = ErrorResponse, content_type = "application/json")
    )
)]
async fn post_order(
    // Inject shared RabbitMQ producer
    Extension(producer): Extension<SharedProducer>,
    // Deserialize JSON payload into OrderRequest
    Json(payload): Json<OrderRequest>,
) -> Result<(StatusCode, Json<OrderResponse>), (StatusCode, Json<ErrorResponse>)> {
    // 1) Validate the requested drink type
    if !matches!(
        payload.drink_type.as_str(),
        "espresso" | "coffee" | "cappuccino"
    ) {
        let err = ErrorResponse {
            error: "This is a coffee-only establishment ☕".into(),
        };
        // Return 400 Bad Request for unsupported drink types
        return Err((StatusCode::BAD_REQUEST, Json(err)));
    }

    // 2) Construct the order message with a new UUID and current timestamp
    let order_id = Uuid::new_v4().to_string();
    let order_msg = rabbitmq::OrderMessage {
        order_id: order_id.clone(),
        r#type: payload.drink_type.clone(),
        timestamp: chrono::Utc::now(),
    };

    // Acquire lock on the producer and attempt to publish the message
    let mut prod = producer.lock().await;
    if let Err(e) = prod.publish(order_msg).await {
        error!("Publish failed: {e}");
        let err = ErrorResponse {
            error: "Internal server error".into(),
        };
        // Return 500 Internal Server Error if publish fails
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(err)));
    }

    // 3) On success, respond with 202 Accepted and the generated order ID
    let resp = OrderResponse {
        message: "Order received".into(),
        order_id,
    };
    Ok((StatusCode::ACCEPTED, Json(resp)))
}

/// Handler for fetching the current queue length from RabbitMQ
#[utoipa::path(
    get,
    path = "/orders/queue-length",
    tag = "Orders",
    responses(
        (status = 200, description = "Current queue length", body = rabbitmq::QueueLength, content_type = "application/json"),
        (status = 500, description = "Internal server error", body = ErrorResponse, content_type = "application/json")
    )
)]
async fn get_queue_length()
-> Result<(StatusCode, Json<rabbitmq::QueueLength>), (StatusCode, Json<ErrorResponse>)> {
    // Attempt to fetch queue length via RabbitMQ management API or passive inspection
    match rabbitmq::fetch_queue_length().await {
        Ok(len) => Ok((
            StatusCode::OK,
            Json(rabbitmq::QueueLength {
                pending_coffee_orders: len,
            }),
        )),
        Err(e) => {
            error!("Queue length error: {e}");
            let err = ErrorResponse {
                error: "Internal server error".into(),
            };
            // Return 500 Internal Server Error on failure
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(err)))
        }
    }
}
