use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;

#[derive(Default)]
pub struct AuthState {
    pub(crate) sesion: Mutex<Option<SesionLocal>>,
}

#[derive(Clone)]
pub(crate) struct SesionLocal {
    pub token: String,
    pub usuario_id: String,
}

impl AuthState {
    pub(crate) fn sesion_actual(&self) -> Result<SesionLocal, ErrorAuth> {
        self.sesion
            .lock()
            .map_err(|_| ErrorAuth::general("No se pudo consultar la sesión."))?
            .clone()
            .ok_or_else(|| ErrorAuth {
                campo: "sesion".into(),
                mensaje: "Inicia sesión para continuar.".into(),
            })
    }
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
    pub(crate) fn general(mensaje: &str) -> Self {
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
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct CredencialesAuth {
    pub correo: String,
    pub password: String,
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
    sesion_protegida: bool,
) -> Result<T, ErrorAuth> {
    let response = request
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|_| ErrorAuth::general("No se pudo conectar con el servidor."))?;
    if !response.status().is_success() {
        if sesion_protegida && response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ErrorAuth {
                campo: "sesion".into(),
                mensaje: "Tu sesión venció, inicia sesión de nuevo.".into(),
            });
        }
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
    nombre_usuario: String,
    nombre_visible: String,
    correo: String,
    password: String,
    state: State<'_, super::PingState>,
    app: State<'_, super::AppState>,
) -> Result<UsuarioAuth, ErrorAuth> {
    let datos = RegistroAuth {
        nombre_usuario,
        nombre_visible,
        correo,
        password,
    };
    respuesta(
        state
            .http_client
            .post(format!("{}/usuarios", app.server_url))
            .json(&datos),
        false,
    )
    .await
}

#[tauri::command]
pub async fn iniciar_sesion(
    correo: String,
    password: String,
    state: State<'_, super::PingState>,
    auth: State<'_, AuthState>,
    app: State<'_, super::AppState>,
) -> Result<SesionAuth, ErrorAuth> {
    let datos = CredencialesAuth { correo, password };
    let sesion: SesionServidor = respuesta(
        state
            .http_client
            .post(format!("{}/sesiones", app.server_url))
            .json(&datos),
        false,
    )
    .await?;
    *auth
        .sesion
        .lock()
        .map_err(|_| ErrorAuth::general("No se pudo guardar la sesión."))? = Some(SesionLocal {
        token: sesion.token,
        usuario_id: sesion.usuario.id.clone(),
    });
    Ok(SesionAuth {
        usuario: sesion.usuario,
        vencimiento: sesion.vencimiento,
    })
}

#[tauri::command]
pub async fn obtener_perfil(
    state: State<'_, super::PingState>,
    auth: State<'_, AuthState>,
    app: State<'_, super::AppState>,
) -> Result<UsuarioAuth, ErrorAuth> {
    let token = auth.sesion_actual()?.token;
    respuesta(
        state
            .http_client
            .get(format!("{}/usuarios/yo", app.server_url))
            .bearer_auth(token),
        true,
    )
    .await
}

#[tauri::command]
pub fn cerrar_sesion(auth: State<'_, AuthState>) -> Result<(), ErrorAuth> {
    *auth
        .sesion
        .lock()
        .map_err(|_| ErrorAuth::general("No se pudo cerrar la sesión."))? = None;
    Ok(())
}
