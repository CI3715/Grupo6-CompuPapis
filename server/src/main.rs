mod auth;

use axum::{Json, Router, routing::get};
use serde::Serialize;
use sqlx::postgres::PgPoolOptions;

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
    let pool = PgPoolOptions::new().connect(&database_url).await?;
    sqlx::raw_sql(include_str!("../migrations/001_usuarios.sql"))
        .execute(&pool)
        .await?;
    let app = Router::new()
        .route("/ping", get(ping))
        .merge(auth::routes(auth::State::new(pool, secret)));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000").await?;
    println!("Servidor activo en http://127.0.0.1:8000");
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn ping_conserva_contrato_entrega_1() {
        let respuesta = super::ping().await.0;
        assert_eq!(respuesta.estado, "ok");
        assert_eq!(respuesta.mensaje, "Servidor Cuentas Claras activo");
        assert_eq!(respuesta.version, "0.1.0");
        assert!(chrono::DateTime::parse_from_rfc3339(&respuesta.timestamp).is_ok());
    }
}
