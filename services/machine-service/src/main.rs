mod rabbitmq;
mod inventory;
mod status;

use axum::{Router, Extension};
use std::{net::SocketAddr, sync::{Arc, Mutex}};
use std::net::Ipv4Addr;
use tokio::net::TcpListener;
use tokio::spawn;
use tracing::info;
use tracing_subscriber;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;
use status::StatusState;

// Define OpenAPI documentation for the API
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
async fn main() {
    // Logging initialisieren
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Gemeinsamen Status initialisieren
    let shared_state = Arc::new(Mutex::new(StatusState::new()));

    // RabbitMQ-Consumer im Hintergrund starten
    let consumer_state = shared_state.clone();
    spawn(async move {
        rabbitmq::Consumer::run(consumer_state).await.expect("TODO: panic message");
    });

    let (api_router, api_spec) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(utoipa_axum::routes![status::get_status])
        .split_for_parts();

    // Construct the full application router
    let app = Router::new()
        // Serve Swagger UI at /swagger-ui
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api_spec.clone()))
        // Mount the API routes
        .merge(api_router)
        // Add shared producer as an extension for handlers to access
        .layer(Extension(shared_state));

    // Server starten
    let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 8082));
    let listener = TcpListener::bind(&addr).await.unwrap();
    info!("Listening on {}", addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}