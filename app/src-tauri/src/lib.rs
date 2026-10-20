//! Punto de entrada de la libreria principal de la aplicacion Tauri.

// Modulo de dominio con entidades y logica de negocio
pub mod dominio;

use serde::{Deserialize, Serialize};
use std::time::Instant;

// Estado compartido que mantiene el cliente HTTP para verificar el servidor
struct AppState {
    http_client: reqwest::Client,
}

// Estructura de datos recibida desde  el endpoint de salud del servidor
#[derive(Debug, Deserialize)]
struct PingResponse {
    estado: String,
    mensaje: String,
    version: String,
    timestamp: String,
}

// Respuesta enviada a la interfaz de usuario con la latencia calculada
#[derive(Debug, Serialize)]
struct PingResult {
    estado: String,
    mensaje: String,
    version: String,
    timestamp: String,
    latencia_ms: u64,
}

// Comando para comprobar conexion con el servidor
#[tauri::command]
async fn ping_servidor(state: tauri::State<'_, AppState>) -> Result<PingResult, String> {
    let inicio = Instant::now();

    let respuesta = state
        .http_client
        .get("http://127.0.0.1:8000/ping")
        .send()
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

// Configura el restado local, registra comandos e inicia la aplicacion Tauri
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let http_client = reqwest::Client::new();

    tauri::Builder::default()
        .manage(AppState { http_client })
        .invoke_handler(tauri::generate_handler![ping_servidor])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
