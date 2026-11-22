use crate::{
    AppState, PingState,
    autenticacion::AuthState,
    domain::ErrorDominio,
    sqlite_repo::{EventoCambioRecibido, GastoEnColaDb, PendienteAplicarDb},
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultadoSync {
    pub ultima_sincronizacion: Option<String>,
    pub aplicados: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendienteAplicar {
    pub id: String,
    pub gasto_id: String,
    pub grupo_id: String,
    pub concepto: String,
    pub fecha: String,
    pub moneda: String,
    pub monto_parte: String,
    pub recibio_residuo: bool,
}

impl From<PendienteAplicarDb> for PendienteAplicar {
    fn from(p: PendienteAplicarDb) -> Self {
        Self {
            id: p.id,
            gasto_id: p.gasto_id,
            grupo_id: p.grupo_id,
            concepto: p.concepto,
            fecha: p.fecha,
            moneda: p.moneda,
            monto_parte: p.monto_parte,
            recibio_residuo: p.recibio_residuo,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GastoEnCola {
    pub clave: String,
    pub grupo_id: String,
    pub concepto: String,
    pub monto: String,
    pub estado: String,
    pub motivo: Option<String>,
}

impl From<GastoEnColaDb> for GastoEnCola {
    fn from(g: GastoEnColaDb) -> Self {
        Self {
            clave: g.clave,
            grupo_id: g.grupo_id,
            concepto: g.concepto,
            monto: g.monto,
            estado: g.estado,
            motivo: g.motivo,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RespuestaCambiosServidor {
    pub eventos: Vec<EventoCambioRecibido>,
    pub nuevo_cursor: i64,
    pub hay_mas: bool,
}

#[tauri::command]
pub fn obtener_cuenta_grupo(
    state: State<'_, AppState>,
    grupo_id: String,
) -> Result<Option<String>, ErrorDominio> {
    let repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
    repo.obtener_cuenta_grupo(&grupo_id)
        .map_err(|e| ErrorDominio::nuevo("general", &e))
}

#[tauri::command]
pub fn elegir_cuenta_grupo(
    state: State<'_, AppState>,
    grupo_id: String,
    cuenta_id: String,
) -> Result<(), ErrorDominio> {
    let mut repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
    repo.elegir_cuenta_grupo(&grupo_id, &cuenta_id)
        .map_err(|e| ErrorDominio::nuevo("general", &e))?;
    Ok(())
}

#[tauri::command]
pub fn listar_pendientes_aplicar(
    state: State<'_, AppState>,
) -> Result<Vec<PendienteAplicar>, ErrorDominio> {
    let repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
    let pendientes = repo
        .listar_pendientes_aplicar()
        .map_err(|e| ErrorDominio::nuevo("general", &e))?;
    Ok(pendientes.into_iter().map(PendienteAplicar::from).collect())
}

#[tauri::command]
pub fn listar_cola_gastos(
    state: State<'_, AppState>,
) -> Result<Vec<GastoEnCola>, ErrorDominio> {
    let repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
    let cola = repo
        .listar_cola_gastos()
        .map_err(|e| ErrorDominio::nuevo("general", &e))?;
    Ok(cola.into_iter().map(GastoEnCola::from).collect())
}

#[tauri::command]
pub fn descartar_rechazo(
    state: State<'_, AppState>,
    clave: String,
) -> Result<(), ErrorDominio> {
    let mut repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
    repo.eliminar_de_cola(&clave)
        .map_err(|e| ErrorDominio::nuevo("general", &e))
}

#[tauri::command]
pub fn reiniciar_cursor(state: State<'_, AppState>) -> Result<(), ErrorDominio> {
    let mut repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
    repo.reiniciar_cursor_sincronizacion()
        .map_err(|e| ErrorDominio::nuevo("general", &e))
}

#[tauri::command]
pub async fn sincronizar(
    state: State<'_, AppState>,
    ping: State<'_, PingState>,
    auth: State<'_, AuthState>,
) -> Result<ResultadoSync, ErrorDominio> {
    let token = auth
        .token
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "No se pudo consultar la sesión."))?
        .clone()
        .ok_or_else(|| ErrorDominio::nuevo("sesion", "Inicia sesión para continuar."))?;

    let cliente = &ping.http_client;
    let base = "http://127.0.0.1:8000";

    // Paso 1: Enviar gastos en cola pendientes de envío (RNF-17, ESC-09)
    let pendientes_cola = {
        let repo = state
            .repo
            .lock()
            .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
        repo.listar_cola_gastos()
            .map_err(|e| ErrorDominio::nuevo("general", &e))?
    };

    for item in pendientes_cola {
        if item.estado != "pendiente" {
            continue;
        }

        let payload: Value = match serde_json::from_str(&item.datos_json) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let resp = cliente
            .post(format!("{base}/grupos/{}/gastos", item.grupo_id))
            .bearer_auth(&token)
            .header("Idempotency-Key", &item.clave)
            .json(&payload)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await;

        match resp {
            Ok(response) => {
                if response.status() == StatusCode::UNAUTHORIZED {
                    return Err(ErrorDominio::nuevo(
                        "sesion",
                        "Tu sesión venció, inicia sesión de nuevo.",
                    ));
                }

                if response.status().is_success() {
                    // 201 Created o 200 OK (ya procesado)
                    let mut repo = state.repo.lock().map_err(|_| {
                        ErrorDominio::nuevo("general", "Error de acceso al almacenamiento.")
                    })?;
                    let _ = repo.eliminar_de_cola(&item.clave);
                } else {
                    // Error de validación o conflicto del servidor
                    let error_body = response.json::<Value>().await.unwrap_or_default();
                    let motivo = error_body
                        .get("mensaje")
                        .and_then(Value::as_str)
                        .unwrap_or("El servidor rechazó la operación.")
                        .to_string();

                    let mut repo = state.repo.lock().map_err(|_| {
                        ErrorDominio::nuevo("general", "Error de acceso al almacenamiento.")
                    })?;
                    let _ = repo.marcar_gasto_rechazado(&item.clave, &motivo);
                }
            }
            Err(_) => {
                // Falla de red: se detiene el envío de cola y se reintentará luego
                break;
            }
        }
    }

    // Paso 2: Descargar cambios incrementales desde el cursor guardado (GET /cambios)
    let mut total_aplicados = 0;
    loop {
        let cursor_actual = {
            let repo = state.repo.lock().map_err(|_| {
                ErrorDominio::nuevo("general", "Error de acceso al almacenamiento.")
            })?;
            repo.obtener_cursor_sincronizacion()
                .map_err(|e| ErrorDominio::nuevo("general", &e))?
        };

        let response = match cliente
            .get(format!("{base}/cambios"))
            .query(&[("desde", cursor_actual.to_string())])
            .bearer_auth(&token)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => {
                return Err(ErrorDominio::nuevo(
                    "red",
                    "No se pudo conectar con el servidor. Las cuentas siguen disponibles sin conexión.",
                ));
            }
        };

        if response.status() == StatusCode::UNAUTHORIZED {
            return Err(ErrorDominio::nuevo(
                "sesion",
                "Tu sesión venció, inicia sesión de nuevo.",
            ));
        }

        if !response.status().is_success() {
            let body = response.json::<Value>().await.unwrap_or_default();
            let msg = body
                .get("mensaje")
                .and_then(Value::as_str)
                .unwrap_or("No se pudieron consultar los cambios.");
            return Err(ErrorDominio::nuevo("servidor", msg));
        }

        let lote = response
            .json::<RespuestaCambiosServidor>()
            .await
            .map_err(|_| {
                ErrorDominio::nuevo("servidor", "No se pudo interpretar la respuesta del servidor.")
            })?;

        let aplicados_lote = {
            let mut repo = state.repo.lock().map_err(|_| {
                ErrorDominio::nuevo("general", "Error de acceso al almacenamiento.")
            })?;
            repo.aplicar_lote_cambios(lote.nuevo_cursor, lote.eventos)
                .map_err(|e| ErrorDominio::nuevo("general", &e))?
        };

        total_aplicados += aplicados_lote;

        if !lote.hay_mas {
            break;
        }
    }

    let ultima_sync = {
        let repo = state
            .repo
            .lock()
            .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
        repo.obtener_ultima_sincronizacion()
            .map_err(|e| ErrorDominio::nuevo("general", &e))?
    };

    Ok(ResultadoSync {
        ultima_sincronizacion: ultima_sync,
        aplicados: total_aplicados,
    })
}
