//! Punto de entrada de la libreria principal de la aplicacion Tauri.

// Modulo de dominio con entidades y logica de negocio
pub mod dominio;

// Modulo de aplicacion con los puertos y casos de uso
pub mod aplicacion;

// Modulo de infraestructura para implementar en local SQLite
pub mod sqlite_repo;

// Importa el adaptador SQLite para el almacenamiento local
use sqlite_repo::RepositorioSqlite;
// Importa Mutex para sincronizar el acceso al estado en Tauri
use std::sync::Mutex;
// Importa los tipos de Tauri para gestionar el estado local
use tauri::Manager;

// Estado compartido de la aplicacion que protege el repositorio local
pub struct AppState {
    // Instancia del repositorio SQLite protegida por un Mutex
    pub repo: Mutex<RepositorioSqlite>,
}

// Configura el almacenamiento local e inicia la aplicacion Tauri
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Obtiene el directorio de datos local del dispositivo
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("No se pudo obtener app_data_dir");

            // Define la ruta del archivo SQLite embedido
            let db_path = app_dir.join("cuentas_claras.sqlite");

            // Inicializa la base de datos local y sus triggers
            let repo =
                RepositorioSqlite::inicializar(db_path).expect("No se pudo inicializar SQLite");

            // Registra el estado compartido dentro de la aplicacion Tauri
            app.manage(AppState {
                repo: Mutex::new(repo),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("Error al ejecutar la aplicacion Tauri");
}
