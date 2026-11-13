use crate::{
    amistades_db,
    auth::{self, State},
    usuarios_db,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State as AxumState},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post},
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

type ResultApi<T> = Result<T, (StatusCode, Json<Value>)>;
fn fallo(status: StatusCode, mensaje: &str) -> (StatusCode, Json<Value>) {
    (status, Json(json!({"mensaje":mensaje})))
}
fn interno(_: sqlx::Error) -> (StatusCode, Json<Value>) {
    fallo(
        StatusCode::INTERNAL_SERVER_ERROR,
        "No se pudo completar la operación.",
    )
}
fn sesion(headers: &HeaderMap, s: &State) -> ResultApi<Uuid> {
    auth::autenticar(headers, s)
        .map_err(|_| fallo(StatusCode::UNAUTHORIZED, "Sesión inválida o vencida."))
}
#[derive(Deserialize)]
struct Buscar {
    usuario: String,
}
async fn buscar(
    AxumState(s): AxumState<State>,
    headers: HeaderMap,
    Query(q): Query<Buscar>,
) -> ResultApi<Json<Value>> {
    sesion(&headers, &s)?;
    let u = usuarios_db::buscar_usuario_por_nombre(&s.pool, &q.usuario)
        .await
        .map_err(interno)?
        .ok_or_else(|| fallo(StatusCode::NOT_FOUND, "Usuario no encontrado."))?;
    Ok(Json(
        json!({"nombre_usuario":u.nombre_usuario,"nombre_visible":u.nombre_visible}),
    ))
}
#[derive(Deserialize)]
struct NuevaAmistad {
    nombre_usuario: String,
}
async fn invitar(
    AxumState(s): AxumState<State>,
    headers: HeaderMap,
    Json(q): Json<NuevaAmistad>,
) -> ResultApi<(StatusCode, Json<Value>)> {
    let u = sesion(&headers, &s)?;
    let destinatario = usuarios_db::buscar_usuario_por_nombre(&s.pool, &q.nombre_usuario)
        .await
        .map_err(interno)?
        .ok_or_else(|| fallo(StatusCode::NOT_FOUND, "Usuario no encontrado."))?
        .id;
    if u == destinatario {
        return Err(fallo(
            StatusCode::BAD_REQUEST,
            "No puedes invitarte a ti mismo.",
        ));
    }
    let id = Uuid::new_v4();
    if !amistades_db::guardar_solicitud(&s.pool, id, u, destinatario, Utc::now())
        .await
        .map_err(interno)?
    {
        return Err(fallo(
            StatusCode::CONFLICT,
            "Ya existe una solicitud o amistad.",
        ));
    }
    Ok((
        StatusCode::CREATED,
        Json(json!({"id":id,"estado":"pendiente"})),
    ))
}
#[derive(Deserialize)]
struct Respuesta {
    aceptar: bool,
}
async fn responder(
    AxumState(s): AxumState<State>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(q): Json<Respuesta>,
) -> ResultApi<Json<Value>> {
    let u = sesion(&headers, &s)?;
    let row: Option<(Uuid, String)> =
        sqlx::query_as("SELECT destinatario_id,estado FROM solicitudes_amistad WHERE id=$1")
            .bind(id)
            .fetch_optional(&s.pool)
            .await
            .map_err(interno)?;
    let (dest, estado) =
        row.ok_or_else(|| fallo(StatusCode::NOT_FOUND, "Solicitud no encontrada."))?;
    if dest != u {
        return Err(fallo(
            StatusCode::FORBIDDEN,
            "Solo el destinatario puede responder.",
        ));
    }
    if estado != "pendiente" {
        return Err(fallo(
            StatusCode::CONFLICT,
            "La solicitud ya fue respondida.",
        ));
    }
    let ok = if q.aceptar {
        amistades_db::aceptar_solicitud(&s.pool, id, u, Uuid::new_v4(), Utc::now()).await
    } else {
        amistades_db::rechazar_solicitud(&s.pool, id, u, Utc::now()).await
    }
    .map_err(interno)?;
    if !ok {
        return Err(fallo(
            StatusCode::CONFLICT,
            "La solicitud ya fue respondida.",
        ));
    }
    Ok(Json(
        json!({"id":id,"estado":if q.aceptar {"aceptada"} else {"rechazada"}}),
    ))
}
async fn listar(AxumState(s): AxumState<State>, headers: HeaderMap) -> ResultApi<Json<Value>> {
    let u = sesion(&headers, &s)?;
    let amigos=amistades_db::listar_amigos(&s.pool,u).await.map_err(interno)?.into_iter().map(|a|json!({"id":a.amistad_id,"usuario_id":a.id,"nombre_usuario":a.nombre_usuario,"nombre_visible":a.nombre_visible})).collect::<Vec<_>>();
    let enviados=amistades_db::solicitudes_enviadas(&s.pool,u).await.map_err(interno)?.into_iter().map(|a|json!({"id":a.id,"emisor_id":a.emisor_id,"destinatario_id":a.destinatario_id,"creada_en":a.creada_en})).collect::<Vec<_>>();
    let recibidos=amistades_db::solicitudes_recibidas(&s.pool,u).await.map_err(interno)?.into_iter().map(|a|json!({"id":a.id,"emisor_id":a.emisor_id,"destinatario_id":a.destinatario_id,"creada_en":a.creada_en})).collect::<Vec<_>>();
    Ok(Json(
        json!({"amigos":amigos,"enviadas":enviados,"recibidas":recibidos}),
    ))
}
async fn eliminar(
    AxumState(s): AxumState<State>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ResultApi<StatusCode> {
    let u = sesion(&headers, &s)?;
    if !amistades_db::eliminar_amistad(&s.pool, id, u)
        .await
        .map_err(interno)?
    {
        return Err(fallo(StatusCode::NOT_FOUND, "Amistad no encontrada."));
    }
    Ok(StatusCode::NO_CONTENT)
}
pub fn routes(state: State) -> Router {
    Router::new()
        .route("/usuarios", get(buscar))
        .route("/amistades", post(invitar).get(listar))
        .route("/amistades/{id}/respuesta", post(responder))
        .route("/amistades/{id}", delete(eliminar))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;
    #[tokio::test]
    async fn rutas_sociales_rechazan_sesion_ausente() {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://test:test@127.0.0.1/test")
            .unwrap();
        let state = State::new(pool, "secreto de prueba suficientemente largo".into());
        let app = Router::new()
            .merge(routes(state.clone()))
            .merge(crate::auth::routes(state.clone()))
            .merge(crate::grupos::routes(state));
        for uri in ["/usuarios?usuario=ana", "/amistades", "/grupos"] {
            let response = app
                .clone()
                .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{uri}");
        }
    }
}
