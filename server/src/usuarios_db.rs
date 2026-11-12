use sqlx::{PgPool, Row};
use uuid::Uuid;

pub struct UsuarioBuscado {
    pub id: Uuid,
    pub nombre_usuario: String,
    pub nombre_visible: String,
}

pub async fn buscar_usuario_por_nombre(
    pool: &PgPool,
    nombre: &str,
) -> Result<Option<UsuarioBuscado>, sqlx::Error> {
    // Para el usuario por su nombre
    let datos = sqlx::query(
        "SELECT id, nombre_usuario, nombre_visible FROM usuarios WHERE nombre_usuario=$1"
    )
    .bind(nombre)
    .fetch_optional(pool)
    .await?;

    if let Some(fila) = datos {
        return Ok(Some(UsuarioBuscado {
            id: fila.get("id"),
            nombre_usuario: fila.get("nombre_usuario"),
            nombre_visible: fila.get("nombre_visible"),
        }));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requiere PostgreSQL de pruebas"]
    async fn buscar_por_nombre_usuario_exacto() {
        let url = std::env::var("E5_TEST_DATABASE_URL")
            .expect("Configura E5_TEST_DATABASE_URL con una base de pruebas");

        let pool = PgPool::connect(&url).await.unwrap();

        sqlx::raw_sql(include_str!("../migrations/001_usuarios.sql"))
            .execute(&pool)
            .await
            .unwrap();

        // Agregamos un usuario para hacer la prueba
        let id = Uuid::new_v4();
        let nombre = format!("u{}", &id.simple().to_string()[..20]);
        let correo = format!("{nombre}@example.com");

        sqlx::query(
            "INSERT INTO usuarios (id, nombre_usuario, nombre_visible, correo, password_hash)
             VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(id)
        .bind(&nombre)
        .bind("Ana de prueba")
        .bind(&correo)
        .bind("hash_solo_para_prueba")
        .execute(&pool)
        .await
        .unwrap();

        let encontrado = buscar_usuario_por_nombre(&pool, &nombre)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(encontrado.id, id);
        assert_eq!(encontrado.nombre_usuario, nombre);
        assert_eq!(encontrado.nombre_visible, "Ana de prueba");

        // Probamos con el nombre en mayusculas y con una letra menos
        assert!(
            buscar_usuario_por_nombre(&pool, &nombre.to_uppercase())
                .await
                .unwrap()
                .is_none()
        );

        assert!(
            buscar_usuario_por_nombre(&pool, &nombre[..nombre.len() - 1])
                .await
                .unwrap()
                .is_none()
        );

        sqlx::query("DELETE FROM usuarios WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }
    #[tokio::test]
    #[ignore = "requiere PostgreSQL de pruebas"]
    async fn guardar_consultar_y_revisar_usuarios() -> Result<(), sqlx::Error> {
        let url = std::env::var("E5_TEST_DATABASE_URL")
            .expect("Falta E5_TEST_DATABASE_URL");

        let pool = PgPool::connect(&url).await?;

        let base: String = sqlx::query_scalar("SELECT current_database()")
            .fetch_one(&pool)
            .await?;

        assert_eq!(base, "cuentas_claras_pruebas");

        sqlx::raw_sql(include_str!("../migrations/001_usuarios.sql"))
            .execute(&pool)
            .await?;

        let id = Uuid::new_v4();
        let nombre = format!("u{}", &id.simple().to_string()[..20]);
        let correo = format!("{nombre}@example.com");

        let insertar = "INSERT INTO usuarios (id, nombre_usuario, nombre_visible, correo, password_hash)
                        VALUES ($1, $2, $3, $4, $5)";

        sqlx::query(insertar)
            .bind(id)
            .bind(&nombre)
            .bind("Usuario de prueba")
            .bind(&correo)
            .bind("hash_solo_para_prueba")
            .execute(&pool)
            .await?;

        let encontrado: Uuid = sqlx::query_scalar(
            "SELECT id FROM usuarios WHERE lower(correo) = lower($1)"
        )
        .bind(correo.to_uppercase())
        .fetch_one(&pool)
        .await?;

        assert_eq!(encontrado, id);

        let encontrado: String = sqlx::query_scalar(
            "SELECT nombre_usuario FROM usuarios WHERE id = $1"
        )
        .bind(id)
        .fetch_one(&pool)
        .await?;

        assert_eq!(encontrado, nombre);

        let nombre2 = format!("v{}", &Uuid::new_v4().simple().to_string()[..20]);
        let correo2 = format!("{nombre2}@example.com");

        // Usamos otro correo para poder probar el nombre repetido
        let repetido = sqlx::query(insertar)
            .bind(Uuid::new_v4())
            .bind(&nombre)
            .bind("Otro usuario")
            .bind(&correo2)
            .bind("hash_solo_para_prueba")
            .execute(&pool)
            .await;

        let error = repetido.unwrap_err();
        assert!(error.as_database_error().unwrap().is_unique_violation());

        // Ahora cambiamos el nombre y repetimos el correo en mayúsculas
        let repetido = sqlx::query(insertar)
            .bind(Uuid::new_v4())
            .bind(&nombre2)
            .bind("Otro usuario")
            .bind(correo.to_uppercase())
            .bind("hash_solo_para_prueba")
            .execute(&pool)
            .await;

        let error = repetido.unwrap_err();
        assert!(error.as_database_error().unwrap().is_unique_violation());

        pool.close().await;

        let pool = PgPool::connect(&url).await?;

        let guardado: String = sqlx::query_scalar(
            "SELECT nombre_usuario FROM usuarios WHERE id = $1"
        )
        .bind(id)
        .fetch_one(&pool)
        .await?;

        assert_eq!(guardado, nombre);

        sqlx::query("DELETE FROM usuarios WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await?;

        Ok(())
    }
}