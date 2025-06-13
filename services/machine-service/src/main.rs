mod inventory;
mod rabbitmq;
mod status;

use axum::{Extension, Router};
use status::StatusState;
use std::net::Ipv4Addr;
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::net::TcpListener;
use tokio::spawn;
use tracing::info;
use tracing_subscriber;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

// Define OpenAPI documentation for the service
#[derive(OpenApi)]
#[openapi(
    paths(status::get_status),
    components(schemas(status::StatusResponse)),
    tags(
        (name = "Orders", description = "Order APIs")
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing subscriber for structured logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Initialize shared machine status state wrapped in a thread-safe mutex
    let shared_state = Arc::new(Mutex::new(StatusState::new()));

    // Start the RabbitMQ consumer in the background, passing cloned state
    let consumer_state = shared_state.clone();
    spawn(async move {
        rabbitmq::Consumer::run(consumer_state)
            .await
            .expect("Consumer encountered an unrecoverable error");
    });

    // Build the OpenAPI router and specification
    let (api_router, api_spec) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(utoipa_axum::routes![status::get_status])
        .split_for_parts();

    // Construct the main application router
    let app = Router::new()
        // Serve Swagger UI at /swagger-ui with OpenAPI JSON at /api-docs/openapi.json
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api_spec.clone()))
        // Mount API endpoints
        .merge(api_router)
        // Make shared state available to handlers via Axum extension
        .layer(Extension(shared_state));

    // Determine service port from environment or default to 8082
    let port: u16 = std::env::var("SERVICE_PORT")
        .unwrap_or_else(|_| "8082".into())
        .parse()?;
    let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, port));

    // Bind TCP listener
    let listener = TcpListener::bind(&addr).await.unwrap();
    info!("Listening on {}", addr);

    // Start the Axum HTTP server
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
