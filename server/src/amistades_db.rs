use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub struct SolicitudPendiente {
    pub id: Uuid,
    pub emisor_id: Uuid,
    pub destinatario_id: Uuid,
    pub creada_en: DateTime<Utc>,
    pub nombre_usuario: String,
    pub nombre_visible: String,
}

pub struct Amigo {
    pub amistad_id: Uuid,
    pub id: Uuid,
    pub nombre_usuario: String,
    pub nombre_visible: String,
}

fn leer_solicitudes(filas: Vec<sqlx::postgres::PgRow>) -> Vec<SolicitudPendiente> {
    let mut solicitudes = Vec::new();

    // Guardamos las solicitudes en una lista
    for fila in filas {
        solicitudes.push(SolicitudPendiente {
            id: fila.get("id"),
            emisor_id: fila.get("emisor_id"),
            destinatario_id: fila.get("destinatario_id"),
            creada_en: fila.get("creada_en"),
            nombre_usuario: fila.get("nombre_usuario"),
            nombre_visible: fila.get("nombre_visible"),
        });
    }

    solicitudes
}

pub async fn son_amigos(
    pool: &PgPool,
    usuario1: Uuid,
    usuario2: Uuid,
) -> Result<bool, sqlx::Error> {
    if usuario1 == usuario2 {
        return Ok(false);
    }

    let datos = sqlx::query(
        "SELECT id FROM amistades
         WHERE (usuario1_id = $1 AND usuario2_id = $2)
         OR (usuario1_id = $2 AND usuario2_id = $1)",
    )
    .bind(usuario1)
    .bind(usuario2)
    .fetch_optional(pool)
    .await?;

    Ok(datos.is_some())
}

async fn bloquear_par(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    a: Uuid,
    b: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(LEAST($1::uuid,$2::uuid)::text || ':' || GREATEST($1::uuid,$2::uuid)::text, 0))")
        .bind(a).bind(b).execute(&mut **tx).await?;
    Ok(())
}

pub async fn guardar_solicitud(
    pool: &PgPool,
    id: Uuid,
    emisor: Uuid,
    destinatario: Uuid,
    fecha: DateTime<Utc>,
) -> Result<bool, sqlx::Error> {
    if emisor == destinatario {
        return Ok(false);
    }

    let mut tx = pool.begin().await?;
    bloquear_par(&mut tx, emisor, destinatario).await?;
    // Serializa este par con la aceptación para evitar amistad y solicitud pendiente simultáneas.
    let resultado = sqlx::query(
        "INSERT INTO solicitudes_amistad
         (id, emisor_id, destinatario_id, estado, creada_en)
         SELECT $1::uuid, $2::uuid, $3::uuid, 'pendiente', $4::timestamptz
         WHERE NOT EXISTS (
             SELECT 1 FROM amistades
             WHERE (usuario1_id = $2::uuid AND usuario2_id = $3::uuid)
             OR (usuario1_id = $3::uuid AND usuario2_id = $2::uuid)
         )
         ON CONFLICT DO NOTHING",
    )
    .bind(id)
    .bind(emisor)
    .bind(destinatario)
    .bind(fecha)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(resultado.rows_affected() == 1)
}

pub async fn solicitudes_enviadas(
    pool: &PgPool,
    usuario: Uuid,
) -> Result<Vec<SolicitudPendiente>, sqlx::Error> {
    let filas = sqlx::query(
        "SELECT s.id, s.emisor_id, s.destinatario_id, s.creada_en, u.nombre_usuario, u.nombre_visible
         FROM solicitudes_amistad s JOIN usuarios u ON u.id=s.destinatario_id
         WHERE s.emisor_id = $1 AND s.estado = 'pendiente'
         ORDER BY s.creada_en, s.id",
    )
    .bind(usuario)
    .fetch_all(pool)
    .await?;

    Ok(leer_solicitudes(filas))
}

pub async fn solicitudes_recibidas(
    pool: &PgPool,
    usuario: Uuid,
) -> Result<Vec<SolicitudPendiente>, sqlx::Error> {
    let filas = sqlx::query(
        "SELECT s.id, s.emisor_id, s.destinatario_id, s.creada_en, u.nombre_usuario, u.nombre_visible
         FROM solicitudes_amistad s JOIN usuarios u ON u.id=s.emisor_id
         WHERE s.destinatario_id = $1 AND s.estado = 'pendiente'
         ORDER BY s.creada_en, s.id",
    )
    .bind(usuario)
    .fetch_all(pool)
    .await?;

    Ok(leer_solicitudes(filas))
}

pub async fn aceptar_solicitud(
    pool: &PgPool,
    solicitud: Uuid,
    destinatario: Uuid,
    amistad_id: Uuid,
    fecha: DateTime<Utc>,
) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let emisor: Option<Uuid> = sqlx::query_scalar(
        "SELECT emisor_id FROM solicitudes_amistad WHERE id=$1 AND destinatario_id=$2",
    )
    .bind(solicitud)
    .bind(destinatario)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(emisor) = emisor else {
        return Ok(false);
    };
    bloquear_par(&mut tx, emisor, destinatario).await?;

    let fila = sqlx::query(
        "UPDATE solicitudes_amistad
         SET estado = 'aceptada', respondida_en = $3
         WHERE id = $1 AND destinatario_id = $2 AND estado = 'pendiente'
         RETURNING emisor_id",
    )
    .bind(solicitud)
    .bind(destinatario)
    .bind(fecha)
    .fetch_optional(&mut *tx)
    .await?;

    let emisor;

    if let Some(datos) = fila {
        emisor = datos.get("emisor_id");
    } else {
        return Ok(false);
    }

    // Guardamos los usuarios en el mismo orden
    let mut usuario1 = emisor;
    let mut usuario2 = destinatario;

    if usuario1 > usuario2 {
        std::mem::swap(&mut usuario1, &mut usuario2);
    }

    sqlx::query(
        "INSERT INTO amistades
         (id, usuario1_id, usuario2_id, creada_en)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(amistad_id)
    .bind(usuario1)
    .bind(usuario2)
    .bind(fecha)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(true)
}

pub async fn rechazar_solicitud(
    pool: &PgPool,
    solicitud: Uuid,
    destinatario: Uuid,
    fecha: DateTime<Utc>,
) -> Result<bool, sqlx::Error> {
    let resultado = sqlx::query(
        "UPDATE solicitudes_amistad
         SET estado = 'rechazada', respondida_en = $3
         WHERE id = $1 AND destinatario_id = $2 AND estado = 'pendiente'",
    )
    .bind(solicitud)
    .bind(destinatario)
    .bind(fecha)
    .execute(pool)
    .await?;

    Ok(resultado.rows_affected() == 1)
}

pub async fn listar_amigos(pool: &PgPool, usuario: Uuid) -> Result<Vec<Amigo>, sqlx::Error> {
    let filas = sqlx::query(
        "SELECT a.id AS amistad_id, u.id, u.nombre_usuario, u.nombre_visible
         FROM amistades a
         JOIN usuarios u ON u.id = a.usuario1_id OR u.id = a.usuario2_id
         WHERE (a.usuario1_id = $1 OR a.usuario2_id = $1)
         AND u.id <> $1
         ORDER BY u.nombre_usuario",
    )
    .bind(usuario)
    .fetch_all(pool)
    .await?;

    let mut amigos = Vec::new();

    // Guardamos los datos de cada amigo
    for fila in filas {
        amigos.push(Amigo {
            amistad_id: fila.get("amistad_id"),
            id: fila.get("id"),
            nombre_usuario: fila.get("nombre_usuario"),
            nombre_visible: fila.get("nombre_visible"),
        });
    }

    Ok(amigos)
}

pub async fn eliminar_amistad(
    pool: &PgPool,
    amistad: Uuid,
    usuario: Uuid,
) -> Result<bool, sqlx::Error> {
    let resultado = sqlx::query(
        "DELETE FROM amistades
         WHERE id = $1 AND (usuario1_id = $2 OR usuario2_id = $2)",
    )
    .bind(amistad)
    .bind(usuario)
    .execute(pool)
    .await?;

    Ok(resultado.rows_affected() == 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requiere PostgreSQL de pruebas"]
    async fn guardar_y_revisar_amistades() -> Result<(), sqlx::Error> {
        let url = std::env::var("E5_TEST_DATABASE_URL").expect("Falta E5_TEST_DATABASE_URL");
        let pool = PgPool::connect(&url).await?;

        let base: String = sqlx::query_scalar("SELECT current_database()")
            .fetch_one(&pool)
            .await?;
        assert_eq!(base, "cuentas_claras_pruebas");

        sqlx::raw_sql(include_str!("../migrations/001_usuarios.sql"))
            .execute(&pool)
            .await?;
        sqlx::raw_sql(include_str!("../migrations/010_amistades.sql"))
            .execute(&pool)
            .await?;

        let ana = Uuid::new_v4();
        let beto = Uuid::new_v4();
        let carla = Uuid::new_v4();
        let fecha = Utc::now();

        // Creamos 3 usuarios
        for (id, nombre) in [(ana, "ana"), (beto, "beto"), (carla, "carla")] {
            let usuario = format!("{}_{}", nombre, id.simple());
            let correo = format!("{}@example.com", usuario);

            sqlx::query(
                "INSERT INTO usuarios
                 (id, nombre_usuario, nombre_visible, correo, password_hash)
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(id)
            .bind(usuario)
            .bind(nombre)
            .bind(correo)
            .bind("hash_prueba")
            .execute(&pool)
            .await?;
        }

        // Ana le manda una solicitud a Beto
        let solicitud1 = Uuid::new_v4();
        assert!(guardar_solicitud(&pool, solicitud1, ana, beto, fecha).await?);

        // Revisamos que no se repita
        assert!(!guardar_solicitud(&pool, Uuid::new_v4(), beto, ana, fecha).await?);
        assert!(!guardar_solicitud(&pool, Uuid::new_v4(), ana, ana, fecha).await?);

        let enviadas = solicitudes_enviadas(&pool, ana).await?;
        let recibidas = solicitudes_recibidas(&pool, beto).await?;

        assert_eq!(enviadas.len(), 1);
        assert_eq!(recibidas.len(), 1);
        assert_eq!(enviadas[0].id, solicitud1);
        assert_eq!(enviadas[0].destinatario_id, beto);
        assert_eq!(recibidas[0].emisor_id, ana);

        // Beto acepta la solicitud
        let amistad = Uuid::new_v4();

        assert!(!aceptar_solicitud(&pool, solicitud1, ana, amistad, fecha).await?);
        let (aceptada, duplicada) = tokio::join!(
            aceptar_solicitud(&pool, solicitud1, beto, amistad, fecha),
            guardar_solicitud(&pool, Uuid::new_v4(), beto, ana, fecha)
        );
        assert!(aceptada?);
        assert!(!duplicada?);
        assert!(!aceptar_solicitud(&pool, solicitud1, beto, Uuid::new_v4(), fecha).await?);

        assert!(son_amigos(&pool, ana, beto).await?);
        assert!(!guardar_solicitud(&pool, Uuid::new_v4(), ana, beto, fecha).await?);

        let enviadas = solicitudes_enviadas(&pool, ana).await?;
        assert!(enviadas.is_empty());

        let amigos = listar_amigos(&pool, ana).await?;
        assert_eq!(amigos.len(), 1);
        assert_eq!(amigos[0].id, beto);
        assert_eq!(amigos[0].amistad_id, amistad);

        // Carla rechaza la solicitud
        let solicitud2 = Uuid::new_v4();
        assert!(guardar_solicitud(&pool, solicitud2, beto, carla, fecha).await?);

        assert!(!rechazar_solicitud(&pool, solicitud2, beto, fecha).await?);
        assert!(rechazar_solicitud(&pool, solicitud2, carla, fecha).await?);
        assert!(!rechazar_solicitud(&pool, solicitud2, carla, fecha).await?);

        assert!(!son_amigos(&pool, beto, carla).await?);

        let recibidas = solicitudes_recibidas(&pool, carla).await?;
        assert!(recibidas.is_empty());

        // Chequeamos que los datos sigan guardados
        pool.close().await;
        let pool = PgPool::connect(&url).await?;

        assert!(son_amigos(&pool, ana, beto).await?);

        let estado: String =
            sqlx::query_scalar("SELECT estado FROM solicitudes_amistad WHERE id = $1")
                .bind(solicitud2)
                .fetch_one(&pool)
                .await?;

        assert_eq!(estado, "rechazada");

        // Eliminamos la amistad
        assert!(!eliminar_amistad(&pool, amistad, carla).await?);
        assert!(eliminar_amistad(&pool, amistad, ana).await?);
        assert!(!son_amigos(&pool, ana, beto).await?);

        let estado: String =
            sqlx::query_scalar("SELECT estado FROM solicitudes_amistad WHERE id = $1")
                .bind(solicitud1)
                .fetch_one(&pool)
                .await?;

        assert_eq!(estado, "aceptada");

        // Borramos los datos de prueba
        sqlx::query("DELETE FROM solicitudes_amistad WHERE id = $1 OR id = $2")
            .bind(solicitud1)
            .bind(solicitud2)
            .execute(&pool)
            .await?;

        sqlx::query("DELETE FROM usuarios WHERE id = $1 OR id = $2 OR id = $3")
            .bind(ana)
            .bind(beto)
            .bind(carla)
            .execute(&pool)
            .await?;

        Ok(())
    }
}
