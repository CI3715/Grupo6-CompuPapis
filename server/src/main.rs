pub mod amistades_db;
mod auth;
mod db;
mod division;
mod grupos;
mod social;
pub mod usuarios_db;

use axum::{Json, Router, routing::get};
use serde::Serialize;

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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL").map_err(|_| "Falta DATABASE_URL")?;
    let secret = std::env::var("JWT_SECRET").map_err(|_| "Falta JWT_SECRET")?;
    if secret.len() < 32 {
        return Err("JWT_SECRET debe tener al menos 32 bytes".into());
    }

    let pool = db::conectar_y_migrar(&database_url).await?;

    let modo_demostracion = std::env::var("TOKEN_DEMO_1_MINUTO").as_deref() == Ok("1");
    let estado = auth::State::new(pool, secret).con_modo_demostracion(modo_demostracion);
    let app = Router::new()
        .route("/ping", get(ping))
        .merge(grupos::routes(estado.clone()))
        .merge(social::routes(estado.clone()))
        .merge(auth::routes(estado));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000").await?;
    println!("Servidor activo en http://127.0.0.1:8000");
    axum::serve(listener, app).await?;
    Ok(())
}
