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
async fn main() {
    let app = Router::new().route("/ping", get(ping));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000")
        .await
        .unwrap();

    println!("Servidor activo en http://127.0.0.1:8000");

    axum::serve(listener, app).await.unwrap();
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
