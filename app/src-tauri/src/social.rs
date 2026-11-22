use crate::{
    AppState, PingState,
    autenticacion::{AuthState, ErrorAuth},
};
use serde_json::{Value, json};
use tauri::State;
use uuid::Uuid;

pub(crate) struct RespuestaSocial {
    pub estado: reqwest::StatusCode,
    pub cuerpo: Value,
}

pub(crate) trait ClienteSocial {
    async fn enviar_gasto(
        &self,
        grupo: &str,
        clave: &str,
        datos: &Value,
    ) -> Result<RespuestaSocial, ()>;
    async fn obtener_cambios(&self, cursor: i64) -> Result<RespuestaSocial, ()>;
}

pub(crate) struct ClienteSocialHttp<'a> {
    pub cliente: &'a reqwest::Client,
    pub base: &'a str,
    pub token: &'a str,
}

impl ClienteSocial for ClienteSocialHttp<'_> {
    async fn enviar_gasto(
        &self,
        grupo: &str,
        clave: &str,
        datos: &Value,
    ) -> Result<RespuestaSocial, ()> {
        let respuesta = self
            .cliente
            .post(format!("{}/grupos/{grupo}/gastos", self.base))
            .bearer_auth(self.token)
            .header("Idempotency-Key", clave)
            .json(datos)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|_| ())?;
        let estado = respuesta.status();
        let cuerpo = respuesta.json().await.unwrap_or(Value::Null);
        Ok(RespuestaSocial { estado, cuerpo })
    }

    async fn obtener_cambios(&self, cursor: i64) -> Result<RespuestaSocial, ()> {
        let respuesta = self
            .cliente
            .get(format!("{}/cambios", self.base))
            .query(&[("desde", cursor)])
            .bearer_auth(self.token)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|_| ())?;
        let estado = respuesta.status();
        let cuerpo = respuesta.json().await.unwrap_or(Value::Null);
        Ok(RespuestaSocial { estado, cuerpo })
    }
}

fn datos_gasto_grupo(datos: &Value) -> Value {
    json!({
        "concepto": datos.get("concepto"),
        "monto": datos.get("monto"),
        "fecha": datos.get("fecha"),
        "pagado_por": datos.get("pagado_por"),
        "regla": datos.get("regla"),
        "participantes": datos.get("participantes"),
    })
}

#[cfg(test)]
#[path = "social_tests.rs"]
mod tests;

#[tauri::command]
pub async fn operacion_social(
    accion: String,
    datos: Value,
    state: State<'_, PingState>,
    auth: State<'_, AuthState>,
    app_state: State<'_, AppState>,
) -> Result<Value, ErrorAuth> {
    let sesion = auth.sesion_actual()?;
    let token = sesion.token;
    let repo = app_state
        .repo
        .lock()
        .map_err(|_| ErrorAuth::general("Error de acceso al almacenamiento."))?
        .para_usuario(&sesion.usuario_id)
        .map_err(|e| ErrorAuth::general(&e))?;
    let cliente = &state.http_client;
    let base = &app_state.server_url;
    let id = || -> Result<Uuid, ErrorAuth> {
        datos
            .get("id")
            .and_then(Value::as_str)
            .and_then(|v| Uuid::parse_str(v).ok())
            .ok_or_else(|| ErrorAuth::general("Identificador inválido."))
    };

    if accion == "registrar_gasto" {
        let grupo_id = id()?;
        let clave = Uuid::new_v4().to_string();
        let payload = datos_gasto_grupo(&datos);

        let hay_pendientes = {
            let mut repo = repo.clone();
            let hay_pendientes = repo
                .listar_cola_gastos()
                .map_err(|e| ErrorAuth::general(&e))?
                .iter()
                .any(|item| item.estado == "pendiente");
            repo.encolar_gasto(
                &clave,
                &grupo_id.to_string(),
                datos.get("concepto").and_then(Value::as_str).unwrap_or(""),
                datos.get("monto").and_then(Value::as_str).unwrap_or(""),
                &payload.to_string(),
            )
            .map_err(|e| ErrorAuth::general(&e))?;
            hay_pendientes
        };
        if hay_pendientes {
            return Ok(json!({ "pendiente": true }));
        }

        let resp = cliente
            .post(format!("{base}/grupos/{grupo_id}/gastos"))
            .bearer_auth(token)
            .header("Idempotency-Key", &clave)
            .json(&payload)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await;

        match resp {
            Ok(response) => {
                if response.status() == reqwest::StatusCode::UNAUTHORIZED {
                    return Err(ErrorAuth {
                        campo: "sesion".into(),
                        mensaje: "Tu sesión venció, inicia sesión de nuevo.".into(),
                    });
                }
                if !response.status().is_success() {
                    if response.status().is_server_error()
                        || response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
                    {
                        return Ok(json!({ "pendiente": true }));
                    }
                    let body = response.json::<Value>().await.unwrap_or_default();
                    let mensaje = body
                        .get("mensaje")
                        .and_then(Value::as_str)
                        .unwrap_or("No se pudo completar la operación.");
                    repo.clone()
                        .marcar_gasto_rechazado(&clave, mensaje)
                        .map_err(|e| ErrorAuth::general(&e))?;
                    return Err(ErrorAuth::general(mensaje));
                }
                let body: Value = response.json().await.unwrap_or(json!({}));
                repo.clone()
                    .eliminar_de_cola(&clave)
                    .map_err(|e| ErrorAuth::general(&e))?;
                return Ok(json!({ "pendiente": false, "datos": body }));
            }
            Err(_) => {
                return Ok(json!({ "pendiente": true }));
            }
        }
    }

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
    let body: Value = response
        .json()
        .await
        .map_err(|_| ErrorAuth::general("No se pudo leer la respuesta del servidor."))?;
    if accion == "detalle_grupo" || accion == "listar_grupos" {
        let grupos: Vec<&Value> = if accion == "detalle_grupo" {
            vec![&body]
        } else {
            body.get("grupos")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .collect()
        };
        for grupo in grupos {
            if let (Some(id), Some(moneda)) = (
                grupo.get("id").and_then(Value::as_str),
                grupo.get("moneda").and_then(Value::as_str),
            ) {
                repo.guardar_moneda_grupo(id, moneda)
                    .map_err(|e| ErrorAuth::general(&e))?;
            }
        }
    }
    Ok(body)
}
