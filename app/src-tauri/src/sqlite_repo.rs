//! Adaptador SQLite que implementa el almacenamiento local de eventos

// Importa el puerto de repositorio desde la capa de aplicacion
use crate::aplicacion::RepositorioEventos;
// Importa el evento CuentaCreada desde la capa de dominio
use crate::dominio::CuentaCreada;
// Importa el conector y manejo de parametros de SQLite embedido
use rusqlite::{Connection, params};
// Importa el manejo de rutas de archivos del sistema operativo
use std::path::PathBuf;

// Repositorio de eventos respaldado por un archivo SQLite local
pub struct RepositorioSqlite {
    // Ruta del archivo SQLite dentro del directorio de datos
    ruta_db: PathBuf,
}

impl RepositorioSqlite {
    // Crea la tabla de eventos y los triggers de inmutabilidad
    pub fn inicializar(ruta_db: PathBuf) -> Result<Self, String> {
        if let Some(parent) = ruta_db.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let conn = Connection::open(&ruta_db).map_err(|e| e.to_string())?;

        // Crea la tabla donde se registran los eventos inmutables
        conn.execute(
            "CREATE TABLE IF NOT EXISTS eventos_libro (
                id TEXT PRIMARY KEY,
                tipo TEXT NOT NULL,
                momento_registro TEXT NOT NULL,
                dispositivo_origen TEXT NOT NULL,
                datos_json TEXT NOT NULL
            )",
            [],
        )
        .map_err(|e| e.to_string())?;

        // Registra triggers que impiden modificar o borrar eventos
        conn.execute_batch(
            "CREATE TRIGGER IF NOT EXISTS impedir_modificar_eventos
             BEFORE UPDATE ON eventos_libro
             BEGIN
                 SELECT RAISE(ABORT, 'Los eventos no pueden modificarse');
             END;

             CREATE TRIGGER IF NOT EXISTS impedir_eliminar_eventos
             BEFORE DELETE ON eventos_libro
             BEGIN
                 SELECT RAISE(ABORT, 'Los eventos no pueden eliminarse');
             END;",
        )
        .map_err(|e| e.to_string())?;
        Ok(Self { ruta_db })
    }

    // Abre una conexion sincrona hacia el archivo SQLite local
    fn abrir(&self) -> Result<Connection, String> {
        Connection::open(&self.ruta_db).map_err(|e| e.to_string())
    }
}

impl RepositorioEventos for RepositorioSqlite {
    // Guarda un evento CuentaCreada en la tabla local de eventos
    fn guardar_evento(&mut self, evento: &CuentaCreada) -> Result<(), String> {
        let conn = self.abrir()?;
        // Serializa el evento completo a formato JSON para persistirlo
        let datos_json = serde_json::to_string(evento).map_err(|e| e.to_string())?;

        conn.execute(
            "INSERT OR IGNORE INTO eventos_libro (id, tipo, momento_registro, dispositivo_origen, datos_json)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                evento.id_evento(),
                evento.tipo_evento(),
                evento.momento_registro(),
                evento.dispositivo_origen(),
                datos_json
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // Recupera en orden de llegada todos los eventos guardados localmente
    fn obtener_eventos(&self) -> Result<Vec<CuentaCreada>, String> {
        let conn = self.abrir()?;
        let mut stmt = conn
            .prepare("SELECT datos_json FROM eventos_libro ORDER BY rowid ASC")
            .map_err(|e| e.to_string())?;

        // Extrae el JSON de cada fila almacenada en la tabla
        let filas = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;

        let mut eventos = Vec::new();
        for fila in filas {
            let datos_json = fila.map_err(|e| e.to_string())?;
            // Reconstruye el evento CuentaCreada desde su representacion JSON
            let evento: CuentaCreada =
                serde_json::from_str(&datos_json).map_err(|e| e.to_string())?;
            eventos.push(evento);
        }
        Ok(eventos)
    }
}

#[cfg(test)]
mod pruebas {
    // BD de prueba para verificar la inmutabilidad en SQLite
    use super::*;
    use crate::dominio::{CodigoMoneda, TipoCuenta};
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn probar_inmutabilidad_eventos() {
        // Crea una base de datos temporal unica para la prueba
        let ruta = std::env::temp_dir().join(format!("prueba_{}.db", uuid::Uuid::new_v4()));
        let mut repo = RepositorioSqlite::inicializar(ruta.clone()).unwrap();

        // Construye un evento valido usando la capa de dominio
        let evento = CuentaCreada::nuevo(
            String::from("evento-01"),
            String::from("2026-10-22T12:00:00Z"),
            String::from("prueba"),
            String::from("cuenta-01"),
            String::from("Cartera"),
            TipoCuenta::Efectivo,
            CodigoMoneda::VES,
            Decimal::from_str("100.00").unwrap(),
        )
        .unwrap();

        // Guarda el evento inicial en la base de datos temporal
        repo.guardar_evento(&evento).unwrap();

        let conn = Connection::open(&ruta).unwrap();

        // Verifica que SQLite rechace cualquier intento de actualizar eventos
        let cambiar = conn.execute(
            "UPDATE evento_libro SET tipo = 'Otro' WHERE tipo = 'CuentaCreada'",
            [],
        );
        assert!(cambiar.is_err());

        // Verifica que SQLite rechace cualquier intento de eliminar eventos
        let borrar = conn.execute("DELETE FROM eventos_libro WHERE tipo = 'CuentaCreada'", []);
        assert!(borrar.is_err());

        // Comprueba que el evento original permanezca en el disco
        let eventos = repo.obtener_eventos().unwrap();
        assert_eq!(eventos.len(), 1);
        assert_eq!(eventos[0].tipo_evento(), "CuentaCreada");

        // Borra la BD temporal al terminar la prueba
        drop(conn);
        drop(repo);
        std::fs::remove_file(ruta).unwrap();
    }
}
