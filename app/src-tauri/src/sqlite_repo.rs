use crate::domain::{DatosCuentaCreada, EventoLibro, RepositorioEventos};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::PathBuf;

pub struct RepositorioSqlite {
    ruta_db: PathBuf,
}

impl RepositorioSqlite {
    pub fn inicializar(ruta_db: PathBuf) -> Result<Self, String> {
        if let Some(parent) = ruta_db.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let conn = Connection::open(&ruta_db).map_err(|e| e.to_string())?;
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
             END;"
        )
        .map_err(|e| e.to_string())?;
        Ok(Self { ruta_db })
    }

    fn abrir(&self) -> Result<Connection, String> {
        Connection::open(&self.ruta_db).map_err(|e| e.to_string())
    }
}

impl RepositorioEventos for RepositorioSqlite {
    fn guardar_evento(&mut self, evento: &EventoLibro) -> Result<(), String> {
        let conn = self.abrir()?;
        let datos_json = serde_json::to_string(&evento.datos).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR IGNORE INTO eventos_libro (id, tipo, momento_registro, dispositivo_origen, datos_json)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                evento.id,
                evento.tipo,
                evento.momento_registro.to_rfc3339(),
                evento.dispositivo_origen,
                datos_json
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn obtener_eventos(&self) -> Result<Vec<EventoLibro>, String> {
        let conn = self.abrir()?;
        let mut stmt = conn
            .prepare("SELECT id, tipo, momento_registro, dispositivo_origen, datos_json FROM eventos_libro ORDER BY rowid ASC")
            .map_err(|e| e.to_string())?;

        let filas = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let tipo: String = row.get(1)?;
                let momento_str: String = row.get(2)?;
                let dispositivo_origen: String = row.get(3)?;
                let datos_json: String = row.get(4)?;
                Ok((id, tipo, momento_str, dispositivo_origen, datos_json))
            })
            .map_err(|e| e.to_string())?;

        let mut eventos = Vec::new();
        for fila in filas {
            let (id, tipo, momento_str, dispositivo_origen, datos_json) = fila.map_err(|e| e.to_string())?;
            let momento_registro = DateTime::parse_from_rfc3339(&momento_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let datos: DatosCuentaCreada = serde_json::from_str(&datos_json).map_err(|e| e.to_string())?;
            eventos.push(EventoLibro {
                id,
                tipo,
                momento_registro,
                dispositivo_origen,
                datos,
            });
        }
        Ok(eventos)
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::domain::ejecutar_crear_cuenta;

    #[test]
    fn probar_inmutabilidad_eventos() {
        
        // Creamos una BD de prueba
        let ruta = std::env::temp_dir().join(
            format!("prueba_{}.db", uuid::Uuid::new_v4())
        );

        let mut repo = RepositorioSqlite::inicializar(ruta.clone()).unwrap();

        // Creamos una cuenta y guardamos su evento
        ejecutar_crear_cuenta(
            &mut repo, "Cartera", "efectivo",
            "VES", "100.00", "prueba"
        ).unwrap();

        let conn = Connection::open(&ruta).unwrap();

        // Vemos si se puede cambiar el evento
        let cambiar = conn.execute(
            "UPDATE eventos_libro SET tipo = 'Otro' WHERE tipo = 'CuentaCreada'",
            []
        );
        assert!(cambiar.is_err());

        // Vemos si se puede borrar el evento
        let borrar = conn.execute(
            "DELETE FROM eventos_libro WHERE tipo = 'CuentaCreada'",
            []
        );
        assert!(borrar.is_err());

        // Vemos si el evento sigue guardado
        let eventos = repo.obtener_eventos().unwrap();
        assert_eq!(eventos.len(), 1);
        assert_eq!(eventos[0].tipo, "CuentaCreada");

        // Borramos la BD de prueba
        drop(conn);
        drop(repo);
        std::fs::remove_file(ruta).unwrap();
    }
}