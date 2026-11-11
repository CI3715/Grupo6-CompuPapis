use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;

const SERVER_URL: &str = "http://127.0.0.1:8000";

#[derive(Default)]
pub struct AuthState {
    token: Mutex<Option<String>>,
}

#[derive(Serialize, Deserialize)]
pub struct UsuarioAuth {
    pub id: String,
    pub nombre_usuario: String,
    pub nombre_visible: String,
    pub correo: String,
    pub fecha_alta: String,
}

#[derive(Serialize, Deserialize)]
pub struct ErrorAuth {
    pub campo: String,
    pub mensaje: String,
}

impl ErrorAuth {
    fn general(mensaje: &str) -> Self {
        Self {
            campo: "general".into(),
            mensaje: mensaje.into(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct RegistroAuth {
    pub nombre_usuario: String,
    pub nombre_visible: String,
    pub correo: String,
    pub contrasena: String,
}

#[derive(Serialize, Deserialize)]
pub struct CredencialesAuth {
    pub correo: String,
    pub contrasena: String,
}

#[derive(Deserialize)]
struct SesionServidor {
    token: String,
    vencimiento: String,
    usuario: UsuarioAuth,
}

#[derive(Serialize)]
pub struct SesionAuth {
    usuario: UsuarioAuth,
    vencimiento: String,
}

async fn respuesta<T: serde::de::DeserializeOwned>(
    request: reqwest::RequestBuilder,
) -> Result<T, ErrorAuth> {
    let response = request
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|_| ErrorAuth::general("No se pudo conectar con el servidor."))?;
    if !response.status().is_success() {
        return Err(response
            .json::<ErrorAuth>()
            .await
            .unwrap_or_else(|_| ErrorAuth::general("El servidor respondió con un error.")));
    }
    response
        .json()
        .await
        .map_err(|_| ErrorAuth::general("No se pudo leer la respuesta del servidor."))
}

#[tauri::command]
pub async fn registrar_usuario(
    datos: RegistroAuth,
    state: State<'_, super::PingState>,
) -> Result<UsuarioAuth, ErrorAuth> {
    respuesta(
        state
            .http_client
            .post(format!("{SERVER_URL}/usuarios"))
            .json(&datos),
    )
    .await
}

#[tauri::command]
pub async fn iniciar_sesion(
    datos: CredencialesAuth,
    state: State<'_, super::PingState>,
    auth: State<'_, AuthState>,
) -> Result<SesionAuth, ErrorAuth> {
    let sesion: SesionServidor = respuesta(
        state
            .http_client
            .post(format!("{SERVER_URL}/sesiones"))
            .json(&datos),
    )
    .await?;
    *auth
        .token
        .lock()
        .map_err(|_| ErrorAuth::general("No se pudo guardar la sesión."))? = Some(sesion.token);
    Ok(SesionAuth {
        usuario: sesion.usuario,
        vencimiento: sesion.vencimiento,
    })
}

#[tauri::command]
pub async fn obtener_perfil(
    state: State<'_, super::PingState>,
    auth: State<'_, AuthState>,
) -> Result<UsuarioAuth, ErrorAuth> {
    let token = auth
        .token
        .lock()
        .map_err(|_| ErrorAuth::general("No se pudo consultar la sesión."))?
        .clone()
        .ok_or_else(|| ErrorAuth::general("Inicia sesión para consultar tu perfil."))?;
    respuesta(
        state
            .http_client
            .get(format!("{SERVER_URL}/usuarios/yo"))
            .bearer_auth(token),
    )
    .await
}

#[tauri::command]
pub fn cerrar_sesion(auth: State<'_, AuthState>) -> Result<(), ErrorAuth> {
    *auth
        .token
        .lock()
        .map_err(|_| ErrorAuth::general("No se pudo cerrar la sesión."))? = None;
    Ok(())
}
