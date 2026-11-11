use crate::domain::ErrorDominio;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const KEYRING_SERVICE: &str = "com.cuentasclaras.app";
const KEYRING_USER: &str = "sesion_token";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UsuarioPublico {
    pub id: String,
    pub nombre_usuario: String,
    pub nombre_visible: String,
    pub correo: String,
    pub fecha_alta: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SesionResp {
    pub token: String,
    pub vence_en: String,
    pub usuario: UsuarioPublico,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstadoSesionApp {
    pub activa: bool,
    pub vencida: bool,
    pub mensaje_aviso: Option<String>,
    pub usuario: Option<UsuarioPublico>,
}

#[derive(Debug, Deserialize)]
struct ErrorServidor {
    #[allow(dead_code)]
    pub codigo: Option<String>,
    pub mensaje: String,
    pub campo: Option<String>,
}

pub fn guardar_token_seguro(ruta_fallback: &PathBuf, token: &str) {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        if entry.set_password(token).is_ok() {
            let _ = std::fs::remove_file(ruta_fallback);
            return;
        }
    }
    let _ = std::fs::write(ruta_fallback, token);
}

pub fn leer_token_seguro(ruta_fallback: &PathBuf) -> Option<String> {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        if let Ok(t) = entry.get_password() {
            if !t.trim().is_empty() {
                return Some(t);
            }
        }
    }
    std::fs::read_to_string(ruta_fallback)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn borrar_token_seguro(ruta_fallback: &PathBuf) {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        let _ = entry.delete_credential();
    }
    let _ = std::fs::remove_file(ruta_fallback);
}

fn mapear_error_red() -> ErrorDominio {
    ErrorDominio::nuevo(
        "red",
        "No hay conexión con el servidor. Tus cuentas, movimientos y patrimonio siguen disponibles localmente.",
    )
}

pub fn registrar_en_servidor(
    server_url: &str,
    nombre_usuario: &str,
    nombre_visible: &str,
    correo: &str,
    contrasena: &str,
) -> Result<UsuarioPublico, ErrorDominio> {
    let client = reqwest::blocking::Client::new();
    let url = format!("{}/usuarios", server_url.trim_end_matches('/'));

    let resp = client
        .post(&url)
        .json(&serde_json::json!({
            "nombreUsuario": nombre_usuario,
            "nombreVisible": nombre_visible,
            "correo": correo,
            "contrasena": contrasena,
        }))
        .send()
        .map_err(|_| mapear_error_red())?;

    if resp.status().is_success() {
        resp.json::<UsuarioPublico>()
            .map_err(|_| ErrorDominio::nuevo("general", "Respuesta inesperada del servidor."))
    } else {
        let err = resp.json::<ErrorServidor>().unwrap_or(ErrorServidor {
            codigo: None,
            mensaje: "No se pudo registrar el usuario.".into(),
            campo: Some("general".into()),
        });
        Err(ErrorDominio::nuevo(
            err.campo.as_deref().unwrap_or("general"),
            &err.mensaje,
        ))
    }
}

pub fn iniciar_sesion_en_servidor(
    server_url: &str,
    ruta_fallback: &PathBuf,
    correo: &str,
    contrasena: &str,
) -> Result<UsuarioPublico, ErrorDominio> {
    let client = reqwest::blocking::Client::new();
    let url = format!("{}/sesiones", server_url.trim_end_matches('/'));

    let resp = client
        .post(&url)
        .json(&serde_json::json!({
            "correo": correo,
            "contrasena": contrasena,
        }))
        .send()
        .map_err(|_| mapear_error_red())?;

    if resp.status().is_success() {
        let sesion = resp
            .json::<SesionResp>()
            .map_err(|_| ErrorDominio::nuevo("general", "Respuesta inesperada del servidor."))?;
        guardar_token_seguro(ruta_fallback, &sesion.token);
        Ok(sesion.usuario)
    } else {
        let err = resp.json::<ErrorServidor>().unwrap_or(ErrorServidor {
            codigo: None,
            mensaje: "Correo o contraseña incorrectos.".into(),
            campo: Some("general".into()),
        });
        Err(ErrorDominio::nuevo(
            err.campo.as_deref().unwrap_or("general"),
            &err.mensaje,
        ))
    }
}

pub fn obtener_perfil_actual(server_url: &str, ruta_fallback: &PathBuf) -> Result<EstadoSesionApp, ErrorDominio> {
    let token = match leer_token_seguro(ruta_fallback) {
        Some(t) => t,
        None => {
            return Ok(EstadoSesionApp {
                activa: false,
                vencida: false,
                mensaje_aviso: None,
                usuario: None,
            })
        }
    };

    let client = reqwest::blocking::Client::new();
    let url = format!("{}/usuarios/yo", server_url.trim_end_matches('/'));

    match client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
    {
        Ok(resp) if resp.status().is_success() => {
            let u = resp
                .json::<UsuarioPublico>()
                .map_err(|_| ErrorDominio::nuevo("general", "Error al leer perfil."))?;
            Ok(EstadoSesionApp {
                activa: true,
                vencida: false,
                mensaje_aviso: None,
                usuario: Some(u),
            })
        }
        Ok(resp) if resp.status().as_u16() == 401 => {
            borrar_token_seguro(ruta_fallback);
            Ok(EstadoSesionApp {
                activa: false,
                vencida: true,
                mensaje_aviso: Some("Tu sesión venció, inicia sesión de nuevo".into()),
                usuario: None,
            })
        }
        _ => Ok(EstadoSesionApp {
            activa: false,
            vencida: false,
            mensaje_aviso: None,
            usuario: None,
        }),
    }
}