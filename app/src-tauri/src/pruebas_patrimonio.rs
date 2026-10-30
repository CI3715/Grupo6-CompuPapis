//! Regresiones de consolidación, consultas históricas y persistencia local.
use crate::domain::*;
use crate::sqlite_repo::RepositorioSqlite;
use crate::tasas_monedas::*;
use chrono::{Local, NaiveDate, TimeZone, Utc};
use rust_decimal::Decimal;
use serde_json::{Value, json};

struct RelojFijo;
impl Reloj for RelojFijo {
    fn ahora(&self) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 26, 16, 0, 0).unwrap()
    }
}
fn dia(fecha: &str) -> NaiveDate {
    NaiveDate::parse_from_str(fecha, "%Y-%m-%d").unwrap()
}
fn consultar(repo: &impl RepositorioEventos, fecha: Option<&str>) -> Patrimonio {
    ejecutar_consultar_patrimonio(repo, fecha, Local::now().date_naive()).unwrap()
}
fn crear_cuenta_fixture(repo: &mut impl RepositorioEventos, moneda: &str, saldo: &str) -> String {
    // Cada cuenta del fixture existe desde el 10/10; el caso de uso de producción
    // sigue registrando la fecha real del sistema y nunca reescribe eventos.
    let mut temporal = RepositorioEnMemoria::default();
    let cuenta =
        ejecutar_crear_cuenta(&mut temporal, moneda, "efectivo", moneda, saldo, "test").unwrap();
    let mut evento = temporal.obtener_eventos().unwrap().remove(0);
    evento.momento_registro = Utc.with_ymd_and_hms(2026, 10, 10, 16, 0, 0).unwrap();
    repo.guardar_evento(&evento).unwrap();
    cuenta.id().to_string()
}
fn agregar_gasto_fixture(repo: &mut impl RepositorioEventos, cuenta_id: &str, historico: bool) {
    let datos = if historico {
        json!({"movimiento_id":"gasto", "cuenta_id":cuenta_id,
            "tipo_movimiento":"gasto", "monto":"20.00", "categoria":"Alimentación", "nota":null})
    } else {
        json!({"movimiento_id":"gasto", "cuenta_id":cuenta_id, "tipo":"gasto",
            "monto":{"cantidad":"20.00","moneda":"USD"}, "fecha":"2026-10-20",
            "categoria":"Alimentación", "nota":null})
    };
    repo.guardar_evento(&EventoLibro {
        id: "evento-gasto".into(),
        tipo: "MovimientoRegistrado".into(),
        momento_registro: Utc.with_ymd_and_hms(2026, 10, 20, 16, 0, 0).unwrap(),
        dispositivo_origen: "test".into(),
        datos_json: datos.to_string(),
    })
    .unwrap();
}
fn tasa(
    repo: &mut impl RepositorioEventos,
    origen: &str,
    destino: &str,
    valor: &str,
) -> TasaCambio {
    ejecutar_registrar_tasa(
        repo,
        origen,
        destino,
        valor,
        "2026-10-20",
        "BCV",
        "test",
        &RelojFijo,
        &crate::GeneradorUuid,
    )
    .unwrap()
}
fn cambiar_base(repo: &mut impl RepositorioEventos, codigo: &str) {
    ejecutar_cambiar_moneda_base(repo, codigo, "test", &RelojFijo, &crate::GeneradorUuid).unwrap();
}
fn verificar_saldo(repo: &impl RepositorioEventos, id: &str, esperado: &str) {
    let detalle = ejecutar_obtener_detalle_cuenta(repo, id).unwrap();
    let patrimonio = consultar(repo, None);
    assert_eq!(detalle.saldo_actual, esperado);
    assert_eq!(patrimonio.filas[0].saldo, esperado);
    assert_eq!(patrimonio.subtotales[0].subtotal, esperado);
    assert_eq!(patrimonio.total.as_deref(), Some(esperado));
}

#[test]
fn gastos_actuales_e_historicos_se_restan_una_vez_y_los_ajustes_respetan_su_fecha() {
    for historico in [false, true] {
        let mut repo = RepositorioEnMemoria::default();
        let id = crear_cuenta_fixture(&mut repo, "USD", "100.00");
        agregar_gasto_fixture(&mut repo, &id, historico);
        let originales = repo.obtener_eventos().unwrap();
        verificar_saldo(&repo, &id, "80.00");
        assert_eq!(
            consultar(&repo, Some("2026-10-19")).total.as_deref(),
            Some("100.00")
        );
        assert_eq!(
            consultar(&repo, Some("2026-10-21")).total.as_deref(),
            Some("80.00")
        );
        ejecutar_corregir_movimiento(&mut repo, &id, "gasto", "5.00", "test").unwrap();
        verificar_saldo(&repo, &id, "95.00");
        assert_eq!(
            consultar(&repo, Some("2026-10-21")).total.as_deref(),
            Some("80.00")
        );
        ejecutar_corregir_movimiento(&mut repo, &id, "gasto", "0.00", "test").unwrap();
        verificar_saldo(&repo, &id, "100.00");
        assert_eq!(
            consultar(&repo, Some("2026-10-21")).total.as_deref(),
            Some("80.00")
        );
        let eventos = repo.obtener_eventos().unwrap();
        assert_eq!(&eventos[..originales.len()], originales);
        assert_eq!(eventos.len(), originales.len() + 2);
        assert!(eventos[2..].iter().all(|e| e.tipo == "MovimientoCorregido"));
    }
}

#[test]
fn cuentas_no_existen_antes_de_su_creacion_y_consultas_futuras_se_rechazan() {
    let mut repo = RepositorioEnMemoria::default();
    crear_cuenta_fixture(&mut repo, "USD", "100.00");
    let antes = consultar(&repo, Some("2026-10-09"));
    assert!(antes.filas.is_empty());
    assert!(antes.subtotales.is_empty());
    assert_eq!(antes.total.as_deref(), Some("0.00"));
    let listado = ejecutar_listar_cuentas_en_fecha(&repo, dia("2026-10-09")).unwrap();
    assert!(listado.cuentas.is_empty());
    assert_eq!(
        consultar(&repo, Some("2026-10-10")).total.as_deref(),
        Some("100.00")
    );
    let eventos = repo.obtener_eventos().unwrap();
    for fecha in ["", "2026-02-30", "2026-10-27"] {
        assert_eq!(
            ejecutar_consultar_patrimonio(&repo, Some(fecha), dia("2026-10-26"))
                .unwrap_err()
                .campo,
            "fecha"
        );
    }
    let futuro = Local::now().date_naive().succ_opt().unwrap();
    assert_eq!(
        ejecutar_listar_cuentas_en_fecha(&repo, futuro)
            .unwrap_err()
            .campo,
        "fecha"
    );
    assert_eq!(repo.obtener_eventos().unwrap(), eventos);
}

#[test]
fn inversa_divide_sin_aproximacion_previa_y_conversion_fuera_de_rango_es_error() {
    let tasa = TasaCambio {
        id: "t".into(),
        origen: "USD".into(),
        destino: "VES".into(),
        valor: "887".parse().unwrap(),
        fecha: "2026-10-20".into(),
        fuente: "BCV".into(),
    };
    let (valor, usada, inversa) = convertir_monto_a_base(
        "88700".parse().unwrap(),
        "VES",
        "USD",
        dia("2026-10-21"),
        std::slice::from_ref(&tasa),
    )
    .unwrap();
    assert_eq!(valor, Decimal::from(100)); // Sin round_dp para esconder el error de la inversa aproximada.
    assert!(inversa);
    assert_eq!(usada.unwrap(), tasa);
    let err = convertir_monto_a_base(
        Decimal::MAX,
        "USD",
        "VES",
        dia("2026-10-21"),
        std::slice::from_ref(&tasa),
    )
    .unwrap_err();
    assert_eq!(err.campo, "valor");
    let mut invalida = tasa;
    invalida.valor = Decimal::ZERO;
    assert!(
        convertir_monto_a_base(Decimal::ONE, "VES", "USD", dia("2026-10-21"), &[invalida]).is_err()
    );
}

#[test]
fn sqlite_conserva_gastos_corregidos_catalogo_y_base_no_predeterminada_al_reabrir() {
    let ruta = std::env::temp_dir().join(format!(
        "patrimonio-regresion-{}.sqlite",
        uuid::Uuid::new_v4()
    ));
    let mut repo = RepositorioSqlite::inicializar(ruta.clone()).unwrap();
    let id = crear_cuenta_fixture(&mut repo, "USD", "100.00");
    agregar_gasto_fixture(&mut repo, &id, true);
    verificar_saldo(&repo, &id, "80.00");
    ejecutar_corregir_movimiento(&mut repo, &id, "gasto", "5.00", "test").unwrap();
    verificar_saldo(&repo, &id, "95.00");
    ejecutar_crear_moneda(
        &mut repo,
        "ABC",
        "Moneda propia",
        "A",
        8,
        "test",
        &RelojFijo,
        &crate::GeneradorUuid,
    )
    .unwrap();
    tasa(&mut repo, "USD", "VES", "887");
    cambiar_base(&mut repo, "VES");
    let antes = serde_json::to_value(consultar(&repo, None)).unwrap();
    assert_eq!(antes["total"], "84265.00");
    assert_eq!(
        consultar(&repo, Some("2026-10-21")).total.as_deref(),
        Some("70960.00")
    );
    let eventos = repo.obtener_eventos().unwrap();
    assert!(
        ejecutar_cambiar_moneda_base(&mut repo, "ZZZ", "test", &RelojFijo, &crate::GeneradorUuid)
            .is_err()
    );
    assert!(
        ejecutar_crear_moneda(
            &mut repo,
            "ABC",
            "Duplicada",
            "A",
            8,
            "test",
            &RelojFijo,
            &crate::GeneradorUuid
        )
        .is_err()
    );
    drop(repo);
    let repo = RepositorioSqlite::inicializar(ruta).unwrap();
    assert_eq!(ejecutar_obtener_moneda_base(&repo).unwrap(), "VES");
    assert_eq!(ejecutar_listar_monedas(&repo).unwrap().len(), 5);
    assert_eq!(serde_json::to_value(consultar(&repo, None)).unwrap(), antes);
    assert_eq!(repo.obtener_eventos().unwrap(), eventos);
    assert_eq!(
        ejecutar_obtener_detalle_cuenta(&repo, &id)
            .unwrap()
            .saldo_actual,
        "95.00"
    );
}

#[test]
fn esc01_sqlite_reapertura_anulacion_y_contrato_json_del_frontend() {
    let ruta = std::env::temp_dir().join(format!("esc01-{}.sqlite", uuid::Uuid::new_v4()));
    let mut repo = RepositorioSqlite::inicializar(ruta.clone()).unwrap();
    for (moneda, saldo) in [("VES", "88700"), ("USD", "200"), ("USDT", "50")] {
        crear_cuenta_fixture(&mut repo, moneda, saldo);
    }
    tasa(&mut repo, "USD", "VES", "887");
    tasa(&mut repo, "USDT", "VES", "1000");
    cambiar_base(&mut repo, "VES");
    drop(repo);
    let mut repo = RepositorioSqlite::inicializar(ruta.clone()).unwrap();
    let ves = consultar(&repo, Some("2026-10-23"));
    assert_eq!(ves.total.as_deref(), Some("316100.00"));
    let json = serde_json::to_value(&ves).unwrap();
    assert_eq!(json["monedaBase"], "VES");
    assert_eq!(json["fecha"], "2026-10-23");
    assert_eq!(json["filas"][0]["cuentaId"], ves.filas[0].cuenta_id);
    assert!(json["filas"][0]["saldo"].is_string());
    assert!(json["filas"][1]["tasa"]["valor"].is_string());
    assert_eq!(json["filas"][1]["tasa"]["fuente"], "BCV");
    assert_eq!(json["filas"][1]["tasa"]["fecha"], "2026-10-20");
    cambiar_base(&mut repo, "USD");
    let incompleto = consultar(&repo, Some("2026-10-23"));
    let json = serde_json::to_value(incompleto).unwrap();
    assert_eq!(json["total"], Value::Null);
    assert_eq!(
        json["faltantes"],
        json!([{"origen":"USDT","destino":"USD"}])
    );
    assert_eq!(json["filas"][0]["inversa"], true);
    let registrada = tasa(&mut repo, "USDT", "USD", "1");
    assert_eq!(
        consultar(&repo, Some("2026-10-23")).total.as_deref(),
        Some("350.00")
    );
    let cuentas = serde_json::to_value(ejecutar_listar_cuentas(&repo).unwrap()).unwrap();
    ejecutar_eliminar_tasa(
        &mut repo,
        &registrada.id,
        "test",
        &RelojFijo,
        &crate::GeneradorUuid,
    )
    .unwrap();
    let eventos = repo.obtener_eventos().unwrap();
    assert_eq!(eventos.last().unwrap().tipo, "TasaEliminada");
    drop(repo);
    let repo = RepositorioSqlite::inicializar(ruta).unwrap();
    assert!(consultar(&repo, Some("2026-10-23")).total.is_none());
    assert_eq!(
        serde_json::to_value(ejecutar_listar_cuentas(&repo).unwrap()).unwrap(),
        cuentas
    );
    assert_eq!(repo.obtener_eventos().unwrap(), eventos);
}

#[test]
fn cuentas_movimientos_y_patrimonio_admiten_todas_las_escalas_de_cero_a_ocho() {
    for decimales in 0..=8 {
        let mut repo = RepositorioEnMemoria::default();
        ejecutar_crear_moneda(
            &mut repo,
            "ABC",
            "Moneda propia",
            "A",
            decimales,
            "test",
            &RelojFijo,
            &crate::GeneradorUuid,
        )
        .unwrap();
        let inicial = if decimales == 0 {
            "12".into()
        } else {
            format!("12.{}1", "0".repeat((decimales - 1) as usize))
        };
        let cuenta =
            ejecutar_crear_cuenta(&mut repo, "Caja", "efectivo", "ABC", &inicial, "test").unwrap();
        let ingreso = if decimales == 0 {
            "1".into()
        } else {
            format!("0.{}1", "0".repeat((decimales - 1) as usize))
        };
        let detalle = ejecutar_registrar_movimiento(
            &mut repo,
            RegistrarMovimiento {
                cuenta_id: cuenta.id(),
                tipo_str: "ingreso",
                monto_str: &ingreso,
                fecha_str: &Local::now().date_naive().to_string(),
                categoria_str: "Salario",
                nota_opt: None,
                dispositivo_id: "test",
            },
        )
        .unwrap();
        cambiar_base(&mut repo, "ABC");
        let patrimonio = consultar(&repo, None);
        assert_eq!(
            patrimonio.total.as_deref(),
            Some(detalle.saldo_actual.as_str())
        );
        assert_eq!(patrimonio.filas[0].saldo, detalle.saldo_actual);
        assert_eq!(patrimonio.subtotales[0].subtotal, detalle.saldo_actual);
    }
}
