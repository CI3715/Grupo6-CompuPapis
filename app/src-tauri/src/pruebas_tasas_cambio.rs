use crate::domain::*;
use crate::tasas_monedas::*;
use chrono::{TimeZone, Utc};

struct RelojFijo;
impl Reloj for RelojFijo {
    fn ahora(&self) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 28, 16, 0, 0).unwrap()
    }
}
fn registrar<R: RepositorioEventos>(
    repo: &mut R,
    origen: &str,
    destino: &str,
    valor: &str,
    fecha: &str,
) -> TasaCambio {
    ejecutar_registrar_tasa(
        repo,
        origen,
        destino,
        valor,
        fecha,
        " BCV ",
        "test",
        &RelojFijo,
        &crate::GeneradorUuid,
    )
    .unwrap()
}
fn eliminar<R: RepositorioEventos>(repo: &mut R, id: &str) -> Result<(), ErrorDominio> {
    ejecutar_eliminar_tasa(repo, id, "test", &RelojFijo, &crate::GeneradorUuid)
}
fn patrimonio<R: RepositorioEventos>(repo: &R) -> Patrimonio {
    ejecutar_consultar_patrimonio(repo, None, chrono::Local::now().date_naive()).unwrap()
}
fn base<R: RepositorioEventos>(repo: &mut R, codigo: &str) {
    ejecutar_cambiar_moneda_base(repo, codigo, "test", &RelojFijo, &crate::GeneradorUuid).unwrap();
}

#[test]
fn registro_exacto_validaciones_por_campo_y_rechazos_sin_eventos() {
    let mut repo = RepositorioEnMemoria::default();
    for (origen, destino, valor, fecha, fuente, campo) in [
        ("", "VES", "1", "2026-10-20", "BCV", "origen"),
        ("ZZZ", "VES", "1", "2026-10-20", "BCV", "origen"),
        ("USD", "ZZZ", "1", "2026-10-20", "BCV", "destino"),
        ("USD", "", "1", "2026-10-20", "BCV", "destino"),
        ("USD", "USD", "1", "2026-10-20", "BCV", "destino"),
        ("USD", "VES", "", "2026-10-20", "BCV", "valor"),
        ("USD", "VES", "abc", "2026-10-20", "BCV", "valor"),
        ("USD", "VES", "0", "2026-10-20", "BCV", "valor"),
        ("USD", "VES", "-1", "2026-10-20", "BCV", "valor"),
        ("USD", "VES", "1.123456789", "2026-10-20", "BCV", "valor"),
        ("USD", "VES", "1", "", "BCV", "fecha"),
        ("USD", "VES", "1", "2026-02-30", "BCV", "fecha"),
        ("USD", "VES", "1", "2026-10-29", "BCV", "fecha"),
        ("USD", "VES", "1", "2026-10-20", "   ", "fuente"),
    ] {
        let err = ejecutar_registrar_tasa(
            &mut repo,
            origen,
            destino,
            valor,
            fecha,
            fuente,
            "test",
            &RelojFijo,
            &crate::GeneradorUuid,
        )
        .unwrap_err();
        assert_eq!(err.campo, campo);
        assert!(repo.obtener_eventos().unwrap().is_empty());
    }
    let t = registrar(
        &mut repo,
        "USD",
        "VES",
        "9007199254740993.12345678",
        "2026-10-20",
    );
    assert_eq!(t.fuente, "BCV");
    let json = serde_json::to_value(&t).unwrap();
    assert_eq!(json["valor"], "9007199254740993.12345678");
    assert_eq!(
        ejecutar_historial_tasas(&repo, "USD", "VES").unwrap(),
        vec![t]
    );
    ejecutar_crear_moneda(
        &mut repo,
        "ABC",
        "Propia",
        "A",
        8,
        "test",
        &RelojFijo,
        &crate::GeneradorUuid,
    )
    .unwrap();
    registrar(&mut repo, "ABC", "USD", "0.00000001", "2026-10-20");
}

#[test]
fn historial_desempate_anulacion_y_movimientos_intactos() {
    let mut repo = RepositorioEnMemoria::default();
    let cuenta = ejecutar_crear_cuenta(&mut repo, "USD", "banco", "USD", "100", "test").unwrap();
    ejecutar_registrar_movimiento(
        &mut repo,
        RegistrarMovimiento {
            cuenta_id: cuenta.id(),
            tipo_str: "ingreso",
            monto_str: "10",
            fecha_str: "2026-10-20",
            categoria_str: "Salario",
            nota_opt: None,
            dispositivo_id: "test",
        },
    )
    .unwrap();
    base(&mut repo, "VES");
    let antes =
        serde_json::to_value(ejecutar_obtener_detalle_cuenta(&repo, cuenta.id()).unwrap()).unwrap();
    let vieja = registrar(&mut repo, "USD", "VES", "800", "2026-10-19");
    let primera = registrar(&mut repo, "USD", "VES", "887", "2026-10-20");
    let ultima = registrar(&mut repo, "USD", "VES", "900", "2026-10-20");
    let historial = ejecutar_historial_tasas(&repo, "USD", "VES").unwrap();
    assert_eq!(
        historial.iter().map(|t| &t.id).collect::<Vec<_>>(),
        vec![&ultima.id, &primera.id, &vieja.id]
    );
    assert_eq!(patrimonio(&repo).total.as_deref(), Some("99000.00"));
    eliminar(&mut repo, &ultima.id).unwrap();
    assert_eq!(patrimonio(&repo).total.as_deref(), Some("97570.00"));
    let n = repo.obtener_eventos().unwrap().len();
    for id in [&ultima.id, "inexistente"] {
        assert_eq!(eliminar(&mut repo, id).unwrap_err().campo, "tasa_id");
        assert_eq!(repo.obtener_eventos().unwrap().len(), n);
    }
    assert_eq!(
        ejecutar_historial_tasas(&repo, "USD", "VES").unwrap().len(),
        2
    );
    assert_eq!(
        repo.obtener_eventos().unwrap().last().unwrap().tipo,
        "TasaEliminada"
    );
    assert_eq!(
        serde_json::to_value(ejecutar_obtener_detalle_cuenta(&repo, cuenta.id()).unwrap()).unwrap(),
        antes
    );
}

#[test]
fn ejemplo_23_inv02_inversa_sin_cruces_y_cambio_base() {
    let mut repo = RepositorioEnMemoria::default();
    for (codigo, monto) in [("VES", "88700"), ("USD", "200"), ("USDT", "50")] {
        ejecutar_crear_cuenta(&mut repo, codigo, "efectivo", codigo, monto, "test").unwrap();
    }
    let antes = serde_json::to_value(ejecutar_listar_cuentas(&repo).unwrap()).unwrap();
    assert!(patrimonio(&repo).total.is_none());
    base(&mut repo, "VES");
    registrar(&mut repo, "USD", "VES", "887", "2026-10-20");
    let usdt = registrar(&mut repo, "USDT", "VES", "1000", "2026-10-21");
    assert_eq!(patrimonio(&repo).total.as_deref(), Some("316100.00"));
    base(&mut repo, "USD");
    let p = patrimonio(&repo);
    assert!(p.total.is_none());
    assert_eq!(p.faltantes.len(), 1);
    assert_eq!(p.faltantes[0].origen, "USDT");
    assert_eq!(p.faltantes[0].destino, "USD");
    assert!(p.filas.iter().find(|f| f.moneda == "VES").unwrap().inversa);
    registrar(&mut repo, "USDT", "USD", "1", "2026-10-21");
    assert_eq!(patrimonio(&repo).total.as_deref(), Some("350.00"));
    base(&mut repo, "VES");
    eliminar(&mut repo, &usdt.id).unwrap();
    assert!(patrimonio(&repo).total.is_none());
    assert_eq!(
        serde_json::to_value(ejecutar_listar_cuentas(&repo).unwrap()).unwrap(),
        antes
    );
}

#[test]
fn tasa_vigente_y_saldos_a_fecha_redondeo_solo_al_final() {
    let mut repo = RepositorioEnMemoria::default();
    // Fechas de creación controladas para demostrar consultas anteriores a hoy.
    for codigo in ["USD", "EUR"] {
        let mut temporal = RepositorioEnMemoria::default();
        ejecutar_crear_cuenta(&mut temporal, codigo, "efectivo", codigo, "1", "test").unwrap();
        let mut ev = temporal.obtener_eventos().unwrap().remove(0);
        ev.momento_registro = Utc.with_ymd_and_hms(2026, 10, 10, 16, 0, 0).unwrap();
        repo.guardar_evento(&ev).unwrap();
    }
    base(&mut repo, "VES");
    registrar(&mut repo, "USD", "VES", "0.004", "2026-10-19");
    registrar(&mut repo, "EUR", "VES", "0.004", "2026-10-19");
    registrar(&mut repo, "USD", "VES", "100", "2026-10-21");
    let p =
        ejecutar_consultar_patrimonio(&repo, Some("2026-10-20"), RelojFijo.ahora().date_naive())
            .unwrap();
    assert_eq!(p.total.as_deref(), Some("0.01"));
    assert_eq!(p.filas[0].tasa.as_ref().unwrap().fecha, "2026-10-19");
    let cuenta_id = p.filas[0].cuenta_id.clone();
    ejecutar_registrar_movimiento(
        &mut repo,
        RegistrarMovimiento {
            cuenta_id: &cuenta_id,
            tipo_str: "ingreso",
            monto_str: "10",
            fecha_str: "2026-10-21",
            categoria_str: "Salario",
            nota_opt: None,
            dispositivo_id: "test",
        },
    )
    .unwrap();
    let p =
        ejecutar_consultar_patrimonio(&repo, Some("2026-10-20"), RelojFijo.ahora().date_naive())
            .unwrap();
    assert_eq!(p.filas[0].saldo, "1.00");
    assert!(
        ejecutar_consultar_patrimonio(&repo, Some("2026-10-29"), RelojFijo.ahora().date_naive())
            .is_err()
    );
}

#[test]
fn sqlite_reapertura_conserva_tasas_anulaciones_y_rechazos() {
    let ruta = std::env::temp_dir().join(format!("tasas-cambio-{}.sqlite", uuid::Uuid::new_v4()));
    let mut repo = crate::sqlite_repo::RepositorioSqlite::inicializar(ruta.clone()).unwrap();
    let t = registrar(&mut repo, "USD", "VES", "887.12345678", "2026-10-20");
    drop(repo);
    let mut repo = crate::sqlite_repo::RepositorioSqlite::inicializar(ruta.clone()).unwrap();
    assert_eq!(ejecutar_listar_tasas(&repo).unwrap(), vec![t.clone()]);
    eliminar(&mut repo, &t.id).unwrap();
    let eventos = repo.obtener_eventos().unwrap();
    assert!(eliminar(&mut repo, &t.id).is_err());
    assert!(
        ejecutar_registrar_tasa(
            &mut repo,
            "USD",
            "VES",
            "0",
            "2026-10-20",
            "BCV",
            "test",
            &RelojFijo,
            &crate::GeneradorUuid
        )
        .is_err()
    );
    drop(repo);
    let repo = crate::sqlite_repo::RepositorioSqlite::inicializar(ruta.clone()).unwrap();
    assert!(ejecutar_listar_tasas(&repo).unwrap().is_empty());
    assert_eq!(repo.obtener_eventos().unwrap(), eventos);
    assert_eq!(eventos.len(), 2);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&eventos[0].datos_json).unwrap()["valor"],
        "887.12345678"
    );
    // Se conserva este archivo temporal de prueba; nunca se toca la base del usuario.
}

#[test]
fn entrada_comando_ausente_o_null_devuelve_campo_sin_escribir() {
    let mut repo = RepositorioEnMemoria::default();
    for campo in ["origen", "destino", "valor", "fecha", "fuente"] {
        for null in [false, true] {
            let mut json = serde_json::json!({"origen":"USD","destino":"VES","valor":"887","fecha":"2026-10-20","fuente":"BCV"});
            if null {
                json[campo] = serde_json::Value::Null;
            } else {
                json.as_object_mut().unwrap().remove(campo);
            }
            let entrada = serde_json::from_value(json).unwrap();
            let err = registrar_tasa_desde_entrada(
                &mut repo,
                entrada,
                "test",
                &RelojFijo,
                &crate::GeneradorUuid,
            )
            .unwrap_err();
            assert_eq!(err.campo, campo);
            assert!(repo.obtener_eventos().unwrap().is_empty());
        }
    }
    let err = ejecutar_registrar_tasa(
        &mut repo,
        "USD",
        "VES",
        "1.00000000000000000000000000001",
        "2026-10-20",
        "BCV",
        "test",
        &RelojFijo,
        &crate::GeneradorUuid,
    )
    .unwrap_err();
    assert_eq!(err.campo, "valor");
    assert!(repo.obtener_eventos().unwrap().is_empty());
}

#[test]
fn sqlite_reapertura_conserva_catalogo_base_y_patrimonio() {
    let ruta = std::env::temp_dir().join(format!("patrimonio-{}.sqlite", uuid::Uuid::new_v4()));
    let mut repo = crate::sqlite_repo::RepositorioSqlite::inicializar(ruta.clone()).unwrap();
    ejecutar_crear_moneda(
        &mut repo,
        "ABC",
        "Moneda",
        "A",
        8,
        "test",
        &RelojFijo,
        &crate::GeneradorUuid,
    )
    .unwrap();
    ejecutar_crear_cuenta(&mut repo, "Propia", "efectivo", "ABC", "1.12345678", "test").unwrap();
    registrar(&mut repo, "ABC", "USD", "2", "2026-10-20");
    base(&mut repo, "USD");
    let antes = serde_json::to_value(patrimonio(&repo)).unwrap();
    let eventos = repo.obtener_eventos().unwrap();
    drop(repo);
    let repo = crate::sqlite_repo::RepositorioSqlite::inicializar(ruta).unwrap();
    assert_eq!(ejecutar_listar_monedas(&repo).unwrap().len(), 5);
    assert_eq!(ejecutar_obtener_moneda_base(&repo).unwrap(), "USD");
    assert_eq!(serde_json::to_value(patrimonio(&repo)).unwrap(), antes);
    assert_eq!(repo.obtener_eventos().unwrap(), eventos);
}

#[test]
fn tasas_reproyeccion_idempotente_y_eventos_incompatibles_explicitos() {
    let mut repo = RepositorioEnMemoria::default();
    let t = registrar(&mut repo, "USD", "VES", "887", "2026-10-20");
    let evento = repo.obtener_eventos().unwrap().remove(0);
    repo.guardar_evento(&evento).unwrap();
    assert_eq!(ejecutar_listar_tasas(&repo).unwrap().len(), 1);
    eliminar(&mut repo, &t.id).unwrap();
    let evento = repo.obtener_eventos().unwrap().pop().unwrap();
    repo.guardar_evento(&evento).unwrap();
    assert!(ejecutar_listar_tasas(&repo).unwrap().is_empty());
    let mut incompatible = evento;
    incompatible.id = "otro".into();
    incompatible.datos_json = serde_json::json!({"tasa_id":"inexistente"}).to_string();
    repo.guardar_evento(&incompatible).unwrap();
    assert_eq!(
        ejecutar_historial_tasas(&repo, "USD", "VES")
            .unwrap_err()
            .campo,
        "almacenamiento"
    );
}
