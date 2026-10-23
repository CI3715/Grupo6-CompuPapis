use crate::domain::RegistrarMovimiento;
pub mod domain;
pub mod sqlite_repo;

#[cfg(test)]
mod pruebas_movimientos;

#[cfg(test)]
mod pruebas_integracion;

use domain::{
    Categoria, Cuenta, DetalleCuenta, ErrorDominio, Moneda, ResumenCuentas, catalogo_monedas,
    ejecutar_corregir_movimiento, ejecutar_crear_categoria, ejecutar_crear_cuenta,
    ejecutar_listar_categorias, ejecutar_listar_cuentas, ejecutar_obtener_detalle_cuenta,
    ejecutar_registrar_movimiento,
};
use sqlite_repo::RepositorioSqlite;
use std::sync::Mutex;
use tauri::{Manager, State};

pub struct AppState {
    pub repo: Mutex<RepositorioSqlite>,
}

#[tauri::command]
fn listar_monedas() -> Vec<Moneda> {
    catalogo_monedas()
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
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("No se pudo obtener app_data_dir");
            let db_path = app_dir.join("cuentas_claras.sqlite");
            println!("===> RUTA DE SQLITE: {:?}", db_path);

            let repo = RepositorioSqlite::inicializar(db_path)
                .expect("No se pudo inicializar el archivo SQLite local");

            app.manage(AppState {
                repo: Mutex::new(repo),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            listar_monedas,
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
