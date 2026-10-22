//! Punto de entrada de la libreria principal de la aplicacion Tauri.

// Modulo de dominio con entidades y logica de negocio
pub mod dominio;

use serde::{Deserialize, Serialize};

// -----------------------------------------------------------------------------

pub mod domain;
pub mod sqlite_repo;

use domain::{
    catalogo_monedas, ejecutar_crear_cuenta, ejecutar_listar_cuentas, Cuenta, ErrorDominio, Moneda,
    ResumenCuentas,
};
use sqlite_repo::RepositorioSqlite;
use std::sync::Mutex;
use tauri::{Manager, State};

pub struct AppState {
    
    pub repo: Mutex<RepositorioSqlite>,
}

// ----------------------------------------------------------------------------

// Comando para comprobar conexion con el servidor
#[tauri::command]
fn listar_monedas() -> Vec<Moneda> {
    catalogo_monedas()
}

#[tauri::command]
fn listar_cuentas(state: State<'_, AppState>) -> Result<ResumenCuentas, ErrorDominio> {
    let repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento local."))?;
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
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento local."))?;
    ejecutar_crear_cuenta(
        &mut *repo,
        &nombre,
        &tipo,
        &moneda,
        &saldo_inicial,
        "dispositivo-local",
    )
}

// Configura el restado local, registra comandos e inicia la aplicacion Tauri
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("No se pudo obtener app_data_dir");
            let db_path = app_dir.join("cuentas_claras.sqlite");
            let repo = RepositorioSqlite::inicializar(db_path)
                .expect("No se pudo inicializar SQLite");

            app.manage(AppState {
                repo: Mutex::new(repo),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            listar_monedas,
            listar_cuentas,
            crear_cuenta
        ])
        .run(tauri::generate_context!())
        .expect("Error al ejecutar la aplicación Tauri");
}