use crate::domain::{EventoLibro, RepositorioEventos};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendienteAplicarDb {
    pub id: String,
    pub gasto_id: String,
    pub grupo_id: String,
    pub grupo_nombre: String,
    pub concepto: String,
    pub fecha: String,
    pub moneda: String,
    pub monto_parte: String,
    pub recibio_residuo: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GastoEnColaDb {
    pub clave: String,
    pub grupo_id: String,
    pub concepto: String,
    pub monto: String,
    pub datos_json: String,
    pub estado: String,
    pub motivo: Option<String>,
    pub creado_en: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventoCambioRecibido {
    pub id: String,
    pub cursor: i64,
    pub tipo: String,
    pub grupo_id: String,
    pub grupo_nombre: String,
    pub gasto_id: String,
    pub concepto: String,
    pub fecha: String,
    pub moneda: String,
    pub monto_parte: String,
    pub incluye_centimos_redondeo: bool,
}

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

        conn.execute(
            "CREATE TABLE IF NOT EXISTS cuentas_grupos (
                grupo_id TEXT PRIMARY KEY,
                cuenta_id TEXT NOT NULL
            )",
            [],
        )
        .map_err(|e| e.to_string())?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS pendientes_aplicar (
                id TEXT PRIMARY KEY,
                gasto_id TEXT NOT NULL,
                grupo_id TEXT NOT NULL,
                grupo_nombre TEXT NOT NULL,
                concepto TEXT NOT NULL,
                fecha TEXT NOT NULL,
                moneda TEXT NOT NULL,
                monto_parte TEXT NOT NULL,
                recibio_residuo INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|e| e.to_string())?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS cola_gastos (
                clave TEXT PRIMARY KEY,
                grupo_id TEXT NOT NULL,
                concepto TEXT NOT NULL,
                monto TEXT NOT NULL,
                datos_json TEXT NOT NULL,
                estado TEXT NOT NULL,
                motivo TEXT,
                creado_en TEXT NOT NULL
            )",
            [],
        )
        .map_err(|e| e.to_string())?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS estado_sincronizacion (
                clave TEXT PRIMARY KEY,
                valor TEXT NOT NULL
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
             END;",
        )
        .map_err(|e| e.to_string())?;
        Ok(Self { ruta_db })
    }

    fn abrir(&self) -> Result<Connection, String> {
        Connection::open(&self.ruta_db).map_err(|e| e.to_string())
    }

    pub fn obtener_cuenta_grupo(&self, grupo_id: &str) -> Result<Option<String>, String> {
        let conn = self.abrir()?;
        let mut stmt = conn
            .prepare("SELECT cuenta_id FROM cuentas_grupos WHERE grupo_id = ?1")
            .map_err(|e| e.to_string())?;
        let mut filas = stmt
            .query_map([grupo_id], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        if let Some(fila) = filas.next() {
            Ok(Some(fila.map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    pub fn elegir_cuenta_grupo(
        &mut self,
        grupo_id: &str,
        cuenta_id: &str,
    ) -> Result<usize, String> {
        use std::str::FromStr;
        let mut conn = self.abrir()?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        tx.execute(
            "INSERT OR REPLACE INTO cuentas_grupos (grupo_id, cuenta_id) VALUES (?1, ?2)",
            params![grupo_id, cuenta_id],
        )
        .map_err(|e| e.to_string())?;

        let mut stmt = tx
            .prepare(
                "SELECT id, gasto_id, grupo_id, grupo_nombre, concepto, fecha, moneda, monto_parte, recibio_residuo
                 FROM pendientes_aplicar WHERE grupo_id = ?1 ORDER BY rowid ASC",
            )
            .map_err(|e| e.to_string())?;

        let pendientes: Vec<PendienteAplicarDb> = stmt
            .query_map([grupo_id], |row| {
                Ok(PendienteAplicarDb {
                    id: row.get(0)?,
                    gasto_id: row.get(1)?,
                    grupo_id: row.get(2)?,
                    grupo_nombre: row.get(3)?,
                    concepto: row.get(4)?,
                    fecha: row.get(5)?,
                    moneda: row.get(6)?,
                    monto_parte: row.get(7)?,
                    recibio_residuo: row.get::<_, i64>(8)? != 0,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?;
        drop(stmt);

        let mut aplicados = 0;
        let ahora = Utc::now();
        for p in pendientes {
            let cant = rust_decimal::Decimal::from_str(&p.monto_parte)
                .map_err(|e| e.to_string())?;
            let datos = crate::domain::DatosGastoDeGrupoAplicado {
                movimiento_id: uuid::Uuid::new_v4().to_string(),
                cuenta_id: cuenta_id.to_string(),
                evento_servidor_id: p.id.clone(),
                grupo_id: p.grupo_id.clone(),
                gasto_id: p.gasto_id.clone(),
                concepto: format!("{} · {}", p.grupo_nombre, p.concepto),
                fecha: p.fecha.clone(),
                monto: crate::domain::Monto::nuevo(cant, &p.moneda).map_err(|e| e.mensaje)?,
                incluye_centimos_redondeo: p.recibio_residuo,
            };

            let evento = EventoLibro {
                id: uuid::Uuid::new_v4().to_string(),
                tipo: "GastoDeGrupoAplicado".to_string(),
                momento_registro: ahora,
                dispositivo_origen: "dispositivo-local".to_string(),
                datos_json: serde_json::to_string(&datos).map_err(|e| e.to_string())?,
            };

            tx.execute(
                "INSERT OR IGNORE INTO eventos_libro (id, tipo, momento_registro, dispositivo_origen, datos_json)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    evento.id,
                    evento.tipo,
                    evento.momento_registro.to_rfc3339(),
                    evento.dispositivo_origen,
                    evento.datos_json
                ],
            )
            .map_err(|e| e.to_string())?;

            tx.execute("DELETE FROM pendientes_aplicar WHERE id = ?1", params![p.id])
                .map_err(|e| e.to_string())?;

            aplicados += 1;
        }

        tx.commit().map_err(|e| e.to_string())?;
        Ok(aplicados)
    }

    pub fn listar_pendientes_aplicar(&self) -> Result<Vec<PendienteAplicarDb>, String> {
        let conn = self.abrir()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, gasto_id, grupo_id, grupo_nombre, concepto, fecha, moneda, monto_parte, recibio_residuo
                 FROM pendientes_aplicar ORDER BY rowid ASC",
            )
            .map_err(|e| e.to_string())?;
        let filas = stmt
            .query_map([], |row| {
                Ok(PendienteAplicarDb {
                    id: row.get(0)?,
                    gasto_id: row.get(1)?,
                    grupo_id: row.get(2)?,
                    grupo_nombre: row.get(3)?,
                    concepto: row.get(4)?,
                    fecha: row.get(5)?,
                    moneda: row.get(6)?,
                    monto_parte: row.get(7)?,
                    recibio_residuo: row.get::<_, i64>(8)? != 0,
                })
            })
            .map_err(|e| e.to_string())?;
        filas.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn listar_cola_gastos(&self) -> Result<Vec<GastoEnColaDb>, String> {
        let conn = self.abrir()?;
        let mut stmt = conn
            .prepare(
                "SELECT clave, grupo_id, concepto, monto, datos_json, estado, motivo, creado_en
                 FROM cola_gastos ORDER BY rowid ASC",
            )
            .map_err(|e| e.to_string())?;
        let filas = stmt
            .query_map([], |row| {
                Ok(GastoEnColaDb {
                    clave: row.get(0)?,
                    grupo_id: row.get(1)?,
                    concepto: row.get(2)?,
                    monto: row.get(3)?,
                    datos_json: row.get(4)?,
                    estado: row.get(5)?,
                    motivo: row.get(6)?,
                    creado_en: row.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?;
        filas.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn encolar_gasto(
        &mut self,
        clave: &str,
        grupo_id: &str,
        concepto: &str,
        monto: &str,
        datos_json: &str,
    ) -> Result<(), String> {
        let conn = self.abrir()?;
        let creado_en = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR REPLACE INTO cola_gastos (clave, grupo_id, concepto, monto, datos_json, estado, motivo, creado_en)
             VALUES (?1, ?2, ?3, ?4, ?5, 'pendiente', NULL, ?6)",
            params![clave, grupo_id, concepto, monto, datos_json, creado_en],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn marcar_gasto_rechazado(&mut self, clave: &str, motivo: &str) -> Result<(), String> {
        let conn = self.abrir()?;
        conn.execute(
            "UPDATE cola_gastos SET estado = 'rechazado', motivo = ?1 WHERE clave = ?2",
            params![motivo, clave],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn eliminar_de_cola(&mut self, clave: &str) -> Result<(), String> {
        let conn = self.abrir()?;
        conn.execute("DELETE FROM cola_gastos WHERE clave = ?1", params![clave])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn obtener_cursor_sincronizacion(&self) -> Result<i64, String> {
        let conn = self.abrir()?;
        let mut stmt = conn
            .prepare("SELECT valor FROM estado_sincronizacion WHERE clave = 'ultimo_cursor'")
            .map_err(|e| e.to_string())?;
        let mut filas = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        if let Some(fila) = filas.next() {
            let s = fila.map_err(|e| e.to_string())?;
            s.parse::<i64>().map_err(|e| e.to_string())
        } else {
            Ok(0)
        }
    }

    pub fn guardar_cursor_y_fecha(&mut self, nuevo_cursor: i64, fecha: &str) -> Result<(), String> {
        let conn = self.abrir()?;
        conn.execute(
            "INSERT OR REPLACE INTO estado_sincronizacion (clave, valor) VALUES ('ultimo_cursor', ?1)",
            params![nuevo_cursor.to_string()],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO estado_sincronizacion (clave, valor) VALUES ('ultima_sincronizacion', ?1)",
            params![fecha],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn reiniciar_cursor_sincronizacion(&mut self) -> Result<(), String> {
        let conn = self.abrir()?;
        conn.execute(
            "INSERT OR REPLACE INTO estado_sincronizacion (clave, valor) VALUES ('ultimo_cursor', '0')",
            [],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn obtener_ultima_sincronizacion(&self) -> Result<Option<String>, String> {
        let conn = self.abrir()?;
        let mut stmt = conn
            .prepare("SELECT valor FROM estado_sincronizacion WHERE clave = 'ultima_sincronizacion'")
            .map_err(|e| e.to_string())?;
        let mut filas = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        if let Some(fila) = filas.next() {
            Ok(Some(fila.map_err(|e| e.to_string())?))
        } else {
            Ok(None)
        }
    }

    pub fn aplicar_lote_cambios(
        &mut self,
        nuevo_cursor: i64,
        eventos_cambio: Vec<EventoCambioRecibido>,
    ) -> Result<usize, String> {
        use std::str::FromStr;
        let mut conn = self.abrir()?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        let mut stmt_cuentas = tx
            .prepare("SELECT grupo_id, cuenta_id FROM cuentas_grupos")
            .map_err(|e| e.to_string())?;
        let cuentas_por_grupo: std::collections::HashMap<String, String> = stmt_cuentas
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| e.to_string())?
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?;
        drop(stmt_cuentas);

        let mut stmt_aplicados = tx
            .prepare(
                "SELECT json_extract(datos_json, '$.evento_servidor_id')
                 FROM eventos_libro WHERE tipo = 'GastoDeGrupoAplicado'",
            )
            .map_err(|e| e.to_string())?;
        let ya_aplicados: std::collections::HashSet<String> = stmt_aplicados
            .query_map([], |row| row.get::<_, Option<String>>(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok().flatten())
            .collect();
        drop(stmt_aplicados);

        let mut aplicados = 0;
        let ahora = Utc::now();

        for ev in eventos_cambio {
            if ev.tipo != "gasto_grupo" {
                continue;
            }

            if ya_aplicados.contains(&ev.id) {
                continue;
            }

            if let Some(cuenta_id) = cuentas_por_grupo.get(&ev.grupo_id) {
                let cant = rust_decimal::Decimal::from_str(&ev.monto_parte)
                    .map_err(|e| e.to_string())?;
                let datos = crate::domain::DatosGastoDeGrupoAplicado {
                    movimiento_id: uuid::Uuid::new_v4().to_string(),
                    cuenta_id: cuenta_id.clone(),
                    evento_servidor_id: ev.id.clone(),
                    grupo_id: ev.grupo_id.clone(),
                    gasto_id: ev.gasto_id.clone(),
                    concepto: format!("{} · {}", ev.grupo_nombre, ev.concepto),
                    fecha: ev.fecha.clone(),
                    monto: crate::domain::Monto::nuevo(cant, &ev.moneda)
                        .map_err(|e| e.mensaje)?,
                    incluye_centimos_redondeo: ev.incluye_centimos_redondeo,
                };

                let evento = EventoLibro {
                    id: uuid::Uuid::new_v4().to_string(),
                    tipo: "GastoDeGrupoAplicado".to_string(),
                    momento_registro: ahora,
                    dispositivo_origen: "dispositivo-local".to_string(),
                    datos_json: serde_json::to_string(&datos).map_err(|e| e.to_string())?,
                };

                tx.execute(
                    "INSERT OR IGNORE INTO eventos_libro (id, tipo, momento_registro, dispositivo_origen, datos_json)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        evento.id,
                        evento.tipo,
                        evento.momento_registro.to_rfc3339(),
                        evento.dispositivo_origen,
                        evento.datos_json
                    ],
                )
                .map_err(|e| e.to_string())?;

                tx.execute(
                    "DELETE FROM pendientes_aplicar WHERE id = ?1",
                    params![ev.id],
                )
                .map_err(|e| e.to_string())?;

                aplicados += 1;
            } else {
                tx.execute(
                    "INSERT OR REPLACE INTO pendientes_aplicar
                     (id, gasto_id, grupo_id, grupo_nombre, concepto, fecha, moneda, monto_parte, recibio_residuo)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        ev.id,
                        ev.gasto_id,
                        ev.grupo_id,
                        ev.grupo_nombre,
                        ev.concepto,
                        ev.fecha,
                        ev.moneda,
                        ev.monto_parte,
                        if ev.incluye_centimos_redondeo { 1 } else { 0 }
                    ],
                )
                .map_err(|e| e.to_string())?;
            }
        }

        tx.execute(
            "INSERT OR REPLACE INTO estado_sincronizacion (clave, valor) VALUES ('ultimo_cursor', ?1)",
            params![nuevo_cursor.to_string()],
        )
        .map_err(|e| e.to_string())?;

        tx.execute(
            "INSERT OR REPLACE INTO estado_sincronizacion (clave, valor) VALUES ('ultima_sincronizacion', ?1)",
            params![ahora.to_rfc3339()],
        )
        .map_err(|e| e.to_string())?;

        tx.commit().map_err(|e| e.to_string())?;
        Ok(aplicados)
    }
}

impl RepositorioEventos for RepositorioSqlite {
    fn guardar_evento(&mut self, evento: &EventoLibro) -> Result<(), String> {
        let conn = self.abrir()?;
        // INV-04 e INV-13: La tabla de eventos solo admite inserciones idempotentes
        let insertados = conn.execute(
            "INSERT OR IGNORE INTO eventos_libro (id, tipo, momento_registro, dispositivo_origen, datos_json)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                evento.id,
                evento.tipo,
                evento.momento_registro.to_rfc3339(),
                evento.dispositivo_origen,
                evento.datos_json
            ],
        )
        .map_err(|e| e.to_string())?;
        if insertados == 0 {
            let existente = self
                .obtener_eventos()?
                .into_iter()
                .find(|e| e.id == evento.id);
            if existente.as_ref() != Some(evento) {
                return Err("El identificador ya pertenece a otro evento.".into());
            }
        }
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
            let (id, tipo, momento_str, dispositivo_origen, datos_json) =
                fila.map_err(|e| e.to_string())?;
            let momento_registro = DateTime::parse_from_rfc3339(&momento_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|_| "El libro contiene una fecha de registro inválida.".to_string())?;
            eventos.push(EventoLibro {
                id,
                tipo,
                momento_registro,
                dispositivo_origen,
                datos_json,
            });
        }
        Ok(eventos)
    }
}

/// Copia verificada del libro anterior al cambiar el identificador de la aplicación.
/// SQLite Backup incluye páginas pendientes en WAL; el original nunca se reescribe.
/// Publicar mediante hard link evita reemplazar una base creada concurrentemente.
pub fn migrar_libro_anterior(
    anterior: &std::path::Path,
    nueva: &std::path::Path,
) -> Result<(), String> {
    if nueva.exists() || !anterior.exists() {
        return Ok(());
    }
    let directorio = nueva.parent().ok_or("Directorio de datos inválido")?;
    std::fs::create_dir_all(directorio).map_err(|e| e.to_string())?;
    let temporal = directorio.join(format!("migracion-{}.sqlite", uuid::Uuid::new_v4()));
    let resultado = (|| {
        let origen =
            Connection::open_with_flags(anterior, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|e| e.to_string())?;
        let mut destino = Connection::open(&temporal).map_err(|e| e.to_string())?;
        {
            let backup =
                rusqlite::backup::Backup::new(&origen, &mut destino).map_err(|e| e.to_string())?;
            if backup.step(-1).map_err(|e| e.to_string())? != rusqlite::backup::StepResult::Done {
                return Err(
                    "La base anterior está ocupada. Ciérrala y vuelve a abrir Cuentas Claras."
                        .into(),
                );
            }
        }
        let integridad: String = destino
            .query_row("PRAGMA quick_check", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if integridad != "ok" {
            return Err("La copia de SQLite no superó la verificación de integridad.".into());
        }
        drop(destino);
        let copia = RepositorioSqlite::inicializar(temporal.clone())?;
        crate::domain::ejecutar_listar_cuentas(&copia).map_err(|e| e.mensaje)?;
        drop(copia);
        std::fs::hard_link(&temporal, nueva)
            .map_err(|e| format!("No se pudo publicar la copia sin sobrescribir datos: {e}"))?;
        Ok(())
    })();
    let _ = std::fs::remove_file(&temporal);
    resultado
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::domain::ejecutar_crear_cuenta;

    #[test]
    fn probar_inmutabilidad_eventos() {
        // Creamos una BD de prueba
        let ruta = std::env::temp_dir().join(format!("prueba_{}.db", uuid::Uuid::new_v4()));

        let mut repo = RepositorioSqlite::inicializar(ruta.clone()).unwrap();

        // Creamos una cuenta y guardamos su evento
        ejecutar_crear_cuenta(&mut repo, "Cartera", "efectivo", "VES", "100.00", "prueba").unwrap();

        let conn = Connection::open(&ruta).unwrap();

        // Vemos si se puede cambiar el evento
        let cambiar = conn.execute(
            "UPDATE eventos_libro SET tipo = 'Otro' WHERE tipo = 'CuentaCreada'",
            [],
        );
        assert!(cambiar.is_err());

        // Vemos si se puede borrar el evento
        let borrar = conn.execute("DELETE FROM eventos_libro WHERE tipo = 'CuentaCreada'", []);
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
