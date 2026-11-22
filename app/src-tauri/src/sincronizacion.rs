use crate::{
    AppState, PingState,
    autenticacion::AuthState,
    domain::ErrorDominio,
    social::{ClienteSocial, ClienteSocialHttp},
    sqlite_repo::{EventoCambioRecibido, GastoEnColaDb, PendienteAplicarDb, RepositorioSqlite},
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Mutex;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultadoSync {
    pub ultima_sincronizacion: Option<String>,
    pub aplicados: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendienteAplicar {
    pub id: String,
    pub gasto_id: String,
    pub grupo_id: String,
    pub concepto: String,
    pub fecha: String,
    pub moneda: String,
    pub monto_parte: String,
    pub recibio_residuo: bool,
}

impl From<PendienteAplicarDb> for PendienteAplicar {
    fn from(p: PendienteAplicarDb) -> Self {
        Self {
            id: p.id,
            gasto_id: p.gasto_id,
            grupo_id: p.grupo_id,
            concepto: p.concepto,
            fecha: p.fecha,
            moneda: p.moneda,
            monto_parte: p.monto_parte,
            recibio_residuo: p.recibio_residuo,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GastoEnCola {
    pub clave: String,
    pub grupo_id: String,
    pub concepto: String,
    pub monto: String,
    pub estado: String,
    pub motivo: Option<String>,
}

impl From<GastoEnColaDb> for GastoEnCola {
    fn from(g: GastoEnColaDb) -> Self {
        Self {
            clave: g.clave,
            grupo_id: g.grupo_id,
            concepto: g.concepto,
            monto: g.monto,
            estado: g.estado,
            motivo: g.motivo,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RespuestaCambiosServidor {
    pub eventos: Vec<Value>,
    pub nuevo_cursor: i64,
    pub hay_mas: bool,
}

#[tauri::command]
pub fn obtener_cuenta_grupo(
    state: State<'_, AppState>,
    grupo_id: String,
) -> Result<Option<String>, ErrorDominio> {
    let repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
    repo.obtener_cuenta_grupo(&grupo_id)
        .map_err(|e| ErrorDominio::nuevo("general", &e))
}

#[tauri::command]
pub fn elegir_cuenta_grupo(
    state: State<'_, AppState>,
    grupo_id: String,
    cuenta_id: String,
) -> Result<(), ErrorDominio> {
    let mut repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
    repo.elegir_cuenta_grupo_verificada(&grupo_id, &cuenta_id)
        .map_err(|e| ErrorDominio::nuevo("general", &e))?;
    Ok(())
}

#[tauri::command]
pub fn listar_pendientes_aplicar(
    state: State<'_, AppState>,
) -> Result<Vec<PendienteAplicar>, ErrorDominio> {
    let repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
    let pendientes = repo
        .listar_pendientes_aplicar()
        .map_err(|e| ErrorDominio::nuevo("general", &e))?;
    Ok(pendientes.into_iter().map(PendienteAplicar::from).collect())
}

#[tauri::command]
pub fn listar_cola_gastos(state: State<'_, AppState>) -> Result<Vec<GastoEnCola>, ErrorDominio> {
    let repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
    let cola = repo
        .listar_cola_gastos()
        .map_err(|e| ErrorDominio::nuevo("general", &e))?;
    Ok(cola.into_iter().map(GastoEnCola::from).collect())
}

#[tauri::command]
pub fn descartar_rechazo(state: State<'_, AppState>, clave: String) -> Result<(), ErrorDominio> {
    let mut repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
    repo.eliminar_de_cola(&clave)
        .map_err(|e| ErrorDominio::nuevo("general", &e))
}

#[tauri::command]
pub fn reiniciar_cursor(state: State<'_, AppState>) -> Result<(), ErrorDominio> {
    let mut repo = state
        .repo
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
    repo.reiniciar_cursor_sincronizacion()
        .map_err(|e| ErrorDominio::nuevo("general", &e))
}

#[tauri::command]
pub async fn sincronizar(
    state: State<'_, AppState>,
    ping: State<'_, PingState>,
    auth: State<'_, AuthState>,
) -> Result<ResultadoSync, ErrorDominio> {
    let token = auth
        .token
        .lock()
        .map_err(|_| ErrorDominio::nuevo("general", "No se pudo consultar la sesión."))?
        .clone()
        .ok_or_else(|| ErrorDominio::nuevo("sesion", "Inicia sesión para continuar."))?;

    let cliente = ClienteSocialHttp {
        cliente: &ping.http_client,
        base: &state.server_url,
        token: &token,
    };
    sincronizar_con_cliente(&state.repo, &cliente).await
}

pub(crate) async fn sincronizar_con_cliente<C: ClienteSocial>(
    repo: &Mutex<RepositorioSqlite>,
    cliente: &C,
) -> Result<ResultadoSync, ErrorDominio> {
    // Paso 1: Enviar gastos en cola pendientes de envío (RNF-17, ESC-09)
    let pendientes_cola = {
        let repo = repo
            .lock()
            .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
        repo.listar_cola_gastos()
            .map_err(|e| ErrorDominio::nuevo("general", &e))?
    };

    for item in pendientes_cola {
        if item.estado != "pendiente" {
            continue;
        }

        let payload: Value = match serde_json::from_str(&item.datos_json) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let resp = cliente
            .enviar_gasto(&item.grupo_id, &item.clave, &payload)
            .await;

        match resp {
            Ok(response) => {
                if response.estado == StatusCode::UNAUTHORIZED {
                    return Err(ErrorDominio::nuevo(
                        "sesion",
                        "Tu sesión venció, inicia sesión de nuevo.",
                    ));
                }

                if response.estado.is_success() {
                    // 201 Created o 200 OK (ya procesado)
                    let mut repo = repo.lock().map_err(|_| {
                        ErrorDominio::nuevo("general", "Error de acceso al almacenamiento.")
                    })?;
                    repo.eliminar_de_cola(&item.clave)
                        .map_err(|e| ErrorDominio::nuevo("general", &e))?;
                } else if response.estado.is_server_error()
                    || response.estado == StatusCode::TOO_MANY_REQUESTS
                {
                    return Err(ErrorDominio::nuevo(
                        "servidor",
                        "No se pudo enviar la cola; se reintentará más tarde.",
                    ));
                } else {
                    // Error de validación o conflicto del servidor
                    let error_body = response.cuerpo;
                    let motivo = error_body
                        .get("mensaje")
                        .and_then(Value::as_str)
                        .unwrap_or("El servidor rechazó la operación.")
                        .to_string();

                    let mut repo = repo.lock().map_err(|_| {
                        ErrorDominio::nuevo("general", "Error de acceso al almacenamiento.")
                    })?;
                    repo.marcar_gasto_rechazado(&item.clave, &motivo)
                        .map_err(|e| ErrorDominio::nuevo("general", &e))?;
                }
            }
            Err(_) => {
                return Err(ErrorDominio::nuevo(
                    "red",
                    "No se pudo enviar la cola; se reintentará más tarde.",
                ));
            }
        }
    }

    // Paso 2: Descargar cambios incrementales desde el cursor guardado (GET /cambios)
    let mut total_aplicados = 0;
    loop {
        let cursor_actual = {
            let repo = repo.lock().map_err(|_| {
                ErrorDominio::nuevo("general", "Error de acceso al almacenamiento.")
            })?;
            repo.obtener_cursor_sincronizacion()
                .map_err(|e| ErrorDominio::nuevo("general", &e))?
        };

        let response = match cliente.obtener_cambios(cursor_actual).await {
            Ok(r) => r,
            Err(_) => {
                return Err(ErrorDominio::nuevo(
                    "red",
                    "No se pudo conectar con el servidor. Las cuentas siguen disponibles sin conexión.",
                ));
            }
        };

        if response.estado == StatusCode::UNAUTHORIZED {
            return Err(ErrorDominio::nuevo(
                "sesion",
                "Tu sesión venció, inicia sesión de nuevo.",
            ));
        }

        if !response.estado.is_success() {
            let body = response.cuerpo;
            let msg = body
                .get("mensaje")
                .and_then(Value::as_str)
                .unwrap_or("No se pudieron consultar los cambios.");
            return Err(ErrorDominio::nuevo("servidor", msg));
        }

        let lote =
            serde_json::from_value::<RespuestaCambiosServidor>(response.cuerpo).map_err(|_| {
                ErrorDominio::nuevo(
                    "servidor",
                    "No se pudo interpretar la respuesta del servidor.",
                )
            })?;

        if lote
            .eventos
            .iter()
            .any(|ev| ev.get("tipo").and_then(Value::as_str).is_none())
        {
            return Err(ErrorDominio::nuevo(
                "servidor",
                "Un evento recibido no indica su tipo.",
            ));
        }
        let eventos = lote
            .eventos
            .into_iter()
            .filter(|ev| ev.get("tipo").and_then(Value::as_str) == Some("gasto_grupo"))
            .map(serde_json::from_value::<EventoCambioRecibido>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| {
                ErrorDominio::nuevo("servidor", "No se pudo interpretar un gasto de grupo.")
            })?;
        if lote.nuevo_cursor < cursor_actual || (lote.hay_mas && lote.nuevo_cursor <= cursor_actual)
        {
            return Err(ErrorDominio::nuevo(
                "servidor",
                "El cursor recibido no es válido.",
            ));
        }
        let aplicados_lote = {
            let mut repo = repo.lock().map_err(|_| {
                ErrorDominio::nuevo("general", "Error de acceso al almacenamiento.")
            })?;
            repo.aplicar_lote_cambios(lote.nuevo_cursor, eventos)
                .map_err(|e| ErrorDominio::nuevo("general", &e))?
        };

        total_aplicados += aplicados_lote;

        if !lote.hay_mas {
            break;
        }
    }

    let ultima_sync = {
        let repo = repo
            .lock()
            .map_err(|_| ErrorDominio::nuevo("general", "Error de acceso al almacenamiento."))?;
        repo.registrar_sincronizacion_exitosa()
            .map_err(|e| ErrorDominio::nuevo("general", &e))?;
        repo.obtener_ultima_sincronizacion()
            .map_err(|e| ErrorDominio::nuevo("general", &e))?
    };

    Ok(ResultadoSync {
        ultima_sincronizacion: ultima_sync,
        aplicados: total_aplicados,
    })
}

#[cfg(test)]
mod pruebas_motor {
    use super::*;
    use crate::social::RespuestaSocial;
    use serde_json::json;
    use std::collections::VecDeque;
    use uuid::Uuid;

    struct Fake {
        envios: Mutex<Vec<(String, String, Value)>>,
        resultados: Mutex<VecDeque<Result<StatusCode, ()>>>,
        cambios: Mutex<VecDeque<(StatusCode, Value)>>,
        cursores: Mutex<Vec<i64>>,
    }
    impl ClienteSocial for Fake {
        async fn enviar_gasto(
            &self,
            grupo: &str,
            clave: &str,
            datos: &Value,
        ) -> Result<RespuestaSocial, ()> {
            self.envios
                .lock()
                .unwrap()
                .push((grupo.into(), clave.into(), datos.clone()));
            self.resultados
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(Ok(StatusCode::CREATED))
                .map(|estado| RespuestaSocial {
                    estado,
                    cuerpo: json!({"mensaje":"rechazado"}),
                })
        }
        async fn obtener_cambios(&self, cursor: i64) -> Result<RespuestaSocial, ()> {
            self.cursores.lock().unwrap().push(cursor);
            let (estado, cuerpo) = self.cambios.lock().unwrap().pop_front().unwrap_or((
                StatusCode::OK,
                json!({"eventos":[],"nuevo_cursor":cursor,"hay_mas":false}),
            ));
            Ok(RespuestaSocial { estado, cuerpo })
        }
    }
    fn fake() -> Fake {
        Fake {
            envios: Mutex::new(vec![]),
            resultados: Mutex::new(VecDeque::new()),
            cambios: Mutex::new(VecDeque::new()),
            cursores: Mutex::new(vec![]),
        }
    }
    fn repo() -> (Mutex<RepositorioSqlite>, std::path::PathBuf) {
        let ruta = std::env::temp_dir().join(format!("motor-sync-{}.sqlite", Uuid::new_v4()));
        (
            Mutex::new(RepositorioSqlite::inicializar(ruta.clone()).unwrap()),
            ruta,
        )
    }

    #[tokio::test]
    async fn cola_reintenta_misma_clave_en_orden_y_continua_tras_rechazo() {
        let (repo, ruta) = repo();
        {
            let mut db = repo.lock().unwrap();
            db.encolar_gasto(
                "clave-1",
                "grupo",
                "Cena",
                "10",
                r#"{"concepto":"Cena","monto":"10"}"#,
            )
            .unwrap();
            db.encolar_gasto(
                "clave-2",
                "grupo",
                "Taxi",
                "5",
                r#"{"concepto":"Taxi","monto":"5"}"#,
            )
            .unwrap();
        }
        let fake = fake();
        fake.resultados.lock().unwrap().extend([
            Err(()),
            Ok(StatusCode::BAD_REQUEST),
            Ok(StatusCode::CREATED),
        ]);
        assert_eq!(
            sincronizar_con_cliente(&repo, &fake)
                .await
                .unwrap_err()
                .campo,
            "red"
        );
        assert_eq!(repo.lock().unwrap().listar_cola_gastos().unwrap().len(), 2);
        sincronizar_con_cliente(&repo, &fake).await.unwrap();
        let envios = fake.envios.lock().unwrap();
        assert_eq!(
            envios.iter().map(|e| e.1.as_str()).collect::<Vec<_>>(),
            vec!["clave-1", "clave-1", "clave-2"]
        );
        assert!(
            envios
                .iter()
                .all(|e| e.2.get("cuenta").is_none() && e.2.get("saldo").is_none())
        );
        let cola = repo.lock().unwrap().listar_cola_gastos().unwrap();
        assert_eq!(cola.len(), 1);
        assert_eq!(cola[0].estado, "rechazado");
        assert_eq!(cola[0].motivo.as_deref(), Some("rechazado"));
        drop(envios);
        drop(repo);
        let _ = std::fs::remove_file(ruta);
    }

    #[tokio::test]
    async fn cursor_pagina_ignora_eventos_desconocidos_y_401_detiene() {
        let (repo, ruta) = repo();
        let fake = fake();
        fake.cambios.lock().unwrap().extend([
            (
                StatusCode::OK,
                json!({"eventos":[{"tipo":"futuro","id":"x"}],"nuevo_cursor":1,"hay_mas":true}),
            ),
            (
                StatusCode::OK,
                json!({"eventos":[],"nuevo_cursor":1,"hay_mas":false}),
            ),
            (StatusCode::UNAUTHORIZED, json!({})),
        ]);
        sincronizar_con_cliente(&repo, &fake).await.unwrap();
        assert_eq!(*fake.cursores.lock().unwrap(), vec![0, 1]);
        assert_eq!(
            repo.lock()
                .unwrap()
                .obtener_cursor_sincronizacion()
                .unwrap(),
            1
        );
        let err = sincronizar_con_cliente(&repo, &fake).await.unwrap_err();
        assert_eq!(err.campo, "sesion");
        assert_eq!(err.mensaje, "Tu sesión venció, inicia sesión de nuevo.");
        assert_eq!(
            repo.lock()
                .unwrap()
                .obtener_cursor_sincronizacion()
                .unwrap(),
            1
        );
        drop(repo);
        let _ = std::fs::remove_file(ruta);
    }

    #[tokio::test]
    async fn cola_con_401_o_error_temporal_conserva_la_operacion() {
        let (repo, ruta) = repo();
        repo.lock()
            .unwrap()
            .encolar_gasto(
                "misma-clave",
                "grupo",
                "Cena",
                "10",
                r#"{"concepto":"Cena"}"#,
            )
            .unwrap();
        let fake = fake();
        fake.resultados.lock().unwrap().extend([
            Ok(StatusCode::UNAUTHORIZED),
            Ok(StatusCode::INTERNAL_SERVER_ERROR),
            Ok(StatusCode::OK),
        ]);
        assert_eq!(
            sincronizar_con_cliente(&repo, &fake)
                .await
                .unwrap_err()
                .campo,
            "sesion"
        );
        assert!(fake.cursores.lock().unwrap().is_empty());
        assert_eq!(
            sincronizar_con_cliente(&repo, &fake)
                .await
                .unwrap_err()
                .campo,
            "servidor"
        );
        assert_eq!(
            repo.lock().unwrap().listar_cola_gastos().unwrap()[0].estado,
            "pendiente"
        );
        sincronizar_con_cliente(&repo, &fake).await.unwrap();
        assert!(
            repo.lock()
                .unwrap()
                .listar_cola_gastos()
                .unwrap()
                .is_empty()
        );
        assert!(
            fake.envios
                .lock()
                .unwrap()
                .iter()
                .all(|e| e.1 == "misma-clave")
        );
        drop(repo);
        let _ = std::fs::remove_file(ruta);
    }

    #[tokio::test]
    async fn fallo_en_segunda_pagina_conserva_cursor_sin_marcar_exito() {
        let (repo, ruta) = repo();
        let fake = fake();
        fake.cambios.lock().unwrap().extend([
            (
                StatusCode::OK,
                json!({"eventos":[], "nuevo_cursor":1, "hay_mas":true}),
            ),
            (StatusCode::UNAUTHORIZED, json!({})),
        ]);
        assert_eq!(
            sincronizar_con_cliente(&repo, &fake)
                .await
                .unwrap_err()
                .campo,
            "sesion"
        );
        let db = repo.lock().unwrap();
        assert_eq!(db.obtener_cursor_sincronizacion().unwrap(), 1);
        assert_eq!(db.obtener_ultima_sincronizacion().unwrap(), None);
        drop(db);
        drop(repo);
        let _ = std::fs::remove_file(ruta);
    }

    #[tokio::test]
    async fn descarga_real_del_motor_aplica_pendientes_y_no_duplica_al_reiniciar() {
        let (repo, ruta) = repo();
        let cuenta = crate::domain::ejecutar_crear_cuenta(
            &mut *repo.lock().unwrap(),
            "Efectivo",
            "efectivo",
            "USD",
            "100.00",
            "test",
        )
        .unwrap();
        let fake = fake();
        let evento = json!({"id":"evento-1", "cursor":1, "tipo":"gasto_grupo", "grupo_id":"grupo-1", "grupo_nombre":"Viaje", "gasto_id":"hotel", "concepto":"Hotel", "fecha":"2026-11-20", "moneda":"USD", "monto_parte":"10.00", "incluye_centimos_redondeo":false});
        let pagina = json!({"eventos":[evento], "nuevo_cursor":1, "hay_mas":false});
        fake.cambios
            .lock()
            .unwrap()
            .extend([(StatusCode::OK, pagina.clone()), (StatusCode::OK, pagina)]);
        assert_eq!(
            sincronizar_con_cliente(&repo, &fake)
                .await
                .unwrap()
                .aplicados,
            0
        );
        {
            let mut db = repo.lock().unwrap();
            assert_eq!(db.listar_pendientes_aplicar().unwrap().len(), 1);
            db.elegir_cuenta_grupo_verificada("grupo-1", cuenta.id())
                .unwrap();
            db.reiniciar_cursor_sincronizacion().unwrap();
        }
        assert_eq!(
            sincronizar_con_cliente(&repo, &fake)
                .await
                .unwrap()
                .aplicados,
            0
        );
        let db = repo.lock().unwrap();
        let detalle = crate::domain::ejecutar_obtener_detalle_cuenta(&*db, cuenta.id()).unwrap();
        assert_eq!(detalle.saldo_actual, "90.00");
        assert_eq!(detalle.movimientos.len(), 1);
        assert!(db.listar_pendientes_aplicar().unwrap().is_empty());
        drop(db);
        drop(repo);
        let _ = std::fs::remove_file(ruta);
    }

    #[tokio::test]
    async fn rechaza_respuestas_invalidas_sin_avanzar_cursor() {
        let (repo, ruta) = repo();
        let fake = fake();
        fake.cambios.lock().unwrap().extend([
            (
                StatusCode::OK,
                json!({"eventos":[{"id":"sin-tipo"}],"nuevo_cursor":1,"hay_mas":false}),
            ),
            (
                StatusCode::OK,
                json!({"eventos":[],"nuevo_cursor":0,"hay_mas":true}),
            ),
            (
                StatusCode::OK,
                json!({"eventos":[{"tipo":"gasto_grupo"}],"nuevo_cursor":1,"hay_mas":false}),
            ),
            (
                StatusCode::BAD_REQUEST,
                json!({"mensaje":"cursor incorrecto"}),
            ),
        ]);
        for _ in 0..4 {
            assert_eq!(
                sincronizar_con_cliente(&repo, &fake)
                    .await
                    .unwrap_err()
                    .campo,
                "servidor"
            );
            assert_eq!(
                repo.lock()
                    .unwrap()
                    .obtener_cursor_sincronizacion()
                    .unwrap(),
                0
            );
        }
        drop(repo);
        let _ = std::fs::remove_file(ruta);
    }
}
