use crate::domain::RegistrarMovimiento;
pub mod domain;
pub mod sqlite_repo;
pub mod tasas_monedas;

#[cfg(test)]
mod pruebas_movimientos;

#[cfg(test)]
mod pruebas_integracion;

#[cfg(test)]
mod pruebas_patrimonio;

// Registra el módulo de pruebas de tasas y monedas para cargo test
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
use std::sync::Mutex;
use tauri::{Manager, State};

struct PingState {
    http_client: reqwest::Client,
}

pub struct AppState {
    pub repo: Mutex<RepositorioSqlite>,
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
        "dispositivo-local",
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
        "dispositivo-local",
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

    // Buscamos las tasas de esas 2 monedas
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

    // Eliminamos la tasa
    tasas_monedas::ejecutar_eliminar_tasa(
        &mut *repo,
        &tasa_id,
        "dispositivo-local",
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

    // Vemos cuál es la moneda base
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

    // Cambiamos la moneda base
    let resp = tasas_monedas::ejecutar_cambiar_moneda_base(
        &mut *repo,
        &codigo,
        "dispositivo-local",
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
        "dispositivo-local",
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
    ejecutar_crear_categoria(&mut *repo, &nombre, &tipo, "dispositivo-local")
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
            dispositivo_id: "dispositivo-local",
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
        "dispositivo-local",
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(PingState {
            http_client: reqwest::Client::new(),
        })
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("No se pudo obtener app_data_dir");
            let db_path = app_dir.join("cuentas_claras.sqlite");
            println!("===> RUTA DE SQLITE: {:?}", db_path);

            if let Ok(directorio_datos) = app.path().data_dir() {
                let anterior = directorio_datos
                    .join("com.tauri.dev")
                    .join("cuentas_claras.sqlite");
                sqlite_repo::migrar_libro_anterior(&anterior, &db_path)
                    .map_err(std::io::Error::other)?;
            }
            let repo = RepositorioSqlite::inicializar(db_path)
                .expect("No se pudo inicializar el archivo SQLite local");

            app.manage(AppState {
                repo: Mutex::new(repo),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ping_servidor,
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
            corregir_movimiento
        ])
        .run(tauri::generate_context!())
        .expect("Error al ejecutar la aplicación Tauri");
}
