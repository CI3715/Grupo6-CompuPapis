use sqlx::{PgPool, postgres::PgPoolOptions};

pub async fn conectar_y_migrar(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    ejecutar_migraciones(&pool).await?;
    Ok(pool)
}

pub async fn ejecutar_migraciones(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

//prueba
#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn migraciones_funcionan_en_bd_nueva_y_con_datos_existentes() {
        let Ok(url) = std::env::var("E5_TEST_DATABASE_URL") else {
            return;
        };

        let pool = conectar_y_migrar(&url)
            .await
            .expect("Debe conectar y migrar por primera vez");

        let id = Uuid::new_v4();
        let usuario = format!("test_{}", id.simple());
        let correo = format!("{}@ejemplo.com", id.simple());

        sqlx::query(
            "INSERT INTO usuarios (id, nombre_usuario, nombre_visible, correo, password_hash)
             VALUES ($1, $2, 'Usuario Prueba', $3, 'hash_falso')",
        )
        .bind(id)
        .bind(&usuario)
        .bind(&correo)
        .execute(&pool)
        .await
        .expect("Debe insertar usuario de prueba");

        // Segunda ejecución sobre una BD que ya tiene datos (idempotencia)
        ejecutar_migraciones(&pool)
            .await
            .expect("Re-ejecutar migraciones no debe fallar ni borrar datos");

        let sigue_existiendo: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM usuarios WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(sigue_existiendo.0, 1);

        // Limpieza
        let _ = sqlx::query("DELETE FROM usuarios WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await;
    }
}