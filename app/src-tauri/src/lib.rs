use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Deserialize)]
struct PingResponse {
    estado: String,
    mensaje: String,
    version: String,
    timestamp: String,
}

#[derive(Debug, Serialize)]
struct PingResult {
    estado: String,
    mensaje: String,
    version: String,
    timestamp: String,
    latencia_ms: u64,
}

#[tauri::command]
async fn ping_servidor() -> Result<PingResult, String> {
    let inicio = Instant::now();

    let respuesta = reqwest::get("http://127.0.0.1:8000/ping")
        .await
        .map_err(|error| format!("No se pudo conectar con el servidor: {error}"))?;

    if !respuesta.status().is_success() {
        return Err(format!(
            "El servidor respondió con el código {}",
            respuesta.status()
        ));
    }

    let ping = respuesta
        .json::<PingResponse>()
        .await
        .map_err(|error| format!("No se pudo leer la respuesta del servidor: {error}"))?;

    let latencia_ms = inicio.elapsed().as_millis() as u64;

    Ok(PingResult {
        estado: ping.estado,
        mensaje: ping.mensaje,
        version: ping.version,
        timestamp: ping.timestamp,
        latencia_ms,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ping_servidor])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
