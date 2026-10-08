use axum::{Json, Router, routing::get};
use serde::Serialize;
use tower_http::cors::{Any, CorsLayer};

#[derive(Serialize)]
struct PingResponse {
    estado: &'static str,
    mensaje: &'static str,
    version: &'static str,
    timestamp: String,
}

async fn ping() -> Json<PingResponse> {
    Json(PingResponse {
        estado: "ok",
        mensaje: "Servidor Cuentas Claras activo",
        version: "0.1.0",
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

#[tokio::main]
async fn main() {
    // Configuración permissiva de CORS para desarrollo
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Aplicar la capa de CORS a la aplicación
    let app = Router::new()
        .route("/ping", get(ping))
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000")
        .await
        .unwrap();

    println!("Servidor activo en http://127.0.0.1:8000");

    axum::serve(listener, app).await.unwrap();
}