//! sincronizacion_db.rs: acceso a PostgreSQL para eventos de sincronización y claves de idempotencia.
//! Usado para consultar lotes incrementales y evitar duplicados.

use chrono::NaiveDate;
use serde::Serialize;
use sqlx::PgPool;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub async fn guardar_eventos_gasto(
    tx: &mut Transaction<'_, Postgres>,
    gasto_id: Uuid,
) -> Result<(), sqlx::Error> {
    let usuarios: Vec<Uuid> = sqlx::query_scalar(
        "SELECT usuario_id FROM gasto_participantes
         WHERE gasto_id = $1 ORDER BY usuario_id",
    )
    .bind(gasto_id)
    .fetch_all(&mut **tx)
    .await?;

    for usuario_id in usuarios {
        // Evitamos las posiciones repetidas
        sqlx::query("SELECT id FROM usuarios WHERE id = $1 FOR NO KEY UPDATE")
            .bind(usuario_id)
            .fetch_one(&mut **tx)
            .await?;

        // Buscamos la siguiente posicion del usuario
        let posicion: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(posicion), 0) + 1
             FROM eventos_usuario WHERE usuario_id = $1",
        )
        .bind(usuario_id)
        .fetch_one(&mut **tx)
        .await?;

        // Guardamos la parte del gasto que le toca al usuario
        sqlx::query(
            "INSERT INTO eventos_usuario
             (id, usuario_id, posicion, tipo, gasto_id, grupo_id,
              concepto, fecha, moneda, monto_parte, recibio_residuo)
             SELECT $1, p.usuario_id, $2, 'gasto_grupo', g.id, g.grupo_id, g.concepto, (g.fecha_gasto AT TIME ZONE 'UTC')::date,
                    gr.moneda, p.monto_parte, p.recibio_residuo
             FROM gasto_participantes p
             JOIN gastos g ON g.id = p.gasto_id
             JOIN grupos gr ON gr.id = g.grupo_id
             WHERE p.gasto_id = $3 AND p.usuario_id = $4
             ON CONFLICT (usuario_id, gasto_id) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(posicion)
        .bind(gasto_id)
        .bind(usuario_id)
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

pub enum ResultadoClave {
    Nueva,
    YaExiste(Uuid),
    DatosDistintos,
}

pub async fn revisar_clave(
    tx: &mut Transaction<'_, Postgres>,
    usuario_id: Uuid,
    clave: &str,
    grupo_id: Uuid,
    datos: &str,
) -> Result<ResultadoClave, sqlx::Error> {
    // Primero intentamos guardar la clave
    let res = sqlx::query(
        "INSERT INTO claves_gastos (usuario_id, clave, grupo_id, datos)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (usuario_id, clave) DO NOTHING",
    )
    .bind(usuario_id)
    .bind(clave)
    .bind(grupo_id)
    .bind(datos)
    .execute(&mut **tx)
    .await?;

    if res.rows_affected() == 1 {
        return Ok(ResultadoClave::Nueva);
    }

    // Si la clave ya estaba, vemos que datos tenia
    let fila: (Uuid, String, Option<Uuid>) = sqlx::query_as(
        "SELECT grupo_id, datos, gasto_id FROM claves_gastos
         WHERE usuario_id = $1 AND clave = $2",
    )
    .bind(usuario_id)
    .bind(clave)
    .fetch_one(&mut **tx)
    .await?;

    if fila.0 != grupo_id || fila.1 != datos {
        return Ok(ResultadoClave::DatosDistintos);
    }

    if let Some(id) = fila.2 {
        return Ok(ResultadoClave::YaExiste(id));
    }

    Err(sqlx::Error::RowNotFound)
}

pub async fn guardar_clave(
    tx: &mut Transaction<'_, Postgres>,
    usuario_id: Uuid,
    clave: &str,
    gasto_id: Uuid,
) -> Result<(), sqlx::Error> {
    // Guardamos el id del gasto con su clave
    let res = sqlx::query(
        "UPDATE claves_gastos SET gasto_id = $3
         WHERE usuario_id = $1 AND clave = $2 AND gasto_id IS NULL",
    )
    .bind(usuario_id)
    .bind(clave)
    .bind(gasto_id)
    .execute(&mut **tx)
    .await?;

    if res.rows_affected() != 1 {
        return Err(sqlx::Error::RowNotFound);
    }
    Ok(())
}

// Fila de un evento compartido de gasto grupal almacenado por usuario
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventoUsuario {
    pub id: Uuid,
    pub posicion: i64,
    pub tipo: String,
    pub gasto_id: Uuid,
    pub grupo_id: Uuid,
    pub concepto: String,
    pub fecha: NaiveDate,
    pub moneda: String,
    pub monto_parte: String,
    pub recibio_residuo: bool,
}

// Lote paginado de eventos leídos de PostgreSQL con cursor e indicador de continuidad
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LoteCambios {
    pub eventos: Vec<EventoUsuario>,
    pub cursor: i64,
    pub hay_mas: bool,
}

pub async fn consultar_cambios(
    pool: &PgPool,
    usuario_id: Uuid,
    desde: i64,
) -> Result<LoteCambios, sqlx::Error> {
    // Buscamos los eventos nuevos de este usuario
    let mut filas: Vec<(
        Uuid,
        i64,
        String,
        Uuid,
        Uuid,
        String,
        NaiveDate,
        String,
        String,
        bool,
    )> = sqlx::query_as(
        "SELECT id, posicion, tipo, gasto_id, grupo_id, concepto,
                    fecha, moneda, monto_parte::text, recibio_residuo
             FROM eventos_usuario
             WHERE usuario_id = $1 AND posicion > $2
             ORDER BY posicion
             LIMIT 101",
    )
    .bind(usuario_id)
    .bind(desde)
    .fetch_all(pool)
    .await?;

    // Quitamos el evento extra si hay mas de 100
    let hay_mas = filas.len() > 100;
    if hay_mas {
        filas.pop();
    }

    let mut resp = Vec::new();
    let mut cursor = desde;

    // Guardamos los eventos y actualizamos el cursor
    for (
        id,
        posicion,
        tipo,
        gasto_id,
        grupo_id,
        concepto,
        fecha,
        moneda,
        monto_parte,
        recibio_residuo,
    ) in filas
    {
        let evento = EventoUsuario {
            id,
            posicion,
            tipo,
            gasto_id,
            grupo_id,
            concepto,
            fecha,
            moneda,
            monto_parte,
            recibio_residuo,
        };

        resp.push(evento);
        cursor = posicion;
    }

    Ok(LoteCambios {
        eventos: resp,
        cursor,
        hay_mas,
    })
}

// Límite máximo de eventos devueltos por lote de sincronización
#[allow(dead_code)]
pub const LIMITE_LOTE_CAMBIOS: usize = 100;

// Cantidad consultada en SQL para detectar si existen más páginas
#[allow(dead_code)]
const LIMITE_CONSULTA_CAMBIOS: i64 = 101;

// Fila devuelta por PostgreSQL al consultar eventos con grupo
type FilaCambioDb = (
    Uuid,
    i64,
    String,
    Uuid,
    Uuid,
    String,
    String,
    NaiveDate,
    String,
    String,
    bool,
);

// Evento compartido de un usuario acompañado del nombre de su grupo
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventoCambioDb {
    pub evento: EventoUsuario,
    pub grupo_nombre: String,
}

// Página de eventos ordenados de un usuario con cursor y continuidad
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PaginaCambiosDb {
    pub eventos: Vec<EventoCambioDb>,
    pub nuevo_cursor: i64,
    pub hay_mas: bool,
}

// Recorta hasta cien eventos, calcula el nuevo cursor e indica continuidad
#[allow(dead_code)]
pub fn armar_pagina_cambios(
    mut eventos: Vec<EventoCambioDb>,
    cursor_desde: i64,
) -> PaginaCambiosDb {
    let hay_mas = eventos.len() > LIMITE_LOTE_CAMBIOS;
    if hay_mas {
        // Recorta el elemento extra usado solo para detectar continuidad.
        eventos.truncate(LIMITE_LOTE_CAMBIOS);
    }

    // Conserva el cursor recibido cuando no existen eventos nuevos
    let nuevo_cursor = eventos
        .last()
        .map_or(cursor_desde, |item| item.evento.posicion);

    PaginaCambiosDb {
        eventos,
        nuevo_cursor,
        hay_mas,
    }
}

// Consulta en orden hasta cien eventos nuevos exclusivos del usuario autenticado
#[allow(dead_code)]
pub async fn consultar_cambios_paginados(
    pool: &PgPool,
    usuario_id: Uuid,
    desde: i64,
) -> Result<PaginaCambiosDb, sqlx::Error> {
    // Filtra por usuario y cursor para aislar eventos según INV-17
    let filas: Vec<FilaCambioDb> = sqlx::query_as(
        "SELECT e.id, e.posicion, e.tipo, e.gasto_id, e.grupo_id, gr.nombre,
                e.concepto, e.fecha, e.moneda, e.monto_parte::text, e.recibio_residuo
         FROM eventos_usuario e
         JOIN grupos gr ON gr.id = e.grupo_id
         WHERE e.usuario_id = $1 AND e.posicion > $2
         ORDER BY e.posicion ASC
         LIMIT $3",
    )
    .bind(usuario_id)
    .bind(desde)
    .bind(LIMITE_CONSULTA_CAMBIOS)
    .fetch_all(pool)
    .await?;

    let eventos = filas
        .into_iter()
        .map(
            |(
                id,
                posicion,
                tipo,
                gasto_id,
                grupo_id,
                grupo_nombre,
                concepto,
                fecha,
                moneda,
                monto_parte,
                recibio_residuo,
            )| EventoCambioDb {
                evento: EventoUsuario {
                    id,
                    posicion,
                    tipo,
                    gasto_id,
                    grupo_id,
                    concepto,
                    fecha,
                    moneda,
                    monto_parte,
                    recibio_residuo,
                },
                grupo_nombre,
            },
        )
        .collect();

    Ok(armar_pagina_cambios(eventos, desde))
}

// Pruebas unitarias de paginación, recorte y cálculo de cursor
#[cfg(test)]
mod tests {
    use super::*;

    // Crea un evento de prueba en la posición indicada
    fn evento_prueba(posicion: i64) -> EventoCambioDb {
        EventoCambioDb {
            evento: EventoUsuario {
                id: Uuid::new_v4(),
                posicion,
                tipo: String::from("gasto_grupo"),
                gasto_id: Uuid::new_v4(),
                grupo_id: Uuid::new_v4(),
                concepto: String::from("Hotel"),
                fecha: NaiveDate::from_ymd_opt(2026, 11, 22).expect("Fecha válida"),
                moneda: String::from("USD"),
                monto_parte: String::from("33.33"),
                recibio_residuo: false,
            },
            grupo_nombre: String::from("Viaje a Mérida"),
        }
    }

    // Verifica que una consulta vacía conserve el cursor recibido
    #[test]
    fn paginacion_vacia_conserva_cursor_recibido() {
        let pagina = armar_pagina_cambios(Vec::new(), 15);
        assert!(pagina.eventos.is_empty());
        assert_eq!(pagina.nuevo_cursor, 15);
        assert!(!pagina.hay_mas);
    }

    // Comprueba que cien eventos exactos no marquen páginas adicionales
    #[test]
    fn paginacion_con_cien_eventos_no_marca_hay_mas() {
        let eventos: Vec<EventoCambioDb> = (1..=100_i64).map(evento_prueba).collect();
        let pagina = armar_pagina_cambios(eventos, 0);

        assert_eq!(pagina.eventos.len(), 100);
        assert_eq!(pagina.nuevo_cursor, 100);
        assert!(!pagina.hay_mas);
    }

    // Comprueba recorte a cien eventos y actualización correcta del cursor
    #[test]
    fn paginacion_con_mas_de_cien_recorta_y_marca_hay_mas() {
        let eventos: Vec<EventoCambioDb> = (11..=111_i64).map(evento_prueba).collect();
        let pagina = armar_pagina_cambios(eventos, 10);

        assert_eq!(pagina.eventos.len(), 100);
        assert_eq!(pagina.eventos.first().unwrap().evento.posicion, 11);
        assert_eq!(pagina.eventos.last().unwrap().evento.posicion, 110);
        assert_eq!(pagina.nuevo_cursor, 110);
        assert!(pagina.hay_mas);
    }
}
