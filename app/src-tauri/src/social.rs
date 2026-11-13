use crate::{
    PingState,
    autenticacion::{AuthState, ErrorAuth},
};
use serde_json::{Value, json};
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub async fn operacion_social(
    accion: String,
    datos: Value,
    state: State<'_, PingState>,
    auth: State<'_, AuthState>,
) -> Result<Value, ErrorAuth> {
    let token = auth
        .token
        .lock()
        .map_err(|_| ErrorAuth::general("No se pudo consultar la sesión."))?
        .clone()
        .ok_or_else(|| ErrorAuth::general("Inicia sesión para continuar."))?;
    let cliente = &state.http_client;
    let base = "http://127.0.0.1:8000";
    let id = || -> Result<Uuid, ErrorAuth> {
        datos
            .get("id")
            .and_then(Value::as_str)
            .and_then(|v| Uuid::parse_str(v).ok())
            .ok_or_else(|| ErrorAuth::general("Identificador inválido."))
    };
    let request = match accion.as_str() {
        "buscar_usuario" => cliente.get(format!("{base}/usuarios")).query(&[("usuario", datos.get("usuario").and_then(Value::as_str).unwrap_or(""))]),
        "listar_amistades" => cliente.get(format!("{base}/amistades")),
        "invitar_amistad" => cliente.post(format!("{base}/amistades")).json(&json!({"nombre_usuario":datos.get("nombre_usuario")})),
        "responder_amistad" => cliente.post(format!("{base}/amistades/{}/respuesta", id()?)).json(&json!({"aceptar":datos.get("aceptar")})),
        "eliminar_amistad" => cliente.delete(format!("{base}/amistades/{}", id()?)),
        "listar_grupos" => cliente.get(format!("{base}/grupos")),
        "crear_grupo" => cliente.post(format!("{base}/grupos")).json(&json!({"nombre":datos.get("nombre"),"descripcion":datos.get("descripcion"),"moneda":datos.get("moneda"),"actividad":datos.get("actividad")})),
        "registrar_gasto" => cliente.post(format!("{base}/grupos/{}/gastos", id()?)).json(&json!({"concepto":datos.get("concepto"),"monto":datos.get("monto"),"fecha":datos.get("fecha"),"pagado_por":datos.get("pagado_por"),"regla":datos.get("regla"),"participantes":datos.get("participantes")})),
        "detalle_grupo" => cliente.get(format!("{base}/grupos/{}", id()?)),
        "invitar_grupo" => cliente.post(format!("{base}/grupos/{}/miembros", id()?)).json(&json!({"usuario_id":datos.get("usuario_id")})),
        "responder_grupo" => cliente.post(format!("{base}/grupos/{}/miembros/yo/respuesta", id()?)).json(&json!({"aceptar":datos.get("aceptar")})),
        _ => return Err(ErrorAuth::general("Operación no disponible.")),
    };
    let response = request
        .bearer_auth(token)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|_| {
            ErrorAuth::general(
                "No se pudo conectar con el servidor. Tus cuentas siguen disponibles.",
            )
        })?;
    if !response.status().is_success() {
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ErrorAuth {
                campo: "sesion".into(),
                mensaje: "Tu sesión venció, inicia sesión de nuevo.".into(),
            });
        }
        let body = response.json::<Value>().await.unwrap_or_default();
        return Err(ErrorAuth::general(
            body.get("mensaje")
                .and_then(Value::as_str)
                .unwrap_or("No se pudo completar la operación."),
        ));
    }
    if response.status() == reqwest::StatusCode::NO_CONTENT {
        return Ok(Value::Null);
    }
    response
        .json()
        .await
        .map_err(|_| ErrorAuth::general("No se pudo leer la respuesta del servidor."))
}
