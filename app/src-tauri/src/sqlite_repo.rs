use crate::domain::{EventoLibro, RepositorioEventos};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
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
             END;",
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

pub mod entrega7_local {
    use super::*;
    use crate::domain::{
        DatosMovimientoRegistrado, Monto, TipoMovimiento, ejecutar_obtener_detalle_cuenta,
    };
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};
    use std::str::FromStr;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct ParteGastoGrupo {
        pub evento_servidor_id: String,
        pub cursor: i64,
        pub grupo_id: String,
        pub grupo_nombre: String,
        pub gasto_id: String,
        pub concepto: String,
        pub fecha: String, // YYYY-MM-DD
        pub moneda: String,
        pub monto_parte: String,
        pub incluye_redondeo: bool,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct ItemColaGastoGrupo {
        pub id: String,
        pub idempotency_key: String,
        pub grupo_id: String,
        pub payload_json: String,
        pub estado: String, // "pendiente" | "enviado" | "rechazado"
        pub motivo_rechazo: Option<String>,
        pub creado_en: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    pub struct ResultadoAplicacionLote {
        pub aplicados: usize,
        pub omitidos_duplicados: usize,
        pub pendientes_sin_cuenta: usize,
        pub nuevo_cursor: i64,
    }

    pub fn inicializar_tablas_entrega7(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS grupo_cuentas (
                grupo_id TEXT PRIMARY KEY,
                cuenta_id TEXT NOT NULL,
                moneda TEXT NOT NULL,
                actualizado_en TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS partes_pendientes (
                evento_servidor_id TEXT PRIMARY KEY,
                cursor INTEGER NOT NULL,
                grupo_id TEXT NOT NULL,
                grupo_nombre TEXT NOT NULL,
                gasto_id TEXT NOT NULL,
                concepto TEXT NOT NULL,
                fecha TEXT NOT NULL,
                moneda TEXT NOT NULL,
                monto_parte TEXT NOT NULL,
                incluye_redondeo INTEGER NOT NULL DEFAULT 0,
                recibido_en TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS estado_sincronizacion_grupos (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                ultimo_cursor INTEGER NOT NULL DEFAULT 0,
                ultima_sincronizacion TEXT
            );

            INSERT OR IGNORE INTO estado_sincronizacion_grupos (id, ultimo_cursor)
            VALUES (1, 0);

            CREATE TABLE IF NOT EXISTS eventos_grupo_aplicados (
                evento_servidor_id TEXT PRIMARY KEY,
                movimiento_id TEXT NOT NULL,
                cuenta_id TEXT NOT NULL,
                aplicado_en TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS cola_gastos_grupo (
                id TEXT PRIMARY KEY,
                idempotency_key TEXT NOT NULL UNIQUE,
                grupo_id TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                estado TEXT NOT NULL DEFAULT 'pendiente',
                motivo_rechazo TEXT,
                creado_en TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_partes_pendientes_grupo
                ON partes_pendientes(grupo_id, cursor ASC);

            CREATE INDEX IF NOT EXISTS idx_cola_gastos_estado_orden
                ON cola_gastos_grupo(estado, creado_en ASC);
            ",
        )
        .map_err(|e| e.to_string())
    }

    fn construir_evento_gasto_grupo(
        parte: &ParteGastoGrupo,
        cuenta_id: &str,
        dispositivo_id: &str,
    ) -> Result<(EventoLibro, String), String> {
        let cantidad = Decimal::from_str(parte.monto_parte.trim())
            .map_err(|_| format!("Monto de parte inválido: {}", parte.monto_parte))?;
        let monto = Monto::desde_str_con_moneda(
            &cantidad.to_string(),
            &crate::domain::buscar_moneda_en(&parte.moneda, &crate::domain::catalogo_monedas())
                .map_err(|e| e.mensaje)?,
            "monto",
            false,
        )
        .map_err(|e| e.mensaje)?;

        let movimiento_id = uuid::Uuid::new_v4().to_string();
        let concepto_limpio = format!("{} {}", parte.grupo_nombre.trim(), parte.concepto.trim())
            .trim()
            .to_string();
        let nota = format!("[gasto-grupo] {}", concepto_limpio);

        let datos = DatosMovimientoRegistrado {
            movimiento_id: movimiento_id.clone(),
            cuenta_id: cuenta_id.to_string(),
            tipo: TipoMovimiento::Gasto,
            monto,
            fecha: parte.fecha.clone(),
            categoria: "Otros gastos".to_string(),
            nota: Some(nota),
        };

        let evento = EventoLibro {
            id: uuid::Uuid::new_v4().to_string(),
            tipo: "MovimientoRegistrado".to_string(),
            momento_registro: Utc::now(),
            dispositivo_origen: dispositivo_id.to_string(),
            datos_json: serde_json::to_string(&datos).map_err(|e| e.to_string())?,
        };

        Ok((evento, movimiento_id))
    }

    fn aplicar_parte_en_tx(
        tx: &rusqlite::Transaction<'_>,
        parte: &ParteGastoGrupo,
        cuenta_id: &str,
        dispositivo_id: &str,
    ) -> Result<bool, String> {
        let ya_aplicado: i64 = tx
            .query_row(
                "SELECT COUNT(1) FROM eventos_grupo_aplicados WHERE evento_servidor_id = ?1",
                params![parte.evento_servidor_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;

        if ya_aplicado > 0 {
            return Ok(false);
        }

        let (evento, movimiento_id) = construir_evento_gasto_grupo(parte, cuenta_id, dispositivo_id)?;

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
            "INSERT INTO eventos_grupo_aplicados (evento_servidor_id, movimiento_id, cuenta_id, aplicado_en)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                parte.evento_servidor_id,
                movimiento_id,
                cuenta_id,
                Utc::now().to_rfc3339()
            ],
        )
        .map_err(|e| e.to_string())?;

        tx.execute(
            "DELETE FROM partes_pendientes WHERE evento_servidor_id = ?1",
            params![parte.evento_servidor_id],
        )
        .map_err(|e| e.to_string())?;

        Ok(true)
    }

    impl RepositorioSqlite {
        pub fn asegurar_tablas_entrega7(&self) -> Result<(), String> {
            let conn = self.abrir()?;
            inicializar_tablas_entrega7(&conn)
        }

        pub fn obtener_cursor_grupo(&self) -> Result<i64, String> {
            self.asegurar_tablas_entrega7()?;
            let conn = self.abrir()?;
            conn.query_row(
                "SELECT ultimo_cursor FROM estado_sincronizacion_grupos WHERE id = 1",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())
        }

        pub fn establecer_cursor_grupo(&self, nuevo_cursor: i64) -> Result<(), String> {
            self.asegurar_tablas_entrega7()?;
            let conn = self.abrir()?;
            conn.execute(
                "UPDATE estado_sincronizacion_grupos
                 SET ultimo_cursor = ?1, ultima_sincronizacion = ?2
                 WHERE id = 1",
                params![nuevo_cursor, Utc::now().to_rfc3339()],
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        }

        pub fn asignar_cuenta_grupo(
            &mut self,
            grupo_id: &str,
            cuenta_id: &str,
            moneda: &str,
            dispositivo_id: &str,
        ) -> Result<usize, String> {
            self.asegurar_tablas_entrega7()?;
            let mut conn = self.abrir()?;
            let tx = conn.transaction().map_err(|e| e.to_string())?;

            tx.execute(
                "INSERT INTO grupo_cuentas (grupo_id, cuenta_id, moneda, actualizado_en)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(grupo_id) DO UPDATE SET
                     cuenta_id = excluded.cuenta_id,
                     moneda = excluded.moneda,
                     actualizado_en = excluded.actualizado_en",
                params![grupo_id, cuenta_id, moneda, Utc::now().to_rfc3339()],
            )
            .map_err(|e| e.to_string())?;

            let mut pendientes = Vec::new();
            {
                let mut stmt = tx
                    .prepare(
                        "SELECT evento_servidor_id, cursor, grupo_id, grupo_nombre, gasto_id,
                                concepto, fecha, moneda, monto_parte, incluye_redondeo
                         FROM partes_pendientes
                         WHERE grupo_id = ?1 AND moneda = ?2
                         ORDER BY cursor ASC, rowid ASC",
                    )
                    .map_err(|e| e.to_string())?;

                let filas = stmt
                    .query_map(params![grupo_id, moneda], |row| {
                        let redondeo_int: i64 = row.get(9)?;
                        Ok(ParteGastoGrupo {
                            evento_servidor_id: row.get(0)?,
                            cursor: row.get(1)?,
                            grupo_id: row.get(2)?,
                            grupo_nombre: row.get(3)?,
                            gasto_id: row.get(4)?,
                            concepto: row.get(5)?,
                            fecha: row.get(6)?,
                            moneda: row.get(7)?,
                            monto_parte: row.get(8)?,
                            incluye_redondeo: redondeo_int != 0,
                        })
                    })
                    .map_err(|e| e.to_string())?;

                for fila in filas {
                    pendientes.push(fila.map_err(|e| e.to_string())?);
                }
            }

            let mut aplicados = 0usize;
            for parte in pendientes {
                if aplicar_parte_en_tx(&tx, &parte, cuenta_id, dispositivo_id)? {
                    aplicados += 1;
                }
            }

            tx.commit().map_err(|e| e.to_string())?;
            Ok(aplicados)
        }

        pub fn obtener_cuenta_grupo(&self, grupo_id: &str) -> Result<Option<(String, String)>, String> {
            self.asegurar_tablas_entrega7()?;
            let conn = self.abrir()?;
            let mut stmt = conn
                .prepare("SELECT cuenta_id, moneda FROM grupo_cuentas WHERE grupo_id = ?1")
                .map_err(|e| e.to_string())?;
            let mut rows = stmt
                .query_map(params![grupo_id], |r| Ok((r.get(0)?, r.get(1)?)))
                .map_err(|e| e.to_string())?;
            match rows.next() {
                Some(item) => item.map(Some).map_err(|e| e.to_string()),
                None => Ok(None),
            }
        }

        pub fn listar_partes_pendientes(&self) -> Result<Vec<ParteGastoGrupo>, String> {
            self.asegurar_tablas_entrega7()?;
            let conn = self.abrir()?;
            let mut stmt = conn
                .prepare(
                    "SELECT evento_servidor_id, cursor, grupo_id, grupo_nombre, gasto_id,
                            concepto, fecha, moneda, monto_parte, incluye_redondeo
                     FROM partes_pendientes
                     ORDER BY cursor ASC, rowid ASC",
                )
                .map_err(|e| e.to_string())?;

            let filas = stmt
                .query_map([], |row| {
                    let redondeo_int: i64 = row.get(9)?;
                    Ok(ParteGastoGrupo {
                        evento_servidor_id: row.get(0)?,
                        cursor: row.get(1)?,
                        grupo_id: row.get(2)?,
                        grupo_nombre: row.get(3)?,
                        gasto_id: row.get(4)?,
                        concepto: row.get(5)?,
                        fecha: row.get(6)?,
                        moneda: row.get(7)?,
                        monto_parte: row.get(8)?,
                        incluye_redondeo: redondeo_int != 0,
                    })
                })
                .map_err(|e| e.to_string())?;

            let mut out = Vec::new();
            for f in filas {
                out.push(f.map_err(|e| e.to_string())?);
            }
            Ok(out)
        }

        pub fn aplicar_lote_eventos_grupo(
            &mut self,
            partes: &[ParteGastoGrupo],
            nuevo_cursor: i64,
            dispositivo_id: &str,
        ) -> Result<ResultadoAplicacionLote, String> {
            self.asegurar_tablas_entrega7()?;
            let mut conn = self.abrir()?;
            let tx = conn.transaction().map_err(|e| e.to_string())?;

            let mut aplicados = 0usize;
            let mut omitidos_duplicados = 0usize;
            let mut pendientes_sin_cuenta = 0usize;

            for parte in partes {
                let ya_aplicado: i64 = tx
                    .query_row(
                        "SELECT COUNT(1) FROM eventos_grupo_aplicados WHERE evento_servidor_id = ?1",
                        params![parte.evento_servidor_id],
                        |r| r.get(0),
                    )
                    .map_err(|e| e.to_string())?;

                if ya_aplicado > 0 {
                    omitidos_duplicados += 1;
                    continue;
                }

                let cuenta_opt: Option<(String, String)> = {
                    let mut stmt = tx
                        .prepare("SELECT cuenta_id, moneda FROM grupo_cuentas WHERE grupo_id = ?1")
                        .map_err(|e| e.to_string())?;
                    let mut rows = stmt
                        .query_map(params![parte.grupo_id], |r| Ok((r.get(0)?, r.get(1)?)))
                        .map_err(|e| e.to_string())?;
                    match rows.next() {
                        Some(item) => Some(item.map_err(|e| e.to_string())?),
                        None => None,
                    }
                };

                match cuenta_opt {
                    Some((cuenta_id, moneda_cuenta)) if moneda_cuenta == parte.moneda => {
                        if aplicar_parte_en_tx(&tx, parte, &cuenta_id, dispositivo_id)? {
                            aplicados += 1;
                        } else {
                            omitidos_duplicados += 1;
                        }
                    }
                    _ => {
                        tx.execute(
                            "INSERT OR IGNORE INTO partes_pendientes
                             (evento_servidor_id, cursor, grupo_id, grupo_nombre, gasto_id,
                              concepto, fecha, moneda, monto_parte, incluye_redondeo, recibido_en)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                            params![
                                parte.evento_servidor_id,
                                parte.cursor,
                                parte.grupo_id,
                                parte.grupo_nombre,
                                parte.gasto_id,
                                parte.concepto,
                                parte.fecha,
                                parte.moneda,
                                parte.monto_parte,
                                if parte.incluye_redondeo { 1 } else { 0 },
                                Utc::now().to_rfc3339()
                            ],
                        )
                        .map_err(|e| e.to_string())?;
                        pendientes_sin_cuenta += 1;
                    }
                }
            }

            tx.execute(
                "UPDATE estado_sincronizacion_grupos
                 SET ultimo_cursor = ?1, ultima_sincronizacion = ?2
                 WHERE id = 1",
                params![nuevo_cursor, Utc::now().to_rfc3339()],
            )
            .map_err(|e| e.to_string())?;

            tx.commit().map_err(|e| e.to_string())?;

            Ok(ResultadoAplicacionLote {
                aplicados,
                omitidos_duplicados,
                pendientes_sin_cuenta,
                nuevo_cursor,
            })
        }

        pub fn encolar_gasto_grupo_offline(
            &self,
            idempotency_key: &str,
            grupo_id: &str,
            payload_json: &str,
        ) -> Result<ItemColaGastoGrupo, String> {
            self.asegurar_tablas_entrega7()?;
            let conn = self.abrir()?;

            let existente: Option<ItemColaGastoGrupo> = {
                let mut stmt = conn
                    .prepare(
                        "SELECT id, idempotency_key, grupo_id, payload_json, estado, motivo_rechazo, creado_en
                         FROM cola_gastos_grupo WHERE idempotency_key = ?1",
                    )
                    .map_err(|e| e.to_string())?;
                let mut rows = stmt
                    .query_map(params![idempotency_key], |r| {
                        Ok(ItemColaGastoGrupo {
                            id: r.get(0)?,
                            idempotency_key: r.get(1)?,
                            grupo_id: r.get(2)?,
                            payload_json: r.get(3)?,
                            estado: r.get(4)?,
                            motivo_rechazo: r.get(5)?,
                            creado_en: r.get(6)?,
                        })
                    })
                    .map_err(|e| e.to_string())?;
                match rows.next() {
                    Some(item) => Some(item.map_err(|e| e.to_string())?),
                    None => None,
                }
            };

            if let Some(item) = existente {
                return Ok(item);
            }

            let item = ItemColaGastoGrupo {
                id: uuid::Uuid::new_v4().to_string(),
                idempotency_key: idempotency_key.to_string(),
                grupo_id: grupo_id.to_string(),
                payload_json: payload_json.to_string(),
                estado: "pendiente".to_string(),
                motivo_rechazo: None,
                creado_en: Utc::now().to_rfc3339(),
            };

            conn.execute(
                "INSERT INTO cola_gastos_grupo
                 (id, idempotency_key, grupo_id, payload_json, estado, motivo_rechazo, creado_en)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    item.id,
                    item.idempotency_key,
                    item.grupo_id,
                    item.payload_json,
                    item.estado,
                    item.motivo_rechazo,
                    item.creado_en
                ],
            )
            .map_err(|e| e.to_string())?;

            Ok(item)
        }

        pub fn listar_gastos_pendientes_cola(&self) -> Result<Vec<ItemColaGastoGrupo>, String> {
            self.asegurar_tablas_entrega7()?;
            let conn = self.abrir()?;
            let mut stmt = conn
                .prepare(
                    "SELECT id, idempotency_key, grupo_id, payload_json, estado, motivo_rechazo, creado_en
                     FROM cola_gastos_grupo
                     WHERE estado = 'pendiente'
                     ORDER BY creado_en ASC, rowid ASC",
                )
                .map_err(|e| e.to_string())?;

            let filas = stmt
                .query_map([], |r| {
                    Ok(ItemColaGastoGrupo {
                        id: r.get(0)?,
                        idempotency_key: r.get(1)?,
                        grupo_id: r.get(2)?,
                        payload_json: r.get(3)?,
                        estado: r.get(4)?,
                        motivo_rechazo: r.get(5)?,
                        creado_en: r.get(6)?,
                    })
                })
                .map_err(|e| e.to_string())?;

            let mut out = Vec::new();
            for f in filas {
                out.push(f.map_err(|e| e.to_string())?);
            }
            Ok(out)
        }

        pub fn marcar_gasto_cola_enviado(&self, idempotency_key: &str) -> Result<(), String> {
            self.asegurar_tablas_entrega7()?;
            let conn = self.abrir()?;
            conn.execute(
                "UPDATE cola_gastos_grupo
                 SET estado = 'enviado', motivo_rechazo = NULL
                 WHERE idempotency_key = ?1",
                params![idempotency_key],
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        }

        pub fn marcar_gasto_cola_rechazado(
            &self,
            idempotency_key: &str,
            motivo: &str,
        ) -> Result<(), String> {
            self.asegurar_tablas_entrega7()?;
            let conn = self.abrir()?;
            conn.execute(
                "UPDATE cola_gastos_grupo
                 SET estado = 'rechazado', motivo_rechazo = ?2
                 WHERE idempotency_key = ?1",
                params![idempotency_key, motivo],
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        }

        pub fn obtener_detalle_cuenta_entrega7(
            &self,
            cuenta_id: &str,
        ) -> Result<crate::domain::DetalleCuenta, String> {
            ejecutar_obtener_detalle_cuenta(self, cuenta_id).map_err(|e| e.mensaje)
        }
    }

    #[cfg(test)]
    mod tests_entrega7 {
        use super::*;
        use crate::domain::ejecutar_crear_cuenta;

        fn crear_repo_temporal() -> (RepositorioSqlite, PathBuf) {
            let ruta =
                std::env::temp_dir().join(format!("e7_prueba_{}.db", uuid::Uuid::new_v4()));
            let repo = RepositorioSqlite::inicializar(ruta.clone()).unwrap();
            (repo, ruta)
        }

        #[test]
        fn sincronizacion_idempotente_y_reinicio_cursor_no_duplica() {
            let (mut repo, ruta) = crear_repo_temporal();
            let cuenta =
                ejecutar_crear_cuenta(&mut repo, "Efectivo USD", "efectivo", "USD", "200.00", "t")
                    .unwrap();

            repo.asignar_cuenta_grupo("grp-1", cuenta.id(), "USD", "t")
                .unwrap();

            let lote = vec![ParteGastoGrupo {
                evento_servidor_id: "ev-1".to_string(),
                cursor: 1,
                grupo_id: "grp-1".to_string(),
                grupo_nombre: "Viaje a Mérida".to_string(),
                gasto_id: "gst-1".to_string(),
                concepto: "Hotel".to_string(),
                fecha: "2026-11-22".to_string(),
                moneda: "USD".to_string(),
                monto_parte: "33.33".to_string(),
                incluye_redondeo: true,
            }];

            let r1 = repo.aplicar_lote_eventos_grupo(&lote, 1, "t").unwrap();
            assert_eq!(r1.aplicados, 1);

            let r2 = repo.aplicar_lote_eventos_grupo(&lote, 1, "t").unwrap();
            assert_eq!(r2.aplicados, 0);
            assert_eq!(r2.omitidos_duplicados, 1);

            // Reiniciar cursor a 0 y aplicar de nuevo
            repo.establecer_cursor_grupo(0).unwrap();
            let r3 = repo.aplicar_lote_eventos_grupo(&lote, 1, "t").unwrap();
            assert_eq!(r3.aplicados, 0);
            assert_eq!(r3.omitidos_duplicados, 1);

            let detalle = repo.obtener_detalle_cuenta_entrega7(cuenta.id()).unwrap();
            assert_eq!(detalle.saldo_actual, "166.67");
            assert_eq!(detalle.movimientos.len(), 1);

            drop(repo);
            let _ = std::fs::remove_file(ruta);
        }

        #[test]
        fn partes_sin_cuenta_quedan_pendientes_y_se_aplican_al_elegir_cuenta() {
            let (mut repo, ruta) = crear_repo_temporal();
            let lote = vec![ParteGastoGrupo {
                evento_servidor_id: "ev-pend-1".to_string(),
                cursor: 5,
                grupo_id: "grp-2".to_string(),
                grupo_nombre: "Cena".to_string(),
                gasto_id: "gst-9".to_string(),
                concepto: "Pizza".to_string(),
                fecha: "2026-11-22".to_string(),
                moneda: "USD".to_string(),
                monto_parte: "15.00".to_string(),
                incluye_redondeo: false,
            }];

            let res = repo.aplicar_lote_eventos_grupo(&lote, 5, "t").unwrap();
            assert_eq!(res.pendientes_sin_cuenta, 1);
            assert_eq!(repo.listar_partes_pendientes().unwrap().len(), 1);

            let cuenta =
                ejecutar_crear_cuenta(&mut repo, "Banco USD", "banco", "USD", "100.00", "t")
                    .unwrap();
            let aplicados = repo
                .asignar_cuenta_grupo("grp-2", cuenta.id(), "USD", "t")
                .unwrap();
            assert_eq!(aplicados, 1);
            assert_eq!(repo.listar_partes_pendientes().unwrap().len(), 0);

            let detalle = repo.obtener_detalle_cuenta_entrega7(cuenta.id()).unwrap();
            assert_eq!(detalle.saldo_actual, "85.00");

            drop(repo);
            let _ = std::fs::remove_file(ruta);
        }

        #[test]
        fn cola_offline_mantiene_clave_idempotencia_y_no_bloquea_por_rechazo() {
            let (repo, ruta) = crear_repo_temporal();
            let item1 = repo
                .encolar_gasto_grupo_offline("idem-1", "grp-1", r#"{"monto":"60.00"}"#)
                .unwrap();
            let item1_dup = repo
                .encolar_gasto_grupo_offline("idem-1", "grp-1", r#"{"monto":"60.00"}"#)
                .unwrap();
            assert_eq!(item1.id, item1_dup.id);

            let _item2 = repo
                .encolar_gasto_grupo_offline("idem-2", "grp-1", r#"{"monto":"20.00"}"#)
                .unwrap();

            repo.marcar_gasto_cola_rechazado("idem-1", "Porcentajes no cuadran")
                .unwrap();
            let pendientes = repo.listar_gastos_pendientes_cola().unwrap();
            assert_eq!(pendientes.len(), 1);
            assert_eq!(pendientes[0].idempotency_key, "idem-2");

            repo.marcar_gasto_cola_enviado("idem-2").unwrap();
            assert!(repo.listar_gastos_pendientes_cola().unwrap().is_empty());

            drop(repo);
            let _ = std::fs::remove_file(ruta);
        }
    }
}
