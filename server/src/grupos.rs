use crate::{
    amistades_db,
    auth::{self, State},
};
use axum::{
    Json, Router,
    extract::{Path, State as AxumState},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

type Api<T> = Result<T, (StatusCode, Json<Value>)>;
fn error(s: StatusCode, m: &str) -> (StatusCode, Json<Value>) {
    (s, Json(json!({"mensaje":m})))
}
fn db(_: sqlx::Error) -> (StatusCode, Json<Value>) {
    error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "No se pudo completar la operación.",
    )
}
fn user(h: &HeaderMap, s: &State) -> Api<Uuid> {
    auth::autenticar(h, s)
        .map_err(|_| error(StatusCode::UNAUTHORIZED, "Sesión inválida o vencida."))
}
fn oculto() -> (StatusCode, Json<Value>) {
    error(StatusCode::NOT_FOUND, "Grupo no encontrado.")
}
async fn miembro(s: &State, g: Uuid, u: Uuid) -> Api<bool> {
    sqlx::query_scalar("SELECT es_admin FROM grupo_miembros WHERE grupo_id=$1 AND usuario_id=$2 AND estado_invitacion='aceptada'").bind(g).bind(u).fetch_optional(&s.pool).await.map_err(db)?.ok_or_else(oculto)
}
#[derive(Deserialize)]
struct Crear {
    nombre: String,
    descripcion: Option<String>,
    moneda: String,
}
async fn crear(
    AxumState(s): AxumState<State>,
    h: HeaderMap,
    Json(q): Json<Crear>,
) -> Api<(StatusCode, Json<Value>)> {
    let u = user(&h, &s)?;
    if q.nombre.trim().is_empty()
        || q.moneda.len() != 3
        || !q.moneda.bytes().all(|x| x.is_ascii_uppercase())
    {
        return Err(error(StatusCode::BAD_REQUEST, "Nombre o moneda inválidos."));
    }
    let id = Uuid::new_v4();
    let mut tx = s.pool.begin().await.map_err(db)?;
    sqlx::query(
        "INSERT INTO grupos(id,nombre,descripcion,moneda,creador_id) VALUES($1,$2,$3,$4,$5)",
    )
    .bind(id)
    .bind(q.nombre.trim())
    .bind(&q.descripcion)
    .bind(&q.moneda)
    .bind(u)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    sqlx::query("INSERT INTO grupo_miembros(grupo_id,usuario_id,es_admin,estado_invitacion) VALUES($1,$2,true,'aceptada')").bind(id).bind(u).execute(&mut *tx).await.map_err(db)?;
    tx.commit().await.map_err(db)?;
    Ok((
        StatusCode::CREATED,
        Json(
            json!({"id":id,"nombre":q.nombre.trim(),"descripcion":q.descripcion,"moneda":q.moneda}),
        ),
    ))
}
async fn listar(AxumState(s): AxumState<State>, h: HeaderMap) -> Api<Json<Value>> {
    let u = user(&h, &s)?;
    let rows=sqlx::query("SELECT g.id,g.nombre,g.descripcion,g.moneda,m.estado_invitacion,m.es_admin FROM grupos g JOIN grupo_miembros m ON m.grupo_id=g.id WHERE m.usuario_id=$1 AND m.estado_invitacion IN ('aceptada','pendiente') ORDER BY g.fecha_creacion,g.id").bind(u).fetch_all(&s.pool).await.map_err(db)?;
    let mut grupos = Vec::new();
    let mut invitaciones = Vec::new();
    for r in rows {
        let estado: String = r.get("estado_invitacion");
        let v = json!({"id":r.get::<Uuid,_>("id"),"nombre":r.get::<String,_>("nombre"),"descripcion":r.get::<Option<String>,_>("descripcion"),"moneda":r.get::<String,_>("moneda"),"es_admin":r.get::<bool,_>("es_admin")});
        if estado == "aceptada" {
            grupos.push(v)
        } else {
            invitaciones.push(v)
        }
    }
    Ok(Json(json!({"grupos":grupos,"invitaciones":invitaciones})))
}
#[derive(Deserialize)]
struct Invitar {
    usuario_id: Uuid,
}
async fn invitar(
    AxumState(s): AxumState<State>,
    h: HeaderMap,
    Path(g): Path<Uuid>,
    Json(q): Json<Invitar>,
) -> Api<(StatusCode, Json<Value>)> {
    let u = user(&h, &s)?;
    if !miembro(&s, g, u).await? {
        return Err(error(
            StatusCode::FORBIDDEN,
            "Solo el administrador puede invitar.",
        ));
    }
    if !amistades_db::son_amigos(&s.pool, u, q.usuario_id)
        .await
        .map_err(db)?
    {
        return Err(error(
            StatusCode::FORBIDDEN,
            "Solo puedes invitar a tus amigos.",
        ));
    }
    let r=sqlx::query("INSERT INTO grupo_miembros(grupo_id,usuario_id,estado_invitacion) VALUES($1,$2,'pendiente') ON CONFLICT(grupo_id,usuario_id) DO UPDATE SET estado_invitacion='pendiente' WHERE grupo_miembros.estado_invitacion='rechazada'").bind(g).bind(q.usuario_id).execute(&s.pool).await.map_err(db)?;
    if r.rows_affected() == 0 {
        return Err(error(
            StatusCode::CONFLICT,
            "Ya existe una invitación o membresía.",
        ));
    }
    Ok((
        StatusCode::CREATED,
        Json(json!({"grupo_id":g,"usuario_id":q.usuario_id,"estado":"pendiente"})),
    ))
}
#[derive(Deserialize)]
struct Respuesta {
    aceptar: bool,
}
async fn responder(
    AxumState(s): AxumState<State>,
    h: HeaderMap,
    Path(g): Path<Uuid>,
    Json(q): Json<Respuesta>,
) -> Api<Json<Value>> {
    let u = user(&h, &s)?;
    let estado = if q.aceptar { "aceptada" } else { "rechazada" };
    let r=sqlx::query("UPDATE grupo_miembros SET estado_invitacion=$3,fecha_union=now() WHERE grupo_id=$1 AND usuario_id=$2 AND estado_invitacion='pendiente'").bind(g).bind(u).bind(estado).execute(&s.pool).await.map_err(db)?;
    if r.rows_affected() == 0 {
        return Err(error(
            StatusCode::CONFLICT,
            "No hay una invitación pendiente.",
        ));
    }
    Ok(Json(json!({"grupo_id":g,"estado":estado})))
}
async fn detalle(
    AxumState(s): AxumState<State>,
    h: HeaderMap,
    Path(g): Path<Uuid>,
) -> Api<Json<Value>> {
    let u = user(&h, &s)?;
    miembro(&s, g, u).await?;
    let r = sqlx::query("SELECT nombre,descripcion,moneda FROM grupos WHERE id=$1")
        .bind(g)
        .fetch_optional(&s.pool)
        .await
        .map_err(db)?
        .ok_or_else(oculto)?;
    let miembros=sqlx::query("SELECT u.id,u.nombre_usuario,u.nombre_visible,m.es_admin FROM grupo_miembros m JOIN usuarios u ON u.id=m.usuario_id WHERE m.grupo_id=$1 AND m.estado_invitacion='aceptada' ORDER BY u.nombre_usuario").bind(g).fetch_all(&s.pool).await.map_err(db)?.iter().map(|r|json!({"id":r.get::<Uuid,_>("id"),"nombre_usuario":r.get::<String,_>("nombre_usuario"),"nombre_visible":r.get::<String,_>("nombre_visible"),"es_admin":r.get::<bool,_>("es_admin")})).collect::<Vec<_>>();
    // Hasta disponer del contrato de gastos completo, no devolver un balance ficticio.
    Ok(Json(
        json!({"id":g,"nombre":r.get::<String,_>("nombre"),"descripcion":r.get::<Option<String>,_>("descripcion"),"moneda":r.get::<String,_>("moneda"),"miembros":miembros}),
    ))
}
pub fn routes(s: State) -> Router {
    Router::new()
        .route("/grupos", post(crear).get(listar))
        .route("/grupos/{id}", get(detalle))
        .route("/grupos/{id}/miembros", post(invitar))
        .route("/grupos/{id}/miembros/yo/respuesta", post(responder))
        .with_state(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use jsonwebtoken::{EncodingKey, Header, encode};
    use serde::Serialize;
    use tower::ServiceExt;
    #[derive(Serialize)]
    struct Claims {
        sub: Uuid,
        exp: i64,
    }
    #[tokio::test]
    #[ignore = "requiere PostgreSQL de pruebas"]
    async fn flujo_grupo_y_aislamiento() {
        let url = std::env::var("E5_TEST_DATABASE_URL").unwrap();
        let pool = crate::db::conectar_y_migrar(&url).await.unwrap();
        let secret = "secreto de prueba con al menos 32 bytes";
        let state = State::new(pool.clone(), secret.into());
        let app = Router::new()
            .merge(routes(state.clone()))
            .merge(crate::social::routes(state));
        let ana = Uuid::new_v4();
        let beto = Uuid::new_v4();
        let ajeno = Uuid::new_v4();
        for id in [ana, beto, ajeno] {
            let nombre = format!("u{}", &id.simple().to_string()[..20]);
            sqlx::query("INSERT INTO usuarios(id,nombre_usuario,nombre_visible,correo,password_hash) VALUES($1,$2,$2,$3,'test')").bind(id).bind(&nombre).bind(format!("{nombre}@example.com")).execute(&pool).await.unwrap();
        }
        let token = |id| {
            encode(
                &Header::default(),
                &Claims {
                    sub: id,
                    exp: (chrono::Utc::now() + chrono::Duration::minutes(5)).timestamp(),
                },
                &EncodingKey::from_secret(secret.as_bytes()),
            )
            .unwrap()
        };
        let request = |method: &str, path: &str, id: Uuid, body: Value| {
            Request::builder()
                .method(method)
                .uri(path)
                .header("authorization", format!("Bearer {}", token(id)))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap()
        };
        let res = app
            .clone()
            .oneshot(request(
                "POST",
                "/amistades",
                ana,
                json!({"nombre_usuario":format!("u{}", &beto.simple().to_string()[..20])}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let b: Value =
            serde_json::from_slice(&to_bytes(res.into_body(), 65536).await.unwrap()).unwrap();
        let solicitud = b["id"].as_str().unwrap();
        let res = app
            .clone()
            .oneshot(request(
                "POST",
                &format!("/amistades/{solicitud}/respuesta"),
                beto,
                json!({"aceptar":true}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let res = app
            .clone()
            .oneshot(request(
                "POST",
                "/grupos",
                ana,
                json!({"nombre":"Viaje","moneda":"USD"}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let b: Value =
            serde_json::from_slice(&to_bytes(res.into_body(), 65536).await.unwrap()).unwrap();
        let grupo = b["id"].as_str().unwrap();
        let res = app
            .clone()
            .oneshot(request(
                "POST",
                &format!("/grupos/{grupo}/miembros"),
                ana,
                json!({"usuario_id":beto}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        for id in [beto, ajeno] {
            let res = app
                .clone()
                .oneshot(request("GET", &format!("/grupos/{grupo}"), id, json!({})))
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::NOT_FOUND);
        }
        let res = app
            .clone()
            .oneshot(request(
                "POST",
                &format!("/grupos/{grupo}/miembros/yo/respuesta"),
                beto,
                json!({"aceptar":true}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let res = app
            .clone()
            .oneshot(request("GET", &format!("/grupos/{grupo}"), beto, json!({})))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        sqlx::query("DELETE FROM grupos WHERE id=$1")
            .bind(Uuid::parse_str(grupo).unwrap())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM amistades WHERE usuario1_id=$1 OR usuario2_id=$1")
            .bind(ana)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM solicitudes_amistad WHERE emisor_id=$1 OR destinatario_id=$1")
            .bind(ana)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM usuarios WHERE id=ANY($1)")
            .bind(&[ana, beto, ajeno][..])
            .execute(&pool)
            .await
            .unwrap();
    }
}
