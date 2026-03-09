mod api_doc;
mod auth;
mod kafka;
mod upload;
mod validation;

use axum::{
    extract::DefaultBodyLimit,
    routing::post,
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use api_doc::ApiDoc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let kafka_brokers =
        std::env::var("KAFKA_BROKERS").unwrap_or_else(|_| "localhost:9092".to_string());
    let kafka_producer =
        kafka::new_producer(&kafka_brokers).expect("Failed to create Kafka producer");

    let jwt_secret =
        std::env::var("JWT_SECRET").unwrap_or_else(|_| "super-secret-key-change-me".to_string());

    let temp_dir =
        std::env::var("TEMP_UPLOAD_DIR").unwrap_or_else(|_| "/tmp/media_uploads".to_string());
    std::fs::create_dir_all(&temp_dir).expect("Failed to create temp upload directory");

    let state = upload::AppState {
        kafka: kafka_producer,
        jwt_secret,
        temp_dir,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/api/media/upload", post(upload::upload_media))
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024))
        .layer(cors)
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8081".to_string());
    let addr = format!("0.0.0.0:{}", port);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind");

    info!("media_api listening on http://{}", addr);
    info!("Swagger UI: http://localhost:{}/swagger-ui/", port);
    info!(
        "OpenAPI JSON: http://localhost:{}/api-docs/openapi.json",
        port
    );

    axum::serve(listener, app).await.expect("Server error");
}
