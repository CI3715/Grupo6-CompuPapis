use sqlx::{Postgres, Transaction};
use chrono::NaiveDate;
use serde::Serialize;
use sqlx::PgPool;
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
    .bind(usuario_id).bind(clave).bind(grupo_id).bind(datos)
    .execute(&mut **tx).await?;

    if res.rows_affected() == 1 {
        return Ok(ResultadoClave::Nueva);
    }

    // Si la clave ya estaba, vemos que datos tenia
    let fila: (Uuid, String, Option<Uuid>) = sqlx::query_as(
        "SELECT grupo_id, datos, gasto_id FROM claves_gastos
         WHERE usuario_id = $1 AND clave = $2",
    )
    .bind(usuario_id).bind(clave)
    .fetch_one(&mut **tx).await?;

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
    .bind(usuario_id).bind(clave).bind(gasto_id)
    .execute(&mut **tx).await?;

    if res.rows_affected() != 1 {
        return Err(sqlx::Error::RowNotFound);
    }
    Ok(())
}

#[derive(Serialize)]
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

#[derive(Serialize)]
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
    let mut filas: Vec<(Uuid, i64, String, Uuid, Uuid, String, NaiveDate, String, String, bool)> =
        sqlx::query_as(
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
    for (id, posicion, tipo, gasto_id, grupo_id, concepto,
         fecha, moneda, monto_parte, recibio_residuo) in filas {
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
