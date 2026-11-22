use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use axum::{
    Json, Router,
    extract::{State as AxumState, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct State {
    pub(crate) pool: PgPool,
    secret: String,
    duracion_sesion: Duration,
}
impl State {
    pub fn new(pool: PgPool, secret: String) -> Self {
        Self {
            pool,
            secret,
            duracion_sesion: Duration::minutes(60),
        }
    }

    pub fn con_modo_demostracion(mut self, habilitado: bool) -> Self {
        if habilitado {
            self.duracion_sesion = Duration::minutes(1);
        }
        self
    }
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct Registro {
    nombre_usuario: String,
    nombre_visible: String,
    correo: String,
    password: String,
}
#[derive(Deserialize)]
struct Credenciales {
    correo: String,
    password: String,
}
#[derive(Serialize)]
struct Usuario {
    id: Uuid,
    nombre_usuario: String,
    nombre_visible: String,
    correo: String,
    fecha_alta: DateTime<Utc>,
}
#[derive(Serialize)]
struct Sesion {
    token: String,
    vencimiento: DateTime<Utc>,
    usuario: Usuario,
}
#[derive(Serialize)]
struct Error {
    campo: &'static str,
    mensaje: &'static str,
}
type ApiError = (StatusCode, Json<Error>);
fn error(status: StatusCode, campo: &'static str, mensaje: &'static str) -> ApiError {
    (status, Json(Error { campo, mensaje }))
}
fn interno() -> ApiError {
    error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "general",
        "No se pudo completar la operación.",
    )
}
fn credenciales_invalidas() -> ApiError {
    error(
        StatusCode::UNAUTHORIZED,
        "general",
        "Correo o contraseña incorrectos.",
    )
}
fn token_invalido() -> ApiError {
    error(
        StatusCode::UNAUTHORIZED,
        "general",
        "Sesión inválida o vencida.",
    )
}

fn validar_registro(r: &Registro) -> Result<(), ApiError> {
    if !(3..=30).contains(&r.nombre_usuario.len())
        || !r
            .nombre_usuario
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'.' || c == b'_')
    {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "nombre_usuario",
            "Usa entre 3 y 30 letras minúsculas, dígitos, puntos o guiones bajos.",
        ));
    }
    if r.nombre_visible.trim().is_empty() {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "nombre_visible",
            "El nombre visible es obligatorio.",
        ));
    }
    if !correo_valido(&r.correo) {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "correo",
            "Introduce un correo válido.",
        ));
    }
    if r.password.chars().count() < 10 {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "password",
            "La contraseña debe tener al menos 10 caracteres.",
        ));
    }
    Ok(())
}
fn correo_valido(correo: &str) -> bool {
    let Some((local, dominio)) = correo.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !local.starts_with('.')
        && !local.ends_with('.')
        && !local.contains("..")
        && local
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b".!#$%&'*+/=?^_`{|}~-".contains(&c))
        && dominio.len() <= 253
        && dominio.split('.').count() >= 2
        && dominio.split('.').all(|p| {
            !p.is_empty()
                && !p.starts_with('-')
                && !p.ends_with('-')
                && p.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
        })
}
fn usuario(row: &sqlx::postgres::PgRow) -> Usuario {
    Usuario {
        id: row.get("id"),
        nombre_usuario: row.get("nombre_usuario"),
        nombre_visible: row.get("nombre_visible"),
        correo: row.get("correo"),
        fecha_alta: row.get("fecha_alta"),
    }
}
async fn registrar(
    AxumState(s): AxumState<State>,
    body: Result<Json<Registro>, JsonRejection>,
) -> Result<(StatusCode, Json<Usuario>), ApiError> {
    let Json(r) = body.map_err(|_| {
        error(
            StatusCode::BAD_REQUEST,
            "general",
            "Datos de registro inválidos.",
        )
    })?;
    validar_registro(&r)?;
    let hash = tokio::task::spawn_blocking(move || {
        Argon2::default()
            .hash_password(r.password.as_bytes(), &SaltString::generate(&mut OsRng))
            .map(|h| h.to_string())
            .map(|h| (r.nombre_usuario, r.nombre_visible, r.correo, h))
            .map_err(|_| ())
    })
    .await
    .map_err(|_| interno())?
    .map_err(|_| interno())?;
    let result = sqlx::query("INSERT INTO usuarios (id,nombre_usuario,nombre_visible,correo,password_hash) VALUES ($1,$2,$3,$4,$5) RETURNING id,nombre_usuario,nombre_visible,correo,fecha_alta")
        .bind(Uuid::new_v4()).bind(&hash.0).bind(hash.1.trim()).bind(hash.2.to_lowercase()).bind(&hash.3).fetch_one(&s.pool).await;
    match result {
        Ok(row) => Ok((StatusCode::CREATED, Json(usuario(&row)))),
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            let campo = if e.constraint() == Some("usuarios_correo_normalizado") {
                "correo"
            } else {
                "nombre_usuario"
            };
            Err(error(
                StatusCode::CONFLICT,
                campo,
                "Este campo ya está registrado.",
            ))
        }
        Err(_) => Err(interno()),
    }
}
#[derive(Serialize, Deserialize)]
struct Claims {
    sub: Uuid,
    exp: usize,
}
async fn iniciar(
    AxumState(s): AxumState<State>,
    body: Result<Json<Credenciales>, JsonRejection>,
) -> Result<Json<Sesion>, ApiError> {
    let Json(c) = body.map_err(|_| {
        error(
            StatusCode::BAD_REQUEST,
            "general",
            "Datos de inicio de sesión inválidos.",
        )
    })?;
    let row = sqlx::query("SELECT id,nombre_usuario,nombre_visible,correo,fecha_alta,password_hash FROM usuarios WHERE lower(correo) = lower($1)").bind(&c.correo).fetch_optional(&s.pool).await.map_err(|_| interno())?;
    // A dummy Argon2 hash keeps the missing-user path on the same expensive verification path.
    let hash: String = row.as_ref().map(|r| r.get("password_hash")).unwrap_or_else(|| "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHRzb21lc2FsdA$u6B8ACMG5q7AnDYaOAYTL3R4cO3l5EfE0rZp8Qm8STI".into());
    let valid = tokio::task::spawn_blocking(move || {
        PasswordHash::new(&hash).ok().is_some_and(|h| {
            Argon2::default()
                .verify_password(c.password.as_bytes(), &h)
                .is_ok()
        })
    })
    .await
    .map_err(|_| interno())?;
    let row = row.filter(|_| valid).ok_or_else(credenciales_invalidas)?;
    let user = usuario(&row);
    let vencimiento = Utc::now() + s.duracion_sesion;
    let token = encode(
        &Header::default(),
        &Claims {
            sub: user.id,
            exp: vencimiento.timestamp() as usize,
        },
        &EncodingKey::from_secret(s.secret.as_bytes()),
    )
    .map_err(|_| interno())?;
    Ok(Json(Sesion {
        token,
        vencimiento,
        usuario: user,
    }))
}
pub(crate) fn autenticar(headers: &HeaderMap, state: &State) -> Result<Uuid, StatusCode> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let mut validation = Validation::default();
    validation.leeway = 0;
    let claims = decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.secret.as_bytes()),
        &validation,
    )
    .map_err(|_| StatusCode::UNAUTHORIZED)?
    .claims;
    if claims.exp <= Utc::now().timestamp() as usize {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(claims.sub)
}

async fn perfil(
    AxumState(s): AxumState<State>,
    headers: HeaderMap,
) -> Result<Json<Usuario>, ApiError> {
    let token = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(token_invalido)?;

    let mut validation = Validation::default();
    validation.leeway = 0; // Sin margen de tolerancia

    let claims = decode::<Claims>(
        token,
        &DecodingKey::from_secret(s.secret.as_bytes()),
        &validation,
    )
    .map_err(|err| {
        // Imprime o verifica si el error específico fue por expiración (ExpiredSignature)
        println!("Error decodificando token: {:?}", err);
        token_invalido()
    })?
    .claims;

    // Comprobación manual adicional por seguridad estricta
    let ahora_timestamp = Utc::now().timestamp() as usize;
    if claims.exp < ahora_timestamp {
        return Err(token_invalido()); // Retorna 401 Unauthorized si ya pasó el tiempo
    }

    let row = sqlx::query(
        "SELECT id,nombre_usuario,nombre_visible,correo,fecha_alta FROM usuarios WHERE id=$1",
    )
    .bind(claims.sub)
    .fetch_optional(&s.pool)
    .await
    .map_err(|_| interno())?
    .ok_or_else(token_invalido)?;

    Ok(Json(usuario(&row)))
}
pub fn routes(state: State) -> Router {
    Router::new()
        .route("/usuarios", post(registrar))
        .route("/sesiones", post(iniciar))
        .route("/usuarios/yo", get(perfil))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn registro() -> Registro {
        Registro {
            nombre_usuario: "ana_1".into(),
            nombre_visible: "Ana".into(),
            correo: "ana@example.com".into(),
            password: "1234567890".into(),
        }
    }
    #[tokio::test]
    async fn rutas_rechazan_antes_de_consultar_la_base_de_datos() {
        use axum::{body::Body, http::Request};
        use tower::ServiceExt;
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://test:test@127.0.0.1/test")
            .unwrap();
        let app = routes(State::new(
            pool,
            "secreto de prueba con al menos 32 bytes".into(),
        ));
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/usuarios/yo")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let response = app.oneshot(Request::builder().method("POST").uri("/usuarios").header("content-type", "application/json").body(Body::from(r#"{"nombre_usuario":"ana","nombre_visible":"Ana","correo":"invalido","password":"1234567890"}"#)).unwrap()).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    // Requiere una base PostgreSQL dedicada configurada en E5_TEST_DATABASE_URL.
    #[tokio::test]
    async fn registro_y_sesion_con_postgres() {
        use axum::{
            body::{Body, to_bytes},
            http::Request,
        };
        use tower::ServiceExt;
        let url = std::env::var("E5_TEST_DATABASE_URL")
            .expect("Configura E5_TEST_DATABASE_URL con una base de pruebas");
        let pool = sqlx::PgPool::connect(&url).await.unwrap();
        sqlx::raw_sql(include_str!("../migrations/001_usuarios.sql"))
            .execute(&pool)
            .await
            .unwrap();
        let app = routes(State::new(
            pool.clone(),
            "secreto de prueba con al menos 32 bytes".into(),
        ));
        let nombre = format!("u{}", &Uuid::new_v4().simple().to_string()[..20]);
        let correo = format!("{}@example.com", nombre);
        let registro = serde_json::json!({"nombre_usuario": nombre, "nombre_visible": "Ana", "correo": correo, "password": "clave-super-segura"});
        let request = |ruta: &str, body: serde_json::Value| {
            Request::builder()
                .method("POST")
                .uri(ruta)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap()
        };
        let response = app
            .clone()
            .oneshot(request("/usuarios", registro.clone()))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = to_bytes(response.into_body(), 65536).await.unwrap();
        let user: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(user.get("password_hash").is_none());
        assert!(user.get("password").is_none());
        let row = sqlx::query("SELECT password_hash FROM usuarios WHERE nombre_usuario=$1")
            .bind(&nombre)
            .fetch_one(&pool)
            .await
            .unwrap();
        let stored: String = row.get("password_hash");
        assert!(!stored.contains("clave-super-segura"));
        assert!(
            Argon2::default()
                .verify_password(b"clave-super-segura", &PasswordHash::new(&stored).unwrap())
                .is_ok()
        );
        let mut duplicate = registro.clone();
        duplicate["nombre_usuario"] =
            format!("v{}", &Uuid::new_v4().simple().to_string()[..20]).into();
        duplicate["correo"] = correo.to_uppercase().into();
        let response = app
            .clone()
            .oneshot(request("/usuarios", duplicate))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let body = to_bytes(response.into_body(), 65536).await.unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap()["campo"],
            "correo"
        );
        let response = app
            .clone()
            .oneshot(request("/usuarios", registro))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let body = to_bytes(response.into_body(), 65536).await.unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap()["campo"],
            "nombre_usuario"
        );
        let response = app
            .clone()
            .oneshot(request(
                "/sesiones",
                serde_json::json!({"correo": correo, "password": "clave-super-segura"}),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 65536).await.unwrap();
        let session: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let token = session["token"].as_str().unwrap();
        let decodificado = decode::<Claims>(
            token,
            &DecodingKey::from_secret(b"secreto de prueba con al menos 32 bytes"),
            &Validation::default(),
        )
        .unwrap()
        .claims;
        let restante = decodificado.exp as i64 - Utc::now().timestamp();
        assert!(
            (3580..=3600).contains(&restante),
            "Vigencia normal inesperada: {restante}"
        );
        let demo = routes(
            State::new(
                pool.clone(),
                "secreto de prueba con al menos 32 bytes".into(),
            )
            .con_modo_demostracion(true),
        );
        let respuesta_demo = demo
            .oneshot(request(
                "/sesiones",
                serde_json::json!({"correo": correo, "password": "clave-super-segura"}),
            ))
            .await
            .unwrap();
        assert_eq!(respuesta_demo.status(), StatusCode::OK);
        let datos_demo = to_bytes(respuesta_demo.into_body(), 65536).await.unwrap();
        let sesion_demo: serde_json::Value = serde_json::from_slice(&datos_demo).unwrap();
        let claims_demo = decode::<Claims>(
            sesion_demo["token"].as_str().unwrap(),
            &DecodingKey::from_secret(b"secreto de prueba con al menos 32 bytes"),
            &Validation::default(),
        )
        .unwrap()
        .claims;
        let restante_demo = claims_demo.exp as i64 - Utc::now().timestamp();
        assert!(
            (40..=60).contains(&restante_demo),
            "Vigencia de demostración inesperada: {restante_demo}"
        );
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/usuarios/yo")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let mut errors = Vec::new();
        for mail in [&correo, &"noexiste@example.com".to_string()] {
            let response = app
                .clone()
                .oneshot(request(
                    "/sesiones",
                    serde_json::json!({"correo": mail, "password": "incorrecta"}),
                ))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            errors.push(to_bytes(response.into_body(), 65536).await.unwrap());
        }
        assert_eq!(errors[0], errors[1]);
        sqlx::query("DELETE FROM usuarios WHERE nombre_usuario=$1")
            .bind(&nombre)
            .execute(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn vigencia_normal_y_modo_demostracion() {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://test:test@127.0.0.1/test")
            .unwrap();
        let normal = State::new(pool.clone(), "secreto de prueba".into());
        let demostracion = State::new(pool, "secreto de prueba".into()).con_modo_demostracion(true);
        assert_eq!(normal.duracion_sesion, Duration::minutes(60));
        assert_eq!(demostracion.duracion_sesion, Duration::minutes(1));
    }

    #[test]
    fn acepta_password_en_registro_y_sesion() {
        let registro = serde_json::json!({"nombre_usuario":"ana", "nombre_visible":"Ana", "correo":"ana@example.com", "password":"1234567890"});
        let datos: Registro = serde_json::from_value(registro).unwrap();
        assert!(validar_registro(&datos).is_ok());
        let credenciales = serde_json::json!({"correo":"ana@example.com", "password":"1234567890"});
        let datos: Credenciales = serde_json::from_value(credenciales).unwrap();
        assert_eq!(datos.password, "1234567890");
    }

    #[test]
    fn validaciones_de_registro() {
        assert!(validar_registro(&registro()).is_ok());
        for nombre in ["ab", "ANA", "abc-", "abcdefghijklmnopqrstuvwxyz12345"] {
            let mut r = registro();
            r.nombre_usuario = nombre.into();
            assert_eq!(
                validar_registro(&r).unwrap_err().1.0.campo,
                "nombre_usuario"
            );
        }
        let mut r = registro();
        r.nombre_visible = "  ".into();
        assert_eq!(
            validar_registro(&r).unwrap_err().1.0.campo,
            "nombre_visible"
        );
        for correo in ["a@", "@x.com", "a@x", "a@@x.com", "a b@x.com"] {
            let mut r = registro();
            r.correo = correo.into();
            assert_eq!(validar_registro(&r).unwrap_err().1.0.campo, "correo");
        }
        let mut r = registro();
        r.password = "123456789".into();
        assert_eq!(validar_registro(&r).unwrap_err().1.0.campo, "password");
    }
    #[test]
    fn hash_es_salado_y_verificable() {
        let a = Argon2::default()
            .hash_password(b"1234567890", &SaltString::generate(&mut OsRng))
            .unwrap()
            .to_string();
        let b = Argon2::default()
            .hash_password(b"1234567890", &SaltString::generate(&mut OsRng))
            .unwrap()
            .to_string();
        assert_ne!(a, b);
        assert!(!a.contains("1234567890"));
        assert!(
            Argon2::default()
                .verify_password(b"1234567890", &PasswordHash::new(&a).unwrap())
                .is_ok()
        );
    }
    #[test]
    fn tokens_alterados_y_vencidos_se_rechazan() {
        let secret = b"secreto para pruebas suficientemente largo";
        let mut validation = Validation::default();
        validation.leeway = 0;
        for exp in [
            (Utc::now() - Duration::minutes(1)).timestamp() as usize,
            (Utc::now() + Duration::minutes(1)).timestamp() as usize,
        ] {
            let token = encode(
                &Header::default(),
                &Claims {
                    sub: Uuid::new_v4(),
                    exp,
                },
                &EncodingKey::from_secret(secret),
            )
            .unwrap();
            assert_eq!(
                decode::<Claims>(&token, &DecodingKey::from_secret(secret), &validation).is_ok(),
                exp > Utc::now().timestamp() as usize
            );
            assert!(
                decode::<Claims>(
                    &token,
                    &DecodingKey::from_secret(b"otro secreto"),
                    &validation
                )
                .is_err()
            );
        }
    }
}
