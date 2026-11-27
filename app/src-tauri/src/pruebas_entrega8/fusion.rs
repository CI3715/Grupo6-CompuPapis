use super::*;

// =========================================================
// 2. PRUEBAS DE FUSIÓN, IDEMPOTENCIA, CONFLICTO Y GRUPOS
// =========================================================

#[test]
fn gastos_grupo_en_distintas_cuentas() {
    let dir = DirTemp::nuevo("gastos_grupo");
    let mut a = bd_en(dir.path(), "a");
    let mut b = bd_en(dir.path(), "b");

    let disp = a.dispositivo_id().unwrap();

    let cuenta1 = ejecutar_crear_cuenta(
        &mut a, "Efectivo", "efectivo", "USD", "200.00", &disp
    ).unwrap();

    let cuenta2 = ejecutar_crear_cuenta(
        &mut a, "Ahorros", "efectivo", "USD", "200.00", &disp
    ).unwrap();

    let c1 = cuenta1.id();
    let c2 = cuenta2.id();

    let inicial = a.obtener_eventos().unwrap();
    b.fusionar_eventos(&inicial).unwrap();

    let grupo = Uuid::new_v4().to_string();

    // Cada dispositivo elige una cuenta distinta
    a.elegir_cuenta_grupo(&grupo, c1).unwrap();
    b.elegir_cuenta_grupo(&grupo, c2).unwrap();

    let mut cambio = EventoCambioRecibido {
        id: Uuid::new_v4().to_string(),
        cursor: 1,
        tipo: "gasto_grupo".into(),
        grupo_id: grupo,
        grupo_nombre: "Viaje".into(),
        gasto_id: Uuid::new_v4().to_string(),
        concepto: "Hotel".into(),
        fecha: "2026-11-20".into(),
        moneda: "USD".into(),
        monto_parte: "33.33".into(),
        incluye_centimos_redondeo: false,
    };

    // Primero pasamos el gasto de un libro al otro
    assert_eq!(a.aplicar_lote_cambios(1, vec![cambio.clone()]).unwrap(), 1);

    let eventos = a.obtener_eventos().unwrap();
    let res = b.fusionar_eventos(&eventos).unwrap();

    assert_eq!(res.nuevos, 1);
    assert!(res.rechazados.is_empty());

    // Bajamos el mismo gasto otra vez
    assert_eq!(b.aplicar_lote_cambios(1, vec![cambio.clone()]).unwrap(), 0);
    assert_eq!(b.obtener_eventos().unwrap(), eventos);

    let det1 = ejecutar_obtener_detalle_cuenta(&b, c1).unwrap();
    let det2 = ejecutar_obtener_detalle_cuenta(&b, c2).unwrap();

    assert_eq!(det1.saldo_actual, "166.67");
    assert_eq!(det1.movimientos.len(), 1);
    assert_eq!(det2.saldo_actual, "200.00");
    assert!(det2.movimientos.is_empty());

    // Ahora los 2 reciben otro gasto por separado
    cambio.id = Uuid::new_v4().to_string();
    cambio.gasto_id = Uuid::new_v4().to_string();
    cambio.cursor = 2;

    assert_eq!(a.aplicar_lote_cambios(2, vec![cambio.clone()]).unwrap(), 1);
    assert_eq!(b.aplicar_lote_cambios(2, vec![cambio]).unwrap(), 1);

    let ev_a = a.obtener_eventos().unwrap();
    let ev_b = b.obtener_eventos().unwrap();

    assert_eq!(ev_a.len(), 4);
    assert_eq!(ev_b.len(), 4);
    assert_eq!(ev_a[3].id, ev_b[3].id);

    // Vemos que pasa al juntar los libros
    let res_a = a.fusionar_eventos(&ev_b).unwrap();
    let res_b = b.fusionar_eventos(&ev_a).unwrap();

    assert_eq!(res_a.nuevos, 0);
    assert_eq!(res_b.nuevos, 0);

    assert_eq!(res_a.rechazados.len(), 1);
    assert_eq!(res_b.rechazados.len(), 1);

    assert_eq!(res_a.rechazados[0].evento_id, ev_a[3].id);
    assert_eq!(res_b.rechazados[0].evento_id, ev_b[3].id);

    assert_eq!(a.obtener_eventos().unwrap(), ev_a);
    assert_eq!(b.obtener_eventos().unwrap(), ev_b);
}

#[test]
fn importacion_revierte_todo_si_sqlite_falla_a_mitad_del_lote() {
    let dir = DirTemp::nuevo("rollback_importacion");
    let mut origen = bd_en(dir.path(), "origen");
    crear_cuenta_y_movimientos_base(&mut origen);
    let (archivo, _, _) =
        exportar_bytes(&origen, false, None, None, ParametrosKdf::para_pruebas()).unwrap();
    let mut destino = bd_en(dir.path(), "destino");
    let conn = rusqlite::Connection::open(dir.path().join("destino.sqlite")).unwrap();
    conn.execute_batch("CREATE TRIGGER fallo_importacion BEFORE INSERT ON eventos_libro WHEN (SELECT count(*) FROM eventos_libro) = 1 BEGIN SELECT RAISE(ABORT, 'fallo simulado'); END;").unwrap();
    assert!(importar_bytes(&mut destino, &archivo, None).is_err());
    assert!(destino.obtener_eventos().unwrap().is_empty());
    conn.execute_batch("DROP TRIGGER fallo_importacion;")
        .unwrap();
    assert_eq!(
        importar_bytes(&mut destino, &archivo, None).unwrap().nuevos,
        3
    );
}

#[test]
fn importar_restaura_tasas_y_saldos_en_instalacion_nueva() {
    use crate::domain::ejecutar_listar_tasas;
    use crate::tasas_monedas::{Reloj, ejecutar_registrar_tasa};
    struct RelojPrueba;
    impl Reloj for RelojPrueba {
        fn ahora(&self) -> chrono::DateTime<chrono::Utc> {
            chrono::Utc::now()
        }
    }
    let dir = DirTemp::nuevo("restaurar_tasas");
    let mut origen = bd_en(dir.path(), "origen");
    let cuenta = crear_cuenta_y_movimientos_base(&mut origen);
    ejecutar_registrar_tasa(
        &mut origen,
        "USD",
        "VES",
        "36.50",
        "2026-01-01",
        "BCV",
        "prueba",
        &RelojPrueba,
        &crate::GeneradorUuid,
    )
    .unwrap();
    let tasas = ejecutar_listar_tasas(&origen).unwrap();
    let (archivo, cantidad, _) =
        exportar_bytes(&origen, false, None, None, ParametrosKdf::para_pruebas()).unwrap();
    let mut destino = bd_en(dir.path(), "destino");
    assert_eq!(
        importar_bytes(&mut destino, &archivo, None).unwrap().nuevos,
        cantidad
    );
    assert_eq!(ejecutar_listar_tasas(&destino).unwrap(), tasas);
    assert_eq!(
        ejecutar_obtener_detalle_cuenta(&destino, &cuenta)
            .unwrap()
            .saldo_actual,
        "151.67"
    );
    assert_eq!(
        importar_bytes(&mut destino, &archivo, None).unwrap().nuevos,
        0
    );
}

#[test]
fn importaciones_concurrentes_del_mismo_archivo_no_duplican() {
    let dir = DirTemp::nuevo("importacion_concurrente");
    let mut origen = bd_en(dir.path(), "origen");
    let cuenta = crear_cuenta_y_movimientos_base(&mut origen);
    let (archivo, _, _) =
        exportar_bytes(&origen, false, None, None, ParametrosKdf::para_pruebas()).unwrap();
    let repo1 = bd_en(dir.path(), "destino");
    let repo2 = bd_en(dir.path(), "destino");
    let barrera = std::sync::Arc::new(std::sync::Barrier::new(2));
    let hilos: Vec<_> = [repo1, repo2]
        .into_iter()
        .map(|mut repo| {
            let archivo = archivo.clone();
            let barrera = barrera.clone();
            std::thread::spawn(move || {
                barrera.wait();
                importar_bytes(&mut repo, &archivo, None).unwrap()
            })
        })
        .collect();
    let resultados: Vec<_> = hilos.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(resultados.iter().map(|r| r.nuevos).sum::<usize>(), 3);
    assert_eq!(resultados.iter().map(|r| r.ya_existian).sum::<usize>(), 3);
    let destino = bd_en(dir.path(), "destino");
    assert_eq!(destino.obtener_eventos().unwrap().len(), 3);
    assert_eq!(
        ejecutar_obtener_detalle_cuenta(&destino, &cuenta)
            .unwrap()
            .saldo_actual,
        "151.67"
    );
}

#[test]
fn fusion_idempotente_archivo_antiguo_no_borra_y_conflicto_se_rechaza() {
    let dir = DirTemp::nuevo("fusion");
    let mut repo_a = bd_en(dir.path(), "a");
    let cid = crear_cuenta_y_movimientos_base(&mut repo_a);

    // Exportación inicial (3 eventos)
    let (respaldo_antiguo, total_exp, _) =
        exportar_bytes(&repo_a, false, None, None, ParametrosKdf::para_pruebas()).unwrap();
    assert_eq!(total_exp, 3);

    // Exportar no modifica el libro (RF-INT-01)
    assert_eq!(repo_a.obtener_eventos().unwrap().len(), 3);

    // Importar en instalación nueva (restauración completa)
    let mut repo_b = bd_en(dir.path(), "b");
    let r1 = importar_bytes(&mut repo_b, &respaldo_antiguo, None).unwrap();
    assert_eq!(r1.nuevos, 3);
    assert_eq!(r1.ya_existian, 0);
    assert!(r1.rechazados.is_empty());

    // Importar el mismo archivo dos veces -> 0 nuevos, 3 ya existían (RF-INT-02, ESC-04)
    let r2 = importar_bytes(&mut repo_b, &respaldo_antiguo, None).unwrap();
    assert_eq!(r2.nuevos, 0);
    assert_eq!(r2.ya_existian, 3);
    assert!(r2.rechazados.is_empty());

    // Agregar un movimiento nuevo en B y volver a importar el archivo ANTIGUO:
    // nunca borra eventos locales (INV-14)
    let disp_b = repo_b.dispositivo_id().unwrap();
    ejecutar_registrar_movimiento(
        &mut repo_b,
        RegistrarMovimiento {
            cuenta_id: &cid,
            tipo_str: "ingreso",
            monto_str: "50.00",
            fecha_str: "2026-11-22",
            categoria_str: "Ventas",
            nota_opt: None,
            dispositivo_id: &disp_b,
        },
    )
    .unwrap();
    let r_antiguo = importar_bytes(&mut repo_b, &respaldo_antiguo, None).unwrap();
    assert_eq!(r_antiguo.nuevos, 0);
    assert_eq!(r_antiguo.ya_existian, 3);
    assert_eq!(repo_b.obtener_eventos().unwrap().len(), 4);
    assert_eq!(
        ejecutar_obtener_detalle_cuenta(&repo_b, &cid)
            .unwrap()
            .saldo_actual,
        "201.67"
    );

    // Conflicto: mismo identificador con contenido distinto -> rechazado con motivo;
    // el resto de eventos nuevos válidos sí se aplica normalmente (Sección 2.1 y 2.7)
    let mut eventos_existentes = repo_b.obtener_eventos().unwrap();
    let mut evento_conflictivo = eventos_existentes.remove(1);
    evento_conflictivo.datos_json = evento_conflictivo.datos_json.replace("100.00", "999.99");

    // Crear un cuarto evento válido en A y mezclarlo con el conflictivo
    let disp_a = repo_a.dispositivo_id().unwrap();
    ejecutar_registrar_movimiento(
        &mut repo_a,
        RegistrarMovimiento {
            cuenta_id: &cid,
            tipo_str: "gasto",
            monto_str: "10.00",
            fecha_str: "2026-11-23",
            categoria_str: "Transporte",
            nota_opt: Some("Bus".into()),
            dispositivo_id: &disp_a,
        },
    )
    .unwrap();
    let evento_nuevo_valido = repo_a.obtener_eventos().unwrap().pop().unwrap();

    let res_conflicto = repo_b
        .fusionar_eventos(&[evento_conflictivo.clone(), evento_nuevo_valido])
        .unwrap();
    assert_eq!(res_conflicto.nuevos, 1);
    assert_eq!(res_conflicto.rechazados.len(), 1);
    assert_eq!(res_conflicto.rechazados[0].evento_id, evento_conflictivo.id);
    assert!(!res_conflicto.rechazados[0].motivo.is_empty());

    // El saldo refleja 201.67 - 10.00 = 191.67 (el conflictivo 999.99 NO modificó el libro)
    assert_eq!(
        ejecutar_obtener_detalle_cuenta(&repo_b, &cid)
            .unwrap()
            .saldo_actual,
        "191.67"
    );
}

#[test]
fn fusion_en_distinto_orden() {
    use crate::domain::ejecutar_consultar_patrimonio;

    let dir = DirTemp::nuevo("orden_eventos");
    let mut a = bd_en(dir.path(), "a");
    let mut b = bd_en(dir.path(), "b");

    let disp = a.dispositivo_id().unwrap();
    let cuenta = ejecutar_crear_cuenta(
        &mut a, "Efectivo EUR", "efectivo", "EUR", "100.00", &disp
    ).unwrap();

    let cid = cuenta.id();
    let inicial = a.obtener_eventos().unwrap();
    b.fusionar_eventos(&inicial).unwrap();

    let mut eventos = Vec::new();

    let tasas = [
        (4, "2026-11-20T16:00:00Z", "b", "9.00"),
        (3, "2026-11-21T16:00:00Z", "a", "8.00"),
        (1, "2026-11-21T16:00:00Z", "b", "2.00"),
        (2, "2026-11-21T16:00:00Z", "b", "3.00"),
    ];

    // Armamos los eventos de las tasas
    for (n, fecha, disp, valor) in tasas {
        let id = format!("00000000-0000-4000-8000-00000000000{}", n);
        let momento = chrono::DateTime::parse_from_rfc3339(fecha).unwrap();
        let momento = momento.with_timezone(&chrono::Utc);

        let datos = serde_json::json!({
            "id": id,
            "origen": "EUR",
            "destino": "USD",
            "valor": valor,
            "fecha": "2026-11-20",
            "fuente": "Prueba"
        });

        eventos.push(EventoLibro {
            id,
            tipo: "TasaRegistrada".into(),
            momento_registro: momento,
            dispositivo_origen: disp.into(),
            datos_json: datos.to_string(),
        });
    }

    // Pasamos los eventos al primer libro
    for ev in &eventos {
        let res = a.fusionar_eventos(&[ev.clone()]).unwrap();
        assert_eq!(res.nuevos, 1);
    }

    // Hacemos lo mismo pero al reves
    for i in (0..eventos.len()).rev() {
        let res = b.fusionar_eventos(&[eventos[i].clone()]).unwrap();
        assert_eq!(res.nuevos, 1);
    }

    assert_eq!(a.obtener_eventos().unwrap().len(), 5);
    assert_eq!(b.obtener_eventos().unwrap().len(), 5);

    let hoy = chrono::Local::now().date_naive();

    let det_a = ejecutar_obtener_detalle_cuenta(&a, cid).unwrap();
    let det_b = ejecutar_obtener_detalle_cuenta(&b, cid).unwrap();

    let pat_a = ejecutar_consultar_patrimonio(&a, None, hoy).unwrap();
    let pat_b = ejecutar_consultar_patrimonio(&b, None, hoy).unwrap();

    // Vemos si al final nos da lo mismo
    assert_eq!(det_a.saldo_actual, "100.00");
    assert_eq!(det_a.saldo_actual, det_b.saldo_actual);

    assert_eq!(pat_a.total.as_deref(), Some("300.00"));
    assert_eq!(pat_a.total, pat_b.total);

    assert_eq!(pat_a.filas[0].tasa.as_ref().unwrap().id, eventos[3].id);
    assert_eq!(pat_b.filas[0].tasa.as_ref().unwrap().id, eventos[3].id);
}

#[test]
fn gastos_de_grupo_descargados_en_dos_dispositivos_convergen_una_sola_vez() {
    let dir = DirTemp::nuevo("grupo_dedup");
    let mut disp1 = bd_en(dir.path(), "disp1");
    let mut disp2 = bd_en(dir.path(), "disp2");

    // Ambos parten de la misma cuenta USD
    let id1 = disp1.dispositivo_id().unwrap();
    let cuenta = ejecutar_crear_cuenta(
        &mut disp1,
        "Efectivo USD",
        "efectivo",
        "USD",
        "200.00",
        &id1,
    )
    .unwrap();
    let cid = cuenta.id().to_string();
    let (exp, _, _) =
        exportar_bytes(&disp1, false, None, None, ParametrosKdf::para_pruebas()).unwrap();
    importar_bytes(&mut disp2, &exp, None).unwrap();

    let grupo_id = Uuid::new_v4().to_string();
    let evento_servidor_id = Uuid::new_v4().to_string();
    let gasto_id = Uuid::new_v4().to_string();

    disp1.elegir_cuenta_grupo(&grupo_id, &cid).unwrap();
    disp2.elegir_cuenta_grupo(&grupo_id, &cid).unwrap();

    let cambio_servidor = EventoCambioRecibido {
        id: evento_servidor_id,
        cursor: 1,
        tipo: "gasto_grupo".into(),
        grupo_id,
        grupo_nombre: "Viaje".into(),
        gasto_id,
        concepto: "Hotel".into(),
        fecha: "2026-11-20".into(),
        moneda: "USD".into(),
        monto_parte: "33.33".into(),
        incluye_centimos_redondeo: false,
    };

    // Ambos dispositivos descargan la misma parte del servidor por separado
    assert_eq!(
        disp1
            .aplicar_lote_cambios(1, vec![cambio_servidor.clone()])
            .unwrap(),
        1
    );
    assert_eq!(
        disp2
            .aplicar_lote_cambios(1, vec![cambio_servidor])
            .unwrap(),
        1
    );

    // Al fusionar disp2 en disp1, el evento derivado coincide y NO se duplica (Sección 2.1)
    let evs2 = disp2.obtener_eventos().unwrap();
    let res = disp1.fusionar_eventos(&evs2).unwrap();
    assert_eq!(res.nuevos, 0);
    assert_eq!(res.ya_existian, 2);
    assert!(res.rechazados.is_empty());

    let det = ejecutar_obtener_detalle_cuenta(&disp1, &cid).unwrap();
    assert_eq!(det.saldo_actual, "166.67");
    assert_eq!(det.movimientos.len(), 1);
}
