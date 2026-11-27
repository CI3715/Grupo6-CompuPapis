mod autenticacion;
mod diagnostico_libro;
pub mod respaldo;
pub mod sincronizacion;
pub mod sincronizacion_personal;
mod social;
use crate::domain::RegistrarMovimiento;
pub mod domain;
pub mod sqlite_repo;
pub mod tasas_monedas;

#[cfg(test)]
mod pruebas_entrega8;

#[cfg(test)]
mod pruebas_movimientos;

#[cfg(test)]
mod pruebas_integracion;

#[cfg(test)]
mod pruebas_patrimonio;

// Registra el módulo de pruebas de tasas y monedas para cargo test
#[cfg(test)]
mod pruebas_sincronizacion;
#[cfg(test)]
mod pruebas_tasas_cambio;
#[cfg(test)]
mod pruebas_tasas_monedas;

// Importa los tipos y casos de uso del dominio, incluyendo consulta de patrimonio
use domain::{
    Categoria, Cuenta, DetalleCuenta, ErrorDominio, Moneda, ResumenCuentas, TasaCambio,
    ejecutar_corregir_movimiento, ejecutar_crear_categoria, ejecutar_crear_cuenta,
    ejecutar_listar_categorias, ejecutar_listar_cuentas, ejecutar_listar_monedas,
    ejecutar_obtener_detalle_cuenta, ejecutar_obtener_moneda_base, ejecutar_registrar_movimiento,
};

use sqlite_repo::RepositorioSqlite;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};

pub struct PingState {
    pub http_client: reqwest::Client,
}

pub struct AppState {
    pub repo: Mutex<RepositorioSqlite>,
    pub server_url: String,
    pub ruta_token_fallback: PathBuf,
    pub app_dir: PathBuf,
    pub dispositivo_id: String,
}

// Obtiene la fecha y hora UTC del sistema
struct RelojSistema;

impl tasas_monedas::Reloj for RelojSistema {
    // Retorna el instante actual en UTC consultando el reloj del sistema
    fn ahora(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now()
    }
}

// Adaptador real que genera identificadores únicos
struct GeneradorUuid;

impl tasas_monedas::GeneradorId for GeneradorUuid {
    // Genera un identificador único
    fn nuevo_id(&self) -> String {
        let id = uuid::Uuid::new_v4();
        id.to_string()
    }
}

// Red explícita y separada del libro financiero local.
#[derive(serde::Deserialize)]
struct PingResponse {
    estado: String,
    mensaje: String,
    version: String,
    timestamp: String,
}

#[derive(serde::Serialize)]
struct PingResult {
    estado: String,
    mensaje: String,
    version: String,
    timestamp: String,
    latencia_ms: u128,
}

#[tauri::command]
async fn ping_servidor(state: State<'_, PingState>) -> Result<PingResult, String> {
    let inicio = std::time::Instant::now();
    let ping = state
        .http_client
        .get("http://127.0.0.1:8000/ping")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|_| {
            "No se pudo conectar con el servidor. Las cuentas siguen disponibles sin conexión."
                .to_string()
        })?
        .error_for_status()
        .map_err(|_| "El servidor respondió con un error.".to_string())?
        .json::<PingResponse>()
        .await
        .map_err(|_| "No se pudo leer la respuesta del servidor.".to_string())?;
    Ok(PingResult {
        estado: ping.estado,
        mensaje: ping.mensaje,
        version: ping.version,
        timestamp: ping.timestamp,
        latencia_ms: inicio.elapsed().as_millis(),
    })
}

#[tauri::command]
fn fecha_local_actual() -> String {
    chrono::Local::now().date_naive().to_string()
}

#[tauri::command]
fn evaluar_movimiento(
    state: State<'_, AppState>,
    cuenta_id: String,
    tipo: String,
    monto: String,
) -> Result<domain::EvaluacionMovimiento, ErrorDominio> {
    let repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
    domain::evaluar_movimiento(&*repo, &cuenta_id, &tipo, &monto)
}

#[tauri::command]
fn listar_monedas(state: State<'_, AppState>) -> Result<Vec<Moneda>, ErrorDominio> {
    let repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;

    ejecutar_listar_monedas(&*repo)
}

#[tauri::command]
fn crear_moneda(
    state: State<'_, AppState>,
    codigo: String,
    nombre: String,
    simbolo: String,
    decimales: String,
) -> Result<Moneda, ErrorDominio> {
    let mut repo = match state.repo.lock() {
        Ok(r) => r,
        Err(_) => {
            return Err(ErrorDominio::nuevo(
                "general",
                "Error de acceso al almacenamiento.",
            ));
        }
    };

    let moneda = tasas_monedas::ejecutar_crear_moneda(
        &mut *repo,
        &codigo,
        &nombre,
        &simbolo,
        decimales
            .parse::<u32>()
            .map_err(|_| ErrorDominio::nuevo("decimales", "Indica un entero de 0 a 8."))?,
        &state.dispositivo_id,
        &RelojSistema,
        &GeneradorUuid,
    )?;

    Ok(moneda)
}

#[tauri::command]
fn registrar_tasa(
    state: State<'_, AppState>,
    origen: Option<String>,
    destino: Option<String>,
    valor: Option<String>,
    fecha: Option<String>,
    fuente: Option<String>,
) -> Result<TasaCambio, ErrorDominio> {
    let mut repo = match state.repo.lock() {
        Ok(r) => r,
        Err(_) => {
            return Err(ErrorDominio::nuevo(
                "general",
                "No se pudieron cargar los datos.",
            ));
        }
    };

    tasas_monedas::registrar_tasa_desde_entrada(
        &mut *repo,
        tasas_monedas::EntradaTasa {
            origen,
            destino,
            valor,
            fecha,
            fuente,
        },
        &state.dispositivo_id,
        &RelojSistema,
        &GeneradorUuid,
    )
}

#[tauri::command]
fn listar_tasas(state: State<'_, AppState>) -> Result<Vec<TasaCambio>, ErrorDominio> {
    let repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
    domain::ejecutar_listar_tasas(&*repo)
}

#[tauri::command]
fn consultar_patrimonio(
    state: State<'_, AppState>,
    fecha: Option<String>,
) -> Result<domain::Patrimonio, ErrorDominio> {
    let repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
    domain::ejecutar_consultar_patrimonio(
        &*repo,
        fecha.as_deref(),
        chrono::Local::now().date_naive(),
    )
}

#[tauri::command]
fn historial_tasas(
    state: State<'_, AppState>,
    origen: String,
    destino: String,
) -> Result<Vec<TasaCambio>, ErrorDominio> {
    let repo = match state.repo.lock() {
        Ok(r) => r,
        Err(_) => {
            return Err(ErrorDominio::nuevo(
                "general",
                "No se pudieron cargar los datos.",
            ));
        }
    };

    let resp = tasas_monedas::ejecutar_historial_tasas(&*repo, &origen, &destino)?;

    Ok(resp)
}

#[tauri::command]
fn eliminar_tasa(state: State<'_, AppState>, tasa_id: String) -> Result<(), ErrorDominio> {
    let mut repo = match state.repo.lock() {
        Ok(r) => r,
        Err(_) => {
            return Err(ErrorDominio::nuevo(
                "general",
                "No se pudieron cargar los datos.",
            ));
        }
    };

    tasas_monedas::ejecutar_eliminar_tasa(
        &mut *repo,
        &tasa_id,
        &state.dispositivo_id,
        &RelojSistema,
        &GeneradorUuid,
    )
}

#[tauri::command]
fn obtener_moneda_base(state: State<'_, AppState>) -> Result<String, ErrorDominio> {
    let repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "No se pudieron cargar los datos."))?;

    let resp = ejecutar_obtener_moneda_base(&*repo)?;

    Ok(resp)
}

#[tauri::command]
fn cambiar_moneda_base(state: State<'_, AppState>, codigo: String) -> Result<String, ErrorDominio> {
    let mut repo;

    match state.repo.lock() {
        Ok(r) => {
            repo = r;
        }
        Err(_) => {
            return Err(ErrorDominio::nuevo(
                "general",
                "No se pudieron cargar los datos.",
            ));
        }
    }

    let resp = tasas_monedas::ejecutar_cambiar_moneda_base(
        &mut *repo,
        &codigo,
        &state.dispositivo_id,
        &RelojSistema,
        &GeneradorUuid,
    )?;

    Ok(resp)
}

#[tauri::command]
fn listar_cuentas(state: State<'_, AppState>) -> Result<ResumenCuentas, ErrorDominio> {
    let repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
    ejecutar_listar_cuentas(&*repo)
}

#[tauri::command]
async fn diagnosticar_libro(
    state: State<'_, AppState>,
) -> Result<diagnostico_libro::DiagnosticoLibro, ErrorDominio> {
    use crate::domain::RepositorioEventos;
    let repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("almacenamiento", "No se pudo acceder al libro."))?
        .clone();
    tokio::task::spawn_blocking(move || {
        let eventos = repo.obtener_eventos().map_err(|_| {
            ErrorDominio::nuevo(
                "almacenamiento",
                "No se pudieron leer los eventos del libro.",
            )
        })?;
        Ok(diagnostico_libro::diagnosticar(&eventos))
    })
    .await
    .map_err(|_| ErrorDominio::nuevo("almacenamiento", "No se pudo completar el diagnóstico."))?
}

#[tauri::command]
fn crear_cuenta(
    state: State<'_, AppState>,
    nombre: String,
    tipo: String,
    moneda: String,
    saldo_inicial: String,
) -> Result<Cuenta, ErrorDominio> {
    let mut repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
    ejecutar_crear_cuenta(
        &mut *repo,
        &nombre,
        &tipo,
        &moneda,
        &saldo_inicial,
        &state.dispositivo_id,
    )
}

#[tauri::command]
fn obtener_detalle_cuenta(
    state: State<'_, AppState>,
    cuenta_id: String,
) -> Result<DetalleCuenta, ErrorDominio> {
    let repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
    ejecutar_obtener_detalle_cuenta(&*repo, &cuenta_id)
}

#[tauri::command]
fn listar_categorias(state: State<'_, AppState>) -> Result<Vec<Categoria>, ErrorDominio> {
    let repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
    ejecutar_listar_categorias(&*repo)
}

#[tauri::command]
fn crear_categoria(
    state: State<'_, AppState>,
    nombre: String,
    tipo: String,
) -> Result<Categoria, ErrorDominio> {
    let mut repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
    ejecutar_crear_categoria(&mut *repo, &nombre, &tipo, &state.dispositivo_id)
}

#[tauri::command]
fn registrar_movimiento(
    state: State<'_, AppState>,
    cuenta_id: String,
    tipo: String,
    monto: String,
    fecha: String,
    categoria: String,
    nota: Option<String>,
) -> Result<DetalleCuenta, ErrorDominio> {
    let mut repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
    ejecutar_registrar_movimiento(
        &mut *repo,
        RegistrarMovimiento {
            cuenta_id: &cuenta_id,
            tipo_str: &tipo,
            monto_str: &monto,
            fecha_str: &fecha,
            categoria_str: &categoria,
            nota_opt: nota,
            dispositivo_id: &state.dispositivo_id,
        },
    )
}

#[tauri::command]
fn corregir_movimiento(
    state: State<'_, AppState>,
    cuenta_id: String,
    movimiento_id: String,
    monto_correcto: String,
) -> Result<DetalleCuenta, ErrorDominio> {
    let mut repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
    ejecutar_corregir_movimiento(
        &mut *repo,
        &cuenta_id,
        &movimiento_id,
        &monto_correcto,
        &state.dispositivo_id,
    )
}

// --- COMANDOS ENTREGA 8: COPIA DE SEGURIDAD Y SINCRONIZACIÓN PERSONAL ---

#[tauri::command]
fn ruta_respaldo_sugerida(state: State<'_, AppState>, cifrado: bool) -> String {
    let fecha = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let ext = if cifrado { "ccbackup" } else { "ndjson" };
    state
        .app_dir
        .join("respaldos")
        .join(format!("cuentas-claras-{fecha}.{ext}"))
        .to_string_lossy()
        .to_string()
}

#[tauri::command]
async fn exportar_copia_seguridad(
    state: State<'_, AppState>,
    ruta: String,
    cifrado: bool,
    frase: Option<String>,
    confirmacion: Option<String>,
) -> Result<respaldo::ResumenExportacion, ErrorDominio> {
    if ruta.trim().is_empty() {
        return Err(ErrorDominio::nuevo(
            "ruta",
            "Indica la ruta donde guardar el archivo de copia de seguridad.",
        ));
    }
    let repo_clon = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?
        .clone();
    tokio::task::spawn_blocking(move || {
        respaldo::exportar_a_archivo(
            &repo_clon,
            std::path::Path::new(ruta.trim()),
            cifrado,
            frase.as_deref(),
            confirmacion.as_deref(),
        )
    })
    .await
    .map_err(|_| ErrorDominio::nuevo("exportacion", "Error interno al exportar la copia."))?
}

#[tauri::command]
fn verificar_archivo_cifrado(ruta: String) -> Result<bool, ErrorDominio> {
    let bytes = std::fs::read(ruta.trim()).map_err(|_| {
        ErrorDominio::nuevo(
            "ruta",
            "No se pudo abrir el archivo indicado. Verifica que exista.",
        )
    })?;
    Ok(respaldo::es_archivo_cifrado(&bytes))
}

#[tauri::command]
async fn importar_copia_seguridad(
    state: State<'_, AppState>,
    ruta: String,
    frase: Option<String>,
) -> Result<domain::ResultadoFusion, ErrorDominio> {
    if ruta.trim().is_empty() {
        return Err(ErrorDominio::nuevo(
            "ruta",
            "Indica la ruta del archivo de copia de seguridad a importar.",
        ));
    }
    let mut repo_clon = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?
        .clone();
    tokio::task::spawn_blocking(move || {
        respaldo::importar_desde_archivo(
            &mut repo_clon,
            std::path::Path::new(ruta.trim()),
            frase.as_deref(),
        )
    })
    .await
    .map_err(|_| ErrorDominio::nuevo("importacion", "Error interno al importar la copia."))?
}

#[tauri::command]
fn obtener_config_sync_personal(
    state: State<'_, AppState>,
) -> Result<sincronizacion_personal::EstadoConfigSyncPersonal, ErrorDominio> {
    let repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
    let secretos = sincronizacion_personal::AlmacenSecretosSo::nuevo(&state.dispositivo_id);
    sincronizacion_personal::obtener_estado_config_sync(&repo, &secretos)
}

#[tauri::command]
fn guardar_config_sync_personal(
    state: State<'_, AppState>,
    tipo: sincronizacion_personal::TipoDestinoSync,
    destino: String,
    rama: Option<String>,
    usuario_git: Option<String>,
    frase: Option<String>,
    credencial_git: Option<String>,
) -> Result<sincronizacion_personal::EstadoConfigSyncPersonal, ErrorDominio> {
    let repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
    let secretos = sincronizacion_personal::AlmacenSecretosSo::nuevo(&state.dispositivo_id);
    let config = sincronizacion_personal::ConfigSyncPersonal {
        tipo,
        destino,
        rama,
        usuario_git,
    };
    sincronizacion_personal::guardar_configuracion_sync(
        &repo,
        &secretos,
        &config,
        frase.as_deref(),
        credencial_git.as_deref(),
    )
}

#[tauri::command]
fn limpiar_config_sync_personal(state: State<'_, AppState>) -> Result<(), ErrorDominio> {
    let repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
    let secretos = sincronizacion_personal::AlmacenSecretosSo::nuevo(&state.dispositivo_id);
    sincronizacion_personal::limpiar_configuracion_sync(&repo, &secretos)
}

#[tauri::command]
async fn sincronizar_personal_ahora(
    state: State<'_, AppState>,
) -> Result<sincronizacion_personal::InformeSyncPersonal, ErrorDominio> {
    let mut repo_clon = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?
        .clone();
    let app_dir = state.app_dir.clone();
    let dispositivo_id = state.dispositivo_id.clone();

    tokio::task::spawn_blocking(move || {
        let secretos = sincronizacion_personal::AlmacenSecretosSo::nuevo(&dispositivo_id);
        sincronizacion_personal::sincronizar_con_destino_configurado(
            &mut repo_clon,
            &secretos,
            &app_dir,
            respaldo::ParametrosKdf::estandar(),
        )
    })
    .await
    .map_err(|_| ErrorDominio::nuevo("sincronizacion", "Error interno al sincronizar."))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(autenticacion::AuthState::default())
        .manage(PingState {
            http_client: reqwest::Client::new(),
        })
        .setup(|app| {
            let app_dir = match std::env::var("CUENTAS_CLARAS_DATA_DIR") {
                Ok(dir) if !dir.trim().is_empty() => PathBuf::from(dir),
                _ => app
                    .path()
                    .app_data_dir()
                    .expect("No se pudo obtener app_data_dir"),
            };
            let db_path = app_dir.join("cuentas_claras.sqlite");
            let ruta_token_fallback = app_dir.join(".sesion_token");
            let server_url = std::env::var("CUENTAS_CLARAS_API_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8000".to_string());
            println!("===> RUTA DE SQLITE: {:?}", db_path);

            if std::env::var("CUENTAS_CLARAS_DATA_DIR").is_err()
                && let Ok(directorio_datos) = app.path().data_dir()
            {
                let anterior = directorio_datos
                    .join("com.tauri.dev")
                    .join("cuentas_claras.sqlite");
                sqlite_repo::migrar_libro_anterior(&anterior, &db_path)
                    .map_err(std::io::Error::other)?;
            }
            let repo = RepositorioSqlite::inicializar(db_path)
                .expect("No se pudo inicializar el archivo SQLite local");
            let dispositivo_id = repo
                .dispositivo_id()
                .expect("No se pudo obtener el identificador del dispositivo");

            app.manage(AppState {
                repo: Mutex::new(repo),
                server_url,
                ruta_token_fallback,
                app_dir,
                dispositivo_id,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            diagnosticar_libro,
            ping_servidor,
            autenticacion::registrar_usuario,
            autenticacion::iniciar_sesion,
            autenticacion::obtener_perfil,
            autenticacion::cerrar_sesion,
            social::operacion_social,
            fecha_local_actual,
            evaluar_movimiento,
            listar_monedas,
            crear_moneda,
            registrar_tasa,
            historial_tasas,
            listar_tasas,
            consultar_patrimonio,
            eliminar_tasa,
            obtener_moneda_base,
            cambiar_moneda_base,
            listar_cuentas,
            crear_cuenta,
            obtener_detalle_cuenta,
            listar_categorias,
            crear_categoria,
            registrar_movimiento,
            corregir_movimiento,
            sincronizacion::sincronizar,
            sincronizacion::obtener_ultima_sincronizacion,
            sincronizacion::vincular_datos_anteriores,
            sincronizacion::obtener_cuenta_grupo,
            sincronizacion::elegir_cuenta_grupo,
            sincronizacion::listar_pendientes_aplicar,
            sincronizacion::listar_cola_gastos,
            sincronizacion::descartar_rechazo,
            sincronizacion::reiniciar_cursor,
            ruta_respaldo_sugerida,
            exportar_copia_seguridad,
            verificar_archivo_cifrado,
            importar_copia_seguridad,
            obtener_config_sync_personal,
            guardar_config_sync_personal,
            limpiar_config_sync_personal,
            sincronizar_personal_ahora,
        ])
        .run(tauri::generate_context!())
        .expect("Error al ejecutar la aplicación Tauri");
}
