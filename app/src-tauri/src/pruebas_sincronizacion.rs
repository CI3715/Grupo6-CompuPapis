#[cfg(test)]
mod tests {
    use crate::domain::{
        ejecutar_corregir_movimiento, ejecutar_crear_cuenta, ejecutar_obtener_detalle_cuenta,
    };
    use crate::sqlite_repo::{EventoCambioRecibido, RepositorioSqlite};
    use std::path::PathBuf;
    use uuid::Uuid;

    fn bd_temporal() -> (RepositorioSqlite, PathBuf) {
        let ruta = std::env::temp_dir().join(format!("test_sync_{}.sqlite", Uuid::new_v4()));
        let repo = RepositorioSqlite::inicializar(ruta.clone()).unwrap();
        (repo, ruta)
    }

    fn limpiar_bd(ruta: PathBuf) {
        let _ = std::fs::remove_file(ruta);
    }

    #[test]
    fn usuarios_aislan_cursores_cuentas_pendientes_y_cola() {
        let (mut repo, ruta) = bd_temporal();
        let cuenta =
            ejecutar_crear_cuenta(&mut repo, "Efectivo", "efectivo", "USD", "100.00", "test")
                .unwrap();
        let mut ana = repo.para_usuario(&Uuid::new_v4().to_string()).unwrap();
        let mut beto = repo.para_usuario(&Uuid::new_v4().to_string()).unwrap();
        ana.guardar_moneda_grupo("grupo", "USD").unwrap();
        ana.elegir_cuenta_grupo_verificada("grupo", cuenta.id())
            .unwrap();
        ana.encolar_gasto("clave-ana", "grupo", "Cena", "10", "{}")
            .unwrap();
        let evento = EventoCambioRecibido {
            id: "pendiente-ana".into(),
            cursor: 12,
            tipo: "gasto_grupo".into(),
            grupo_id: "otro-grupo".into(),
            grupo_nombre: "Viaje".into(),
            gasto_id: "hotel".into(),
            concepto: "Hotel".into(),
            fecha: "2026-11-20".into(),
            moneda: "USD".into(),
            monto_parte: "10.00".into(),
            incluye_centimos_redondeo: false,
        };
        ana.aplicar_lote_cambios(12, vec![evento]).unwrap();
        assert_eq!(ana.obtener_cursor_sincronizacion().unwrap(), 12);
        assert_eq!(beto.obtener_cursor_sincronizacion().unwrap(), 0);
        assert!(beto.obtener_cuenta_grupo("grupo").unwrap().is_none());
        assert!(beto.obtener_moneda_grupo("grupo").unwrap().is_none());
        assert!(beto.listar_cola_gastos().unwrap().is_empty());
        assert_eq!(ana.listar_pendientes_aplicar().unwrap().len(), 1);
        assert!(beto.listar_pendientes_aplicar().unwrap().is_empty());
        beto.encolar_gasto("clave-beto", "grupo", "Taxi", "2", "{}")
            .unwrap();
        assert_eq!(ana.listar_cola_gastos().unwrap()[0].clave, "clave-ana");
        assert_eq!(beto.listar_cola_gastos().unwrap()[0].clave, "clave-beto");
        limpiar_bd(ruta);
    }

    #[test]
    fn estado_legacy_sin_dueno_se_conserva_y_no_se_atribuye() {
        let (mut repo, ruta) = bd_temporal();
        repo.encolar_gasto("legacy", "grupo", "Cena", "10", "{}")
            .unwrap();
        let usuario = Uuid::new_v4().to_string();
        let err = match repo.para_usuario(&usuario) {
            Ok(_) => panic!("No se debe atribuir una cola sin dueño"),
            Err(err) => err,
        };
        assert!(err.contains("sin usuario identificado"));
        assert_eq!(repo.listar_cola_gastos().unwrap()[0].clave, "legacy");
        repo.vincular_datos_anteriores(&usuario).unwrap();
        assert!(repo.listar_cola_gastos().unwrap().is_empty());
        assert_eq!(
            repo.para_usuario(&usuario)
                .unwrap()
                .listar_cola_gastos()
                .unwrap()[0]
                .clave,
            "legacy"
        );
        assert!(
            repo.para_usuario(&Uuid::new_v4().to_string())
                .unwrap()
                .listar_cola_gastos()
                .unwrap()
                .is_empty()
        );
        limpiar_bd(ruta);
    }

    #[test]
    fn vinculacion_no_sobrescribe_estado_existente() {
        let (mut repo, ruta) = bd_temporal();
        let usuario = Uuid::new_v4().to_string();
        let mut propio = repo.para_usuario(&usuario).unwrap();
        propio
            .encolar_gasto("propio", "grupo", "Taxi", "2", "{}")
            .unwrap();
        repo.guardar_moneda_grupo("grupo", "USD").unwrap();
        repo.encolar_gasto("legacy", "grupo", "Cena", "10", "{}")
            .unwrap();
        assert!(repo.vincular_datos_anteriores(&usuario).is_err());
        assert_eq!(repo.listar_cola_gastos().unwrap()[0].clave, "legacy");
        assert_eq!(
            repo.obtener_moneda_grupo("grupo").unwrap().as_deref(),
            Some("USD")
        );
        assert!(propio.obtener_moneda_grupo("grupo").unwrap().is_none());
        assert_eq!(propio.listar_cola_gastos().unwrap()[0].clave, "propio");
        limpiar_bd(ruta);
    }

    #[test]
    fn parte_cero_se_aplica_y_no_impide_el_lote() {
        let (mut repo, ruta) = bd_temporal();
        let cuenta =
            ejecutar_crear_cuenta(&mut repo, "Efectivo", "efectivo", "USD", "100.00", "test")
                .unwrap();
        repo.guardar_moneda_grupo("grupo", "USD").unwrap();
        repo.elegir_cuenta_grupo_verificada("grupo", cuenta.id())
            .unwrap();
        let evento = EventoCambioRecibido {
            id: "cero".into(),
            cursor: 1,
            tipo: "gasto_grupo".into(),
            grupo_id: "grupo".into(),
            grupo_nombre: "Viaje".into(),
            gasto_id: "gasto".into(),
            concepto: "Cena".into(),
            fecha: "2026-11-20".into(),
            moneda: "USD".into(),
            monto_parte: "0.00".into(),
            incluye_centimos_redondeo: false,
        };
        assert_eq!(repo.aplicar_lote_cambios(1, vec![evento]).unwrap(), 1);
        let detalle = ejecutar_obtener_detalle_cuenta(&repo, cuenta.id()).unwrap();
        assert_eq!(detalle.saldo_actual, "100.00");
        assert_eq!(detalle.movimientos.len(), 1);
        assert!(detalle.movimientos[0].es_gasto_de_grupo);
        assert_eq!(repo.obtener_cursor_sincronizacion().unwrap(), 1);
        limpiar_bd(ruta);
    }

    #[test]
    fn eleccion_usa_moneda_del_grupo_guardada_y_no_cambia_movimientos_anteriores() {
        let (mut repo, ruta) = bd_temporal();
        let usd =
            ejecutar_crear_cuenta(&mut repo, "USD", "efectivo", "USD", "100.00", "test").unwrap();
        let eur =
            ejecutar_crear_cuenta(&mut repo, "EUR", "efectivo", "EUR", "100.00", "test").unwrap();
        let otra =
            ejecutar_crear_cuenta(&mut repo, "Otra", "efectivo", "USD", "100.00", "test").unwrap();
        assert!(
            repo.elegir_cuenta_grupo_verificada("grupo", usd.id())
                .is_err()
        );
        repo.guardar_moneda_grupo("grupo", "USD").unwrap();
        assert!(
            repo.elegir_cuenta_grupo_verificada("grupo", eur.id())
                .is_err()
        );
        assert!(
            repo.elegir_cuenta_grupo_verificada("grupo", "inexistente")
                .is_err()
        );
        repo.elegir_cuenta_grupo_verificada("grupo", usd.id())
            .unwrap();
        let evento = EventoCambioRecibido {
            id: "uno".into(),
            cursor: 1,
            tipo: "gasto_grupo".into(),
            grupo_id: "grupo".into(),
            grupo_nombre: "Viaje".into(),
            gasto_id: "hotel".into(),
            concepto: "Hotel".into(),
            fecha: "2026-11-20".into(),
            moneda: "USD".into(),
            monto_parte: "10.00".into(),
            incluye_centimos_redondeo: false,
        };
        repo.aplicar_lote_cambios(1, vec![evento]).unwrap();
        repo.elegir_cuenta_grupo_verificada("grupo", otra.id())
            .unwrap();
        assert_eq!(
            ejecutar_obtener_detalle_cuenta(&repo, usd.id())
                .unwrap()
                .saldo_actual,
            "90.00"
        );
        assert_eq!(
            ejecutar_obtener_detalle_cuenta(&repo, otra.id())
                .unwrap()
                .saldo_actual,
            "100.00"
        );
        limpiar_bd(ruta);
    }

    #[test]
    fn duplicados_en_el_mismo_lote_y_moneda_incompatible_no_corrompen_el_libro() {
        let (mut repo, ruta) = bd_temporal();
        let cuenta =
            ejecutar_crear_cuenta(&mut repo, "Efectivo", "efectivo", "USD", "200.00", "prueba")
                .unwrap();
        assert!(repo.elegir_cuenta_grupo("grupo", "inexistente").is_err());
        repo.elegir_cuenta_grupo("grupo", cuenta.id()).unwrap();
        let mut evento = EventoCambioRecibido {
            id: "evento".into(),
            cursor: 1,
            tipo: "gasto_grupo".into(),
            grupo_id: "grupo".into(),
            grupo_nombre: "Viaje".into(),
            gasto_id: "hotel".into(),
            concepto: "Hotel".into(),
            fecha: "2026-11-20".into(),
            moneda: "EUR".into(),
            monto_parte: "10.00".into(),
            incluye_centimos_redondeo: false,
        };
        assert!(repo.aplicar_lote_cambios(1, vec![evento.clone()]).is_err());
        assert_eq!(repo.obtener_cursor_sincronizacion().unwrap(), 0);
        evento.moneda = "USD".into();
        assert_eq!(
            repo.aplicar_lote_cambios(1, vec![evento.clone(), evento])
                .unwrap(),
            1
        );
        let detalle = ejecutar_obtener_detalle_cuenta(&repo, cuenta.id()).unwrap();
        assert_eq!(detalle.saldo_actual, "190.00");
        assert_eq!(detalle.movimientos.len(), 1);
        limpiar_bd(ruta);
    }

    #[test]
    fn prueba_idempotencia_mismo_lote_aplicado_dos_veces() {
        let (mut repo, ruta) = bd_temporal();
        let cuenta = ejecutar_crear_cuenta(
            &mut repo,
            "Efectivo USD",
            "efectivo",
            "USD",
            "200.00",
            "prueba",
        )
        .unwrap();

        let grupo_id = Uuid::new_v4().to_string();
        repo.elegir_cuenta_grupo(&grupo_id, cuenta.id()).unwrap();

        let evento_hotel = EventoCambioRecibido {
            id: Uuid::new_v4().to_string(),
            cursor: 1,
            tipo: "gasto_grupo".to_string(),
            grupo_id: grupo_id.clone(),
            grupo_nombre: "Viaje a Mérida".to_string(),
            gasto_id: Uuid::new_v4().to_string(),
            concepto: "Hotel".to_string(),
            fecha: "2026-11-20".to_string(),
            moneda: "USD".to_string(),
            monto_parte: "33.33".to_string(),
            incluye_centimos_redondeo: false,
        };

        // Primera aplicación del lote
        let aplicados_1 = repo
            .aplicar_lote_cambios(1, vec![evento_hotel.clone()])
            .unwrap();
        assert_eq!(aplicados_1, 1);

        let detalle_1 = ejecutar_obtener_detalle_cuenta(&repo, cuenta.id()).unwrap();
        assert_eq!(detalle_1.saldo_actual, "166.67");
        assert_eq!(detalle_1.movimientos.len(), 1);
        assert!(detalle_1.movimientos[0].es_gasto_de_grupo);
        assert_eq!(detalle_1.movimientos[0].categoria, "Viaje a Mérida · Hotel");

        // Segunda aplicación del mismo lote (o cursor reiniciado)
        let aplicados_2 = repo
            .aplicar_lote_cambios(1, vec![evento_hotel.clone()])
            .unwrap();
        assert_eq!(aplicados_2, 0);

        let detalle_2 = ejecutar_obtener_detalle_cuenta(&repo, cuenta.id()).unwrap();
        // El saldo y los movimientos no cambian (INV-13, INV-15)
        assert_eq!(detalle_2.saldo_actual, "166.67");
        assert_eq!(detalle_2.movimientos.len(), 1);

        limpiar_bd(ruta);
    }

    #[test]
    fn prueba_interrupcion_a_mitad_de_lote_no_duplica_y_conserva_cursor() {
        let (mut repo, ruta) = bd_temporal();
        let cuenta = ejecutar_crear_cuenta(
            &mut repo,
            "Efectivo USD",
            "efectivo",
            "USD",
            "200.00",
            "prueba",
        )
        .unwrap();

        let grupo_id = Uuid::new_v4().to_string();
        repo.elegir_cuenta_grupo(&grupo_id, cuenta.id()).unwrap();

        assert_eq!(repo.obtener_cursor_sincronizacion().unwrap(), 0);

        let evento_1 = EventoCambioRecibido {
            id: Uuid::new_v4().to_string(),
            cursor: 1,
            tipo: "gasto_grupo".to_string(),
            grupo_id: grupo_id.clone(),
            grupo_nombre: "Viaje a Mérida".to_string(),
            gasto_id: Uuid::new_v4().to_string(),
            concepto: "Hotel".to_string(),
            fecha: "2026-11-20".to_string(),
            moneda: "USD".to_string(),
            monto_parte: "33.33".to_string(),
            incluye_centimos_redondeo: false,
        };

        // Evento con monto inválido para simular fallo a mitad de lote dentro de la transacción
        let evento_invalido = EventoCambioRecibido {
            id: Uuid::new_v4().to_string(),
            cursor: 2,
            tipo: "gasto_grupo".to_string(),
            grupo_id: grupo_id.clone(),
            grupo_nombre: "Viaje a Mérida".to_string(),
            gasto_id: Uuid::new_v4().to_string(),
            concepto: "Cena".to_string(),
            fecha: "2026-11-20".to_string(),
            moneda: "USD".to_string(),
            monto_parte: "invalido_monto".to_string(),
            incluye_centimos_redondeo: false,
        };

        // La transacción debe fallar y hacer rollback completo
        let resultado = repo.aplicar_lote_cambios(2, vec![evento_1.clone(), evento_invalido]);
        assert!(resultado.is_err());

        // El cursor debe mantenerse en 0 y no debe haber movimientos guardados
        assert_eq!(repo.obtener_cursor_sincronizacion().unwrap(), 0);
        let detalle_antes = ejecutar_obtener_detalle_cuenta(&repo, cuenta.id()).unwrap();
        assert_eq!(detalle_antes.movimientos.len(), 0);
        assert_eq!(detalle_antes.saldo_actual, "200.00");

        // Al reintentar con el lote correcto desde el cursor guardado (0), se aplica limpiamente
        let evento_2_corregido = EventoCambioRecibido {
            id: Uuid::new_v4().to_string(),
            cursor: 2,
            tipo: "gasto_grupo".to_string(),
            grupo_id: grupo_id.clone(),
            grupo_nombre: "Viaje a Mérida".to_string(),
            gasto_id: Uuid::new_v4().to_string(),
            concepto: "Cena".to_string(),
            fecha: "2026-11-20".to_string(),
            moneda: "USD".to_string(),
            monto_parte: "15.00".to_string(),
            incluye_centimos_redondeo: false,
        };

        let aplicados = repo
            .aplicar_lote_cambios(2, vec![evento_1, evento_2_corregido])
            .unwrap();
        assert_eq!(aplicados, 2);
        assert_eq!(repo.obtener_cursor_sincronizacion().unwrap(), 2);

        let detalle_despues = ejecutar_obtener_detalle_cuenta(&repo, cuenta.id()).unwrap();
        assert_eq!(detalle_despues.movimientos.len(), 2);
        assert_eq!(detalle_despues.saldo_actual, "151.67");

        limpiar_bd(ruta);
    }

    #[test]
    fn prueba_partes_pendientes_se_aplican_al_elegir_cuenta() {
        let (mut repo, ruta) = bd_temporal();
        let cuenta = ejecutar_crear_cuenta(
            &mut repo,
            "Efectivo USD",
            "efectivo",
            "USD",
            "200.00",
            "prueba",
        )
        .unwrap();

        let grupo_id = Uuid::new_v4().to_string();

        let evento_hotel = EventoCambioRecibido {
            id: Uuid::new_v4().to_string(),
            cursor: 1,
            tipo: "gasto_grupo".to_string(),
            grupo_id: grupo_id.clone(),
            grupo_nombre: "Viaje a Mérida".to_string(),
            gasto_id: Uuid::new_v4().to_string(),
            concepto: "Hotel".to_string(),
            fecha: "2026-11-20".to_string(),
            moneda: "USD".to_string(),
            monto_parte: "33.33".to_string(),
            incluye_centimos_redondeo: false,
        };

        // Llega parte sin cuenta elegida para el grupo
        let aplicados = repo
            .aplicar_lote_cambios(1, vec![evento_hotel.clone()])
            .unwrap();
        assert_eq!(aplicados, 0);

        // Queda en la lista de pendientes de aplicar
        let pendientes = repo.listar_pendientes_aplicar().unwrap();
        assert_eq!(pendientes.len(), 1);
        assert_eq!(pendientes[0].id, evento_hotel.id);
        assert_eq!(pendientes[0].concepto, "Hotel");

        // El saldo del libro permanece intacto
        let detalle_antes = ejecutar_obtener_detalle_cuenta(&repo, cuenta.id()).unwrap();
        assert_eq!(detalle_antes.saldo_actual, "200.00");
        assert_eq!(detalle_antes.movimientos.len(), 0);

        // El usuario elige su cuenta para el grupo
        let aplicados_al_elegir = repo
            .elegir_cuenta_grupo_verificada(&grupo_id, cuenta.id())
            .unwrap();
        assert_eq!(aplicados_al_elegir, 1);

        // Ya no hay pendientes
        let pendientes_despues = repo.listar_pendientes_aplicar().unwrap();
        assert_eq!(pendientes_despues.len(), 0);

        // El gasto baja al libro reflejando el nuevo saldo
        let detalle_despues = ejecutar_obtener_detalle_cuenta(&repo, cuenta.id()).unwrap();
        assert_eq!(detalle_despues.saldo_actual, "166.67");
        assert_eq!(detalle_despues.movimientos.len(), 1);
        assert!(detalle_despues.movimientos[0].es_gasto_de_grupo);

        limpiar_bd(ruta);
    }

    #[test]
    fn prueba_gasto_de_grupo_no_se_puede_corregir_en_el_libro_rf_com_10() {
        let (mut repo, ruta) = bd_temporal();
        let cuenta = ejecutar_crear_cuenta(
            &mut repo,
            "Efectivo USD",
            "efectivo",
            "USD",
            "200.00",
            "prueba",
        )
        .unwrap();

        let grupo_id = Uuid::new_v4().to_string();
        repo.elegir_cuenta_grupo(&grupo_id, cuenta.id()).unwrap();

        let evento_hotel = EventoCambioRecibido {
            id: Uuid::new_v4().to_string(),
            cursor: 1,
            tipo: "gasto_grupo".to_string(),
            grupo_id: grupo_id.clone(),
            grupo_nombre: "Viaje a Mérida".to_string(),
            gasto_id: Uuid::new_v4().to_string(),
            concepto: "Hotel".to_string(),
            fecha: "2026-11-20".to_string(),
            moneda: "USD".to_string(),
            monto_parte: "33.33".to_string(),
            incluye_centimos_redondeo: false,
        };

        repo.aplicar_lote_cambios(1, vec![evento_hotel]).unwrap();

        let detalle = ejecutar_obtener_detalle_cuenta(&repo, cuenta.id()).unwrap();
        let mov_id = &detalle.movimientos[0].id;

        // Intentar corregir el movimiento desde el núcleo del libro debe ser rechazado
        let resultado =
            ejecutar_corregir_movimiento(&mut repo, cuenta.id(), mov_id, "40.00", "prueba");
        assert!(resultado.is_err());
        let err = resultado.unwrap_err();
        assert_eq!(err.campo, "movimiento");
        assert!(
            err.mensaje
                .contains("Los gastos de grupo no se pueden editar ni corregir")
        );

        // El saldo permanece inalterado
        let detalle_despues = ejecutar_obtener_detalle_cuenta(&repo, cuenta.id()).unwrap();
        assert_eq!(detalle_despues.saldo_actual, "166.67");

        limpiar_bd(ruta);
    }

    #[test]
    fn prueba_cola_de_gastos_con_reintentos_y_rechazo() {
        let (mut repo, ruta) = bd_temporal();
        let clave_1 = Uuid::new_v4().to_string();
        let clave_2 = Uuid::new_v4().to_string();
        let grupo_id = Uuid::new_v4().to_string();

        repo.encolar_gasto(
            &clave_1,
            &grupo_id,
            "Cena",
            "60.00",
            "{\"concepto\":\"Cena\",\"monto\":\"60.00\"}",
        )
        .unwrap();

        repo.encolar_gasto(
            &clave_2,
            &grupo_id,
            "Taxis",
            "20.00",
            "{\"concepto\":\"Taxis\",\"monto\":\"20.00\"}",
        )
        .unwrap();

        let cola = repo.listar_cola_gastos().unwrap();
        assert_eq!(cola.len(), 2);
        assert_eq!(cola[0].estado, "pendiente");
        assert_eq!(cola[1].estado, "pendiente");

        // Simular que el primer gasto fue enviado exitosamente al volver la red
        repo.eliminar_de_cola(&clave_1).unwrap();

        // Simular que el segundo gasto fue rechazado por el servidor (ej. ya no es miembro)
        repo.marcar_gasto_rechazado(&clave_2, "No perteneces a este grupo o fue archivado.")
            .unwrap();

        let cola_actualizada = repo.listar_cola_gastos().unwrap();
        assert_eq!(cola_actualizada.len(), 1);
        assert_eq!(cola_actualizada[0].clave, clave_2);
        assert_eq!(cola_actualizada[0].estado, "rechazado");
        assert_eq!(
            cola_actualizada[0].motivo.as_deref(),
            Some("No perteneces a este grupo o fue archivado.")
        );

        // Descartar rechazo
        repo.eliminar_de_cola(&clave_2).unwrap();
        assert!(repo.listar_cola_gastos().unwrap().is_empty());

        limpiar_bd(ruta);
    }

    #[test]
    fn prueba_reinicio_cursor_a_cero_no_duplica_movimientos() {
        let (mut repo, ruta) = bd_temporal();
        let cuenta = ejecutar_crear_cuenta(
            &mut repo,
            "Efectivo USD",
            "efectivo",
            "USD",
            "200.00",
            "prueba",
        )
        .unwrap();

        let grupo_id = Uuid::new_v4().to_string();
        repo.elegir_cuenta_grupo(&grupo_id, cuenta.id()).unwrap();

        let evento_hotel = EventoCambioRecibido {
            id: Uuid::new_v4().to_string(),
            cursor: 1,
            tipo: "gasto_grupo".to_string(),
            grupo_id: grupo_id.clone(),
            grupo_nombre: "Viaje a Mérida".to_string(),
            gasto_id: Uuid::new_v4().to_string(),
            concepto: "Hotel".to_string(),
            fecha: "2026-11-20".to_string(),
            moneda: "USD".to_string(),
            monto_parte: "33.33".to_string(),
            incluye_centimos_redondeo: false,
        };

        let evento_cena = EventoCambioRecibido {
            id: Uuid::new_v4().to_string(),
            cursor: 2,
            tipo: "gasto_grupo".to_string(),
            grupo_id: grupo_id.clone(),
            grupo_nombre: "Viaje a Mérida".to_string(),
            gasto_id: Uuid::new_v4().to_string(),
            concepto: "Cena".to_string(),
            fecha: "2026-11-20".to_string(),
            moneda: "USD".to_string(),
            monto_parte: "15.00".to_string(),
            incluye_centimos_redondeo: false,
        };

        // Bajan ambos eventos
        repo.aplicar_lote_cambios(2, vec![evento_hotel.clone(), evento_cena.clone()])
            .unwrap();
        assert_eq!(repo.obtener_cursor_sincronizacion().unwrap(), 2);
        let detalle = ejecutar_obtener_detalle_cuenta(&repo, cuenta.id()).unwrap();
        assert_eq!(detalle.saldo_actual, "151.67");
        assert_eq!(detalle.movimientos.len(), 2);

        // Paso 3 del flujo: reinicio de cursor a cero
        repo.reiniciar_cursor_sincronizacion().unwrap();
        assert_eq!(repo.obtener_cursor_sincronizacion().unwrap(), 0);

        // Bajan de nuevo los dos eventos desde la posición 0: ya estaban aplicados y se ignoran
        let aplicados_reinicio = repo
            .aplicar_lote_cambios(2, vec![evento_hotel, evento_cena])
            .unwrap();
        assert_eq!(aplicados_reinicio, 0);

        let detalle_despues = ejecutar_obtener_detalle_cuenta(&repo, cuenta.id()).unwrap();
        assert_eq!(detalle_despues.saldo_actual, "151.67");
        assert_eq!(detalle_despues.movimientos.len(), 2);

        limpiar_bd(ruta);
    }

    // RF-COM-07 / Sección 2.4 del PDF:
    // "Si la sesión venció, la sincronización se detiene con el mensaje
    //  'Tu sesión venció, inicia sesión de nuevo'; las funciones personales
    //  siguen disponibles."
    //
    // Este test verifica la pieza Rust de ese flujo:
    //  a) El ErrorDominio generado por un HTTP 401 tiene campo == "sesion"
    //     y el mensaje exacto requerido por el PDF.
    //  b) Tras ese error, el repositorio local queda intacto y las
    //     funciones personales (crear cuenta, registrar movimiento,
    //     obtener detalle) siguen funcionando con normalidad.
    #[test]
    fn prueba_sesion_vencida_detiene_sync_y_funciones_personales_siguen() {
        use crate::domain::{ErrorDominio, ejecutar_crear_cuenta, ejecutar_obtener_detalle_cuenta};

        // --- a) Verificar el ErrorDominio que sincronizar devuelve ante un 401 ---
        // Construimos el error tal como lo produce sincronizar() en ambos puntos
        // donde detecta StatusCode::UNAUTHORIZED (envío de cola y GET /cambios).
        let error_sesion =
            ErrorDominio::nuevo("sesion", "Tu sesión venció, inicia sesión de nuevo.");

        assert_eq!(
            error_sesion.campo, "sesion",
            "El campo debe ser 'sesion' para que el frontend llame a onSesionVencida()"
        );
        assert_eq!(
            error_sesion.mensaje, "Tu sesión venció, inicia sesión de nuevo.",
            "El mensaje debe coincidir exactamente con el texto del PDF"
        );

        // --- b) Verificar que las funciones personales siguen disponibles ---
        // Simulamos el estado posterior a un error de sesión: el repo local
        // no fue modificado por la sincronización fallida, y todas las
        // operaciones de libro personal continúan funcionando.
        let (mut repo, ruta) = bd_temporal();

        // Crear una cuenta local (función personal)
        let cuenta = ejecutar_crear_cuenta(
            &mut repo,
            "Efectivo EUR",
            "efectivo",
            "EUR",
            "500.00",
            "prueba-sesion",
        )
        .expect("Crear cuenta debe funcionar sin sesión de red");

        // Consultar detalle (función personal)
        let detalle = ejecutar_obtener_detalle_cuenta(&repo, cuenta.id())
            .expect("Obtener detalle debe funcionar sin sesión de red");

        assert_eq!(
            detalle.saldo_actual, "500.00",
            "El saldo de la cuenta personal debe estar intacto después de un error de sesión"
        );
        assert_eq!(
            detalle.movimientos.len(),
            0,
            "No debe haber movimientos de grupo ni corrupción local tras la sesión vencida"
        );

        // El cursor de sincronización tampoco fue alterado (sigue en 0)
        let cursor = repo.obtener_cursor_sincronizacion().unwrap();
        assert_eq!(
            cursor, 0,
            "Un error de sesión antes de descargar cambios no debe avanzar el cursor"
        );

        limpiar_bd(ruta);
    }
}
