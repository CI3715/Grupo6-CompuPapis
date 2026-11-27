use super::*;

#[test]
fn payload_no_incluye_datos_del_libro_personal() {
    let datos = json!({"concepto": "Cena", "monto": "10.00", "cuenta": "privada", "saldo": "100", "movimientos": [], "tasas": []});
    let payload = datos_gasto_grupo(&datos);
    for campo in ["cuenta", "saldo", "movimientos", "tasas"] {
        assert!(payload.get(campo).is_none());
    }
    assert_eq!(payload["concepto"], "Cena");
}

#[tokio::test]
async fn comandos_y_adaptador_http_cubren_operaciones_y_errores() {
    use crate::{autenticacion::SesionLocal, sqlite_repo::RepositorioSqlite};
    use axum::{
        Json, Router, extract::State as AxumState, http::StatusCode, response::IntoResponse,
        routing::any,
    };
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicU16, Ordering},
    };
    use tauri::Manager;

    async fn respuesta(AxumState(codigo): AxumState<Arc<AtomicU16>>) -> impl IntoResponse {
        let estado = StatusCode::from_u16(codigo.load(Ordering::SeqCst)).unwrap();
        (
            estado,
            [(axum::http::header::CONNECTION, "close")],
            Json(
                json!({"id":"00000000-0000-0000-0000-000000000001", "moneda":"USD", "grupos":[{"id":"00000000-0000-0000-0000-000000000001","moneda":"USD"}], "mensaje":"rechazo"}),
            ),
        )
    }
    let modo = Arc::new(AtomicU16::new(200));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let direccion = listener.local_addr().unwrap();
    let router = Router::new()
        .fallback(any(respuesta))
        .with_state(modo.clone());
    let servidor = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let ruta = std::env::temp_dir().join(format!("social-test-{}.sqlite", Uuid::new_v4()));
    let repo = RepositorioSqlite::inicializar(ruta.clone()).unwrap();
    let usuario = Uuid::new_v4().to_string();
    let dispositivo_id = repo.dispositivo_id().unwrap();
    let app = tauri::test::mock_app();
    app.manage(AppState {
        repo: Mutex::new(repo),
        server_url: format!("http://{direccion}"),
        ruta_token_fallback: ruta.clone(),
        app_dir: std::env::temp_dir(),
        dispositivo_id,
    });
    app.manage(PingState {
        http_client: reqwest::Client::new(),
    });
    app.manage(AuthState {
        sesion: Mutex::new(Some(SesionLocal {
            token: "token".into(),
            usuario_id: usuario.clone(),
        })),
    });
    let id = "00000000-0000-0000-0000-000000000001";
    let datos = json!({"id":id,"concepto":"Cena","monto":"10.00","fecha":"2026-11-20","pagado_por":id,"regla":"iguales","participantes":[],"usuario":"ana","nombre_usuario":"ana","nombre":"Grupo","moneda":"USD","aceptar":true,"usuario_id":id});
    for accion in [
        "buscar_usuario",
        "listar_amistades",
        "invitar_amistad",
        "responder_amistad",
        "eliminar_amistad",
        "listar_grupos",
        "crear_grupo",
        "detalle_grupo",
        "invitar_grupo",
        "responder_grupo",
        "registrar_gasto",
    ] {
        assert!(
            operacion_social(
                accion.into(),
                datos.clone(),
                app.state(),
                app.state(),
                app.state()
            )
            .await
            .is_ok(),
            "{accion}"
        );
    }
    let repo = app
        .state::<AppState>()
        .repo
        .lock()
        .unwrap()
        .para_usuario(&usuario)
        .unwrap();
    assert_eq!(
        repo.obtener_moneda_grupo(id).unwrap().as_deref(),
        Some("USD")
    );
    assert!(
        operacion_social(
            "inexistente".into(),
            datos.clone(),
            app.state(),
            app.state(),
            app.state()
        )
        .await
        .is_err()
    );
    assert!(
        operacion_social(
            "detalle_grupo".into(),
            json!({}),
            app.state(),
            app.state(),
            app.state()
        )
        .await
        .is_err()
    );
    modo.store(401, Ordering::SeqCst);
    assert!(
        operacion_social(
            "listar_grupos".into(),
            datos.clone(),
            app.state(),
            app.state(),
            app.state()
        )
        .await
        .is_err()
    );
    assert!(
        operacion_social(
            "registrar_gasto".into(),
            datos.clone(),
            app.state(),
            app.state(),
            app.state()
        )
        .await
        .is_err()
    );
    for item in repo.listar_cola_gastos().unwrap() {
        repo.clone().eliminar_de_cola(&item.clave).unwrap();
    }
    modo.store(400, Ordering::SeqCst);
    assert!(
        operacion_social(
            "registrar_gasto".into(),
            datos.clone(),
            app.state(),
            app.state(),
            app.state()
        )
        .await
        .is_err()
    );
    assert_eq!(repo.listar_cola_gastos().unwrap()[0].estado, "rechazado");
    modo.store(500, Ordering::SeqCst);
    assert_eq!(
        operacion_social(
            "registrar_gasto".into(),
            datos.clone(),
            app.state(),
            app.state(),
            app.state()
        )
        .await
        .unwrap()["pendiente"],
        true
    );
    assert_eq!(
        operacion_social(
            "registrar_gasto".into(),
            datos.clone(),
            app.state(),
            app.state(),
            app.state()
        )
        .await
        .unwrap()["pendiente"],
        true
    );
    for item in repo.listar_cola_gastos().unwrap() {
        repo.clone().eliminar_de_cola(&item.clave).unwrap();
    }
    modo.store(204, Ordering::SeqCst);
    assert_eq!(
        operacion_social(
            "eliminar_amistad".into(),
            datos.clone(),
            app.state(),
            app.state(),
            app.state()
        )
        .await
        .unwrap(),
        Value::Null
    );
    modo.store(200, Ordering::SeqCst);
    let ping = app.state::<PingState>();
    let estado = app.state::<AppState>();
    let http = ClienteSocialHttp {
        cliente: &ping.http_client,
        base: &estado.server_url,
        token: "token",
    };
    assert_eq!(
        http.obtener_cambios(0).await.unwrap().estado,
        StatusCode::OK
    );
    assert_eq!(
        http.enviar_gasto(id, "clave", &datos).await.unwrap().estado,
        StatusCode::OK
    );
    servidor.abort();
    let _ = servidor.await;
    assert_eq!(
        operacion_social(
            "registrar_gasto".into(),
            datos,
            app.state(),
            app.state(),
            app.state()
        )
        .await
        .unwrap()["pendiente"],
        true
    );
    drop(app);
    let _ = std::fs::remove_file(ruta);
}
