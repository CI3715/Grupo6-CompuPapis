//! Regresiones de main (Maikel), dev y de la integración local-first.
use crate::domain::*;
use crate::sqlite_repo::RepositorioSqlite;
use chrono::{Local, Utc};
use rust_decimal::Decimal;
use serde_json::json;
use std::str::FromStr;

fn registrar<R: RepositorioEventos>(r: &mut R, id: &str, tipo: &str, monto: &str) -> DetalleCuenta {
    ejecutar_registrar_movimiento(
        r,
        RegistrarMovimiento {
            cuenta_id: id,
            tipo_str: tipo,
            monto_str: monto,
            fecha_str: &Local::now().date_naive().to_string(),
            categoria_str: if tipo == "ingreso" {
                "Salario"
            } else {
                "Alimentación"
            },
            nota_opt: Some("Prueba".into()),
            dispositivo_id: "pruebas",
        },
    )
    .unwrap()
}

fn evento(tipo: &str, datos: serde_json::Value) -> EventoLibro {
    EventoLibro {
        id: uuid::Uuid::new_v4().to_string(),
        tipo: tipo.into(),
        momento_registro: Utc::now(),
        dispositivo_origen: "historico".into(),
        datos_json: datos.to_string(),
    }
}

fn movimiento_historico<R: RepositorioEventos>(
    r: &mut R,
    cuenta: &str,
    tipo: &str,
    monto: &str,
) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    r.guardar_evento(&evento(
        "MovimientoRegistrado",
        json!({
            "movimiento_id":id,"cuenta_id":cuenta,"tipo_movimiento":tipo,"monto":monto,
            "categoria":if tipo=="ingreso" {"Salario"} else {"Supermercado"},"nota":"Formato main"
        }),
    ))
    .unwrap();
    id
}

#[test]
fn probar_error_correccion_ingreso_reportado() {
    let mut r = RepositorioEnMemoria::default();
    let c = ejecutar_crear_cuenta(
        &mut r,
        "Billetera dolares",
        "billetera digital",
        "USD",
        "1000.00",
        "prueba",
    )
    .unwrap();
    let id = movimiento_historico(&mut r, c.id(), "ingreso", "800.00");
    let d = ejecutar_corregir_movimiento(&mut r, c.id(), &id, "700.00", "prueba").unwrap();
    assert_eq!(d.saldo_actual, "1700.00");
    assert_eq!(d.composicion.ajustes, "-100.00");
}

#[test]
fn probar_correccion_gasto_reportado() {
    let mut r = RepositorioEnMemoria::default();
    let c = ejecutar_crear_cuenta(
        &mut r,
        "Billetera banco",
        "banco",
        "USD",
        "1000.00",
        "prueba",
    )
    .unwrap();
    let id = movimiento_historico(&mut r, c.id(), "gasto", "500.00");
    let d = ejecutar_corregir_movimiento(&mut r, c.id(), &id, "50.00", "prueba").unwrap();
    assert_eq!(d.saldo_actual, "950.00");
    assert_eq!(d.composicion.ajustes, "450.00");
}

#[test]
fn formatos_mixtos_persisten_sin_reescribir_eventos_y_saldo_1750() {
    for historico in [false, true] {
        let path =
            std::env::temp_dir().join(format!("cc-integracion-{}.sqlite", uuid::Uuid::new_v4()));
        let mut r = RepositorioSqlite::inicializar(path.clone()).unwrap();
        let c = ejecutar_crear_cuenta(
            &mut r,
            "Billetera dólares",
            "billetera digital",
            "USD",
            "1000.00",
            "prueba",
        )
        .unwrap();
        ejecutar_crear_categoria(&mut r, "Trabajo", "ingreso", "prueba").unwrap();
        let id = if historico {
            movimiento_historico(&mut r, c.id(), "ingreso", "800.00");
            let id = movimiento_historico(&mut r, c.id(), "gasto", "500.00");
            r.guardar_evento(&evento(
                "MovimientoCorregido",
                json!({
                    "ajuste_id":uuid::Uuid::new_v4().to_string(),"movimiento_original_id":id,
                    "cuenta_id":c.id(),"monto_ajuste":"450.00"
                }),
            ))
            .unwrap();
            id
        } else {
            registrar(&mut r, c.id(), "ingreso", "800.00");
            let d = registrar(&mut r, c.id(), "gasto", "500.00");
            let id = d.movimientos[0].id.clone();
            ejecutar_corregir_movimiento(&mut r, c.id(), &id, "50.00", "prueba").unwrap();
            id
        };
        let original = r.obtener_eventos().unwrap();
        for e in &original {
            r.guardar_evento(e).unwrap();
        }
        let antes = ejecutar_obtener_detalle_cuenta(&r, c.id()).unwrap();
        assert_eq!(antes.saldo_actual, "1750.00");
        assert_eq!(
            (
                antes.composicion.inicial.as_str(),
                antes.composicion.ingresos.as_str(),
                antes.composicion.gastos.as_str(),
                antes.composicion.ajustes.as_str()
            ),
            ("1000.00", "800.00", "500.00", "450.00")
        );
        drop(r);
        let mut r = RepositorioSqlite::inicializar(path.clone()).unwrap();
        let despues = ejecutar_obtener_detalle_cuenta(&r, c.id()).unwrap();
        assert_eq!(
            serde_json::to_value(&antes).unwrap(),
            serde_json::to_value(&despues).unwrap()
        );
        let orig = despues.movimientos.iter().find(|m| m.id == id).unwrap();
        assert!(orig.corregido);
        assert_eq!(orig.monto, "-500.00");
        assert!(
            despues
                .movimientos
                .iter()
                .any(|m| m.es_ajuste && m.referencia_id.as_deref() == Some(id.as_str()))
        );
        assert!(
            ejecutar_listar_categorias(&r)
                .unwrap()
                .iter()
                .any(|c| c.nombre == "Trabajo")
        );
        let d = ejecutar_corregir_movimiento(&mut r, c.id(), &id, "25.00", "prueba").unwrap();
        assert_eq!(d.saldo_actual, "1775.00");
        let d = ejecutar_corregir_movimiento(&mut r, c.id(), &id, "0", "prueba").unwrap();
        assert_eq!(d.saldo_actual, "1800.00");
        let posteriores = r.obtener_eventos().unwrap();
        assert_eq!(posteriores.len(), original.len() + 2);
        for (a, b) in original.iter().zip(&posteriores) {
            assert_eq!(a.id, b.id);
            assert_eq!(a.datos_json, b.datos_json);
            assert_eq!(a.momento_registro, b.momento_registro);
        }
        assert_eq!(
            ejecutar_listar_cuentas(&r).unwrap().subtotales[0].subtotal,
            "1800.00"
        );
        drop(r);
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn correcciones_multiples_ingresos_y_gastos_preservan_original_inv04_inv05() {
    for tipo in ["ingreso", "gasto"] {
        let mut r = RepositorioEnMemoria::default();
        let c =
            ejecutar_crear_cuenta(&mut r, "Cuenta", "banco", "USD", "1000.00", "prueba").unwrap();
        let d = registrar(&mut r, c.id(), tipo, "500.00");
        let id = d.movimientos[0].id.clone();
        let eventos_originales = r.obtener_eventos().unwrap();
        for monto in ["50.00", "80.00", "0.00", "25.00"] {
            let d = ejecutar_corregir_movimiento(&mut r, c.id(), &id, monto, "prueba").unwrap();
            let valor = Decimal::from_str(monto).unwrap();
            let esperado = Decimal::from(1000) + if tipo == "ingreso" { valor } else { -valor };
            assert_eq!(d.saldo_actual, format!("{esperado:.2}"));
            let original = d.movimientos.iter().find(|m| m.id == id).unwrap();
            assert!(original.corregido);
            assert_eq!(
                original.monto,
                if tipo == "ingreso" {
                    "500.00"
                } else {
                    "-500.00"
                }
            );
        }
        let posteriores = r.obtener_eventos().unwrap();
        for (a, b) in eventos_originales.iter().zip(&posteriores) {
            assert_eq!(a.datos_json, b.datos_json);
        }
    }
}

#[test]
fn validaciones_corregir_y_categorias_no_escriben_al_fallar() {
    let mut r = RepositorioEnMemoria::default();
    let c = ejecutar_crear_cuenta(&mut r, "Cuenta", "banco", "USD", "1000.00", "prueba").unwrap();
    let otra = ejecutar_crear_cuenta(&mut r, "Otra", "efectivo", "EUR", "0", "prueba").unwrap();
    let d = registrar(&mut r, c.id(), "gasto", "10.00");
    let id = &d.movimientos[0].id;
    let n = r.obtener_eventos().unwrap().len();
    for monto in ["", "abc", "-1", "10.555", "10.00"] {
        assert_eq!(
            ejecutar_corregir_movimiento(&mut r, c.id(), id, monto, "prueba")
                .unwrap_err()
                .campo,
            "monto"
        );
    }
    for (cuenta, mov, campo) in [
        ("inexistente", id.as_str(), "cuenta"),
        (otra.id(), id.as_str(), "movimiento"),
        (c.id(), "inexistente", "movimiento"),
    ] {
        assert_eq!(
            ejecutar_corregir_movimiento(&mut r, cuenta, mov, "1", "prueba")
                .unwrap_err()
                .campo,
            campo
        );
    }
    assert_eq!(
        ejecutar_crear_categoria(&mut r, " ", "gasto", "prueba")
            .unwrap_err()
            .campo,
        "categoria"
    );
    assert_eq!(
        ejecutar_crear_categoria(&mut r, "X", "otro", "prueba")
            .unwrap_err()
            .campo,
        "tipo"
    );
    assert_eq!(
        ejecutar_crear_categoria(&mut r, "ALIMENTACIÓN", "gasto", "prueba")
            .unwrap_err()
            .campo,
        "categoria"
    );
    assert_eq!(r.obtener_eventos().unwrap().len(), n);
}

#[test]
fn inv01_inv03_inv18_cantidad_moneda_exactas_y_sin_mutadores() {
    let usd = Monto::desde_str("0.10", "USD").unwrap();
    let total = usd
        .sumar(&Monto::desde_str("0.20", "USD").unwrap())
        .unwrap();
    assert_eq!(total.cantidad().to_string(), "0.30");
    assert_eq!(total.moneda(), "USD");
    assert!(
        usd.sumar(&Monto::desde_str("0.20", "VES").unwrap())
            .is_err()
    );
    let mut r = RepositorioEnMemoria::default();
    let c = ejecutar_crear_cuenta(&mut r, "Cuenta", "banco", "USD", "10", "prueba").unwrap();
    registrar(&mut r, c.id(), "ingreso", "0.10");
    let actual = ejecutar_listar_cuentas(&r).unwrap();
    assert_eq!(actual.cuentas[0].moneda(), "USD");
    assert_eq!(actual.cuentas[0].saldo().moneda(), "USD");
    assert_eq!(c.tipo(), &TipoCuenta::Banco);
}

#[test]
fn fechas_movimiento_ordenadas_con_desempate_estable() {
    let mut r = RepositorioEnMemoria::default();
    let c = ejecutar_crear_cuenta(&mut r, "Cuenta", "banco", "USD", "100", "prueba").unwrap();
    for (fecha, monto) in [
        ("2020-01-20", "1"),
        ("2020-01-10", "2"),
        ("2020-01-20", "3"),
    ] {
        ejecutar_registrar_movimiento(
            &mut r,
            RegistrarMovimiento {
                cuenta_id: c.id(),
                tipo_str: "gasto",
                monto_str: monto,
                fecha_str: fecha,
                categoria_str: "Alimentación",
                nota_opt: None,
                dispositivo_id: "prueba",
            },
        )
        .unwrap();
    }
    let d = ejecutar_obtener_detalle_cuenta(&r, c.id()).unwrap();
    assert_eq!(
        d.movimientos
            .iter()
            .map(|m| (m.fecha.as_str(), m.monto.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("2020-01-20", "-3.00"),
            ("2020-01-20", "-1.00"),
            ("2020-01-10", "-2.00")
        ]
    );
}

#[test]
fn advertencia_decimal_exacta_incluso_mas_alla_de_number() {
    let mut r = RepositorioEnMemoria::default();
    let c = ejecutar_crear_cuenta(
        &mut r,
        "Cuenta",
        "banco",
        "USD",
        "9007199254740992.00",
        "prueba",
    )
    .unwrap();
    let e = evaluar_movimiento(&r, c.id(), "gasto", "9007199254740992.01").unwrap();
    assert!(e.deja_saldo_negativo);
    assert_eq!(e.saldo_resultante, "-0.01");
    assert!(
        !evaluar_movimiento(&r, c.id(), "gasto", "9007199254740992.00")
            .unwrap()
            .deja_saldo_negativo
    );
    assert_eq!(r.obtener_eventos().unwrap().len(), 1);
    assert_eq!(
        registrar(&mut r, c.id(), "gasto", "9007199254740992.01").saldo_actual,
        "-0.01"
    );
}

#[test]
fn eventos_incompatibles_y_referencias_invalidas_producen_error_explicito() {
    for (tipo, datos) in [
        ("MovimientoRegistrado", json!({"monto":"10"})),
        (
            "MovimientoCorregido",
            json!({"ajuste_id":"a","movimiento_original_id":"inexistente","cuenta_id":"inexistente","monto_ajuste":"1"}),
        ),
        ("Desconocido", json!({})),
    ] {
        let mut r = RepositorioEnMemoria::default();
        r.guardar_evento(&evento(tipo, datos)).unwrap();
        assert_eq!(
            ejecutar_listar_cuentas(&r).unwrap_err().campo,
            "almacenamiento"
        );
    }
}

#[test]
fn creacion_rechaza_texto_vacio_invalido_y_fuera_de_rango() {
    let mut r = RepositorioEnMemoria::default();
    for valor in ["", "abc", "99999999999999999999999999999999999999"] {
        assert_eq!(
            ejecutar_crear_cuenta(&mut r, "Cuenta", "banco", "USD", valor, "prueba")
                .unwrap_err()
                .campo,
            "saldo_inicial"
        );
    }
    assert!(r.obtener_eventos().unwrap().is_empty());
}

#[test]
fn fecha_civil_caracas_rechaza_manana_aunque_utc_ya_cambio_de_dia() {
    let instante = chrono::DateTime::parse_from_rfc3339("2026-10-24T01:00:00Z").unwrap();
    let caracas = chrono::FixedOffset::west_opt(4 * 60 * 60).unwrap();
    let hoy_local = instante.with_timezone(&caracas).date_naive();
    assert!(validar_fecha_movimiento("2026-10-24", hoy_local).is_err());
    assert!(validar_fecha_movimiento("2026-10-23", hoy_local).is_ok());
    assert!(validar_fecha_movimiento("2026-10-22", hoy_local).is_ok());
}

#[test]
fn inv13_reproyectar_eventos_duplicados_no_duplica_movimientos_ni_ajustes() {
    struct LecturaDuplicada(Vec<EventoLibro>);
    impl RepositorioEventos for LecturaDuplicada {
        fn guardar_evento(&mut self, _: &EventoLibro) -> Result<(), String> {
            Err("Solo lectura".into())
        }
        fn obtener_eventos(&self) -> Result<Vec<EventoLibro>, String> {
            Ok(self.0.clone())
        }
    }
    let mut r = RepositorioEnMemoria::default();
    let c = ejecutar_crear_cuenta(&mut r, "Cuenta", "banco", "USD", "1000.00", "prueba").unwrap();
    registrar(&mut r, c.id(), "ingreso", "800.00");
    let d = registrar(&mut r, c.id(), "gasto", "500.00");
    let id = d.movimientos[0].id.clone();
    ejecutar_corregir_movimiento(&mut r, c.id(), &id, "50.00", "prueba").unwrap();
    let eventos = r.obtener_eventos().unwrap();
    let mut repetidos = eventos.clone();
    repetidos.extend(eventos);
    let d = ejecutar_obtener_detalle_cuenta(&LecturaDuplicada(repetidos.clone()), c.id()).unwrap();
    assert_eq!(d.saldo_actual, "1750.00");
    assert_eq!(d.movimientos.len(), 3);
    repetidos.last_mut().unwrap().datos_json = "{}".into();
    assert!(ejecutar_listar_cuentas(&LecturaDuplicada(repetidos)).is_err());
}

#[test]
fn desbordamiento_no_agrega_eventos_ni_deja_saldos_parciales() {
    let mut r = RepositorioEnMemoria::default();
    let c = ejecutar_crear_cuenta(
        &mut r,
        "Cuenta",
        "banco",
        "USD",
        &Decimal::MAX.to_string(),
        "prueba",
    )
    .unwrap();
    let n = r.obtener_eventos().unwrap().len();
    assert!(evaluar_movimiento(&r, c.id(), "ingreso", "1").is_err());
    assert!(
        ejecutar_registrar_movimiento(
            &mut r,
            RegistrarMovimiento {
                cuenta_id: c.id(),
                tipo_str: "ingreso",
                monto_str: "1",
                fecha_str: &Local::now().date_naive().to_string(),
                categoria_str: "Salario",
                nota_opt: None,
                dispositivo_id: "prueba"
            }
        )
        .is_err()
    );
    assert_eq!(r.obtener_eventos().unwrap().len(), n);
}

#[test]
fn id_reutilizado_con_otro_contenido_es_error_en_memoria_y_sqlite() {
    fn verificar<R: RepositorioEventos>(r: &mut R) {
        ejecutar_crear_cuenta(r, "Cuenta", "banco", "USD", "100.00", "prueba").unwrap();
        let evento = r.obtener_eventos().unwrap()[0].clone();
        r.guardar_evento(&evento).unwrap();
        let mut alterado = evento.clone();
        alterado.datos_json = "{}".into();
        assert!(r.guardar_evento(&alterado).is_err());
        assert_eq!(r.obtener_eventos().unwrap(), vec![evento]);
    }
    verificar(&mut RepositorioEnMemoria::default());
    let path = std::env::temp_dir().join(format!("cc-id-{}.sqlite", uuid::Uuid::new_v4()));
    let mut r = RepositorioSqlite::inicializar(path.clone()).unwrap();
    verificar(&mut r);
    drop(r);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn ajustes_historicos_de_ingresos_se_encadenan_y_admiten_correccion_actual() {
    let mut r = RepositorioEnMemoria::default();
    let c = ejecutar_crear_cuenta(&mut r, "Cuenta", "banco", "USD", "1000.00", "prueba").unwrap();
    let id = movimiento_historico(&mut r, c.id(), "ingreso", "800.00");
    for delta in ["-100.00", "50.00"] {
        r.guardar_evento(&evento(
            "MovimientoCorregido",
            json!({
                "ajuste_id":uuid::Uuid::new_v4().to_string(),"movimiento_original_id":id,
                "cuenta_id":c.id(),"monto_ajuste":delta
            }),
        ))
        .unwrap();
    }
    let d = ejecutar_obtener_detalle_cuenta(&r, c.id()).unwrap();
    assert_eq!(d.saldo_actual, "1750.00");
    let orig = d.movimientos.iter().find(|m| m.id == id).unwrap();
    assert_eq!(orig.monto_vigente, "750.00");
    assert_eq!(orig.monto, "800.00");
    assert_eq!(
        ejecutar_corregir_movimiento(&mut r, c.id(), &id, "0", "prueba")
            .unwrap()
            .saldo_actual,
        "1000.00"
    );
}

#[test]
fn subtotal_fuera_de_rango_se_rechaza_antes_de_guardar() {
    let mut r = RepositorioEnMemoria::default();
    ejecutar_crear_cuenta(
        &mut r,
        "Máximo",
        "banco",
        "USD",
        &Decimal::MAX.to_string(),
        "prueba",
    )
    .unwrap();
    let c = ejecutar_crear_cuenta(&mut r, "Cero", "efectivo", "USD", "0", "prueba").unwrap();
    let antes = r.obtener_eventos().unwrap();
    assert!(ejecutar_crear_cuenta(&mut r, "Otra", "banco", "USD", "1", "prueba").is_err());
    assert!(
        ejecutar_registrar_movimiento(
            &mut r,
            RegistrarMovimiento {
                cuenta_id: c.id(),
                tipo_str: "ingreso",
                monto_str: "1",
                fecha_str: &Local::now().date_naive().to_string(),
                categoria_str: "Salario",
                nota_opt: None,
                dispositivo_id: "prueba"
            }
        )
        .is_err()
    );
    assert_eq!(r.obtener_eventos().unwrap(), antes);
    assert_eq!(
        ejecutar_listar_cuentas(&r).unwrap().subtotales[0].subtotal,
        format!("{:.2}", Decimal::MAX)
    );
}

#[test]
fn cambio_identificador_copia_wal_sin_alterar_original_ni_sobrescribir_destino() {
    let dir = std::env::temp_dir().join(format!("cc-migracion-{}", uuid::Uuid::new_v4()));
    let anterior = dir.join("com.tauri.dev/cuentas_claras.sqlite");
    let nueva = dir.join("com.compupapis.cuentasclaras/cuentas_claras.sqlite");
    let mut r = RepositorioSqlite::inicializar(anterior.clone()).unwrap();
    let wal = rusqlite::Connection::open(&anterior).unwrap();
    wal.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;")
        .unwrap();
    let c =
        ejecutar_crear_cuenta(&mut r, "Histórica", "banco", "USD", "1000.00", "prueba").unwrap();
    movimiento_historico(&mut r, c.id(), "ingreso", "800.00");
    let id = movimiento_historico(&mut r, c.id(), "gasto", "500.00");
    ejecutar_corregir_movimiento(&mut r, c.id(), &id, "50.00", "prueba").unwrap();
    let originales = r.obtener_eventos().unwrap();
    crate::sqlite_repo::migrar_libro_anterior(&anterior, &nueva).unwrap();
    let mut destino = RepositorioSqlite::inicializar(nueva.clone()).unwrap();
    assert_eq!(destino.obtener_eventos().unwrap(), originales);
    assert_eq!(
        ejecutar_obtener_detalle_cuenta(&destino, c.id())
            .unwrap()
            .saldo_actual,
        "1750.00"
    );
    assert_eq!(r.obtener_eventos().unwrap(), originales);
    registrar(&mut destino, c.id(), "ingreso", "1.00");
    crate::sqlite_repo::migrar_libro_anterior(&anterior, &nueva).unwrap();
    assert_eq!(
        ejecutar_obtener_detalle_cuenta(&destino, c.id())
            .unwrap()
            .saldo_actual,
        "1751.00"
    );
    assert_eq!(r.obtener_eventos().unwrap(), originales);
    drop(destino);
    drop(r);
    drop(wal);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn migracion_incompatible_no_publica_destino_y_conserva_origen() {
    let dir = std::env::temp_dir().join(format!("cc-migracion-invalida-{}", uuid::Uuid::new_v4()));
    let anterior = dir.join("anterior.sqlite");
    let nueva = dir.join("nueva.sqlite");
    let mut r = RepositorioSqlite::inicializar(anterior.clone()).unwrap();
    let e = evento("MovimientoRegistrado", json!({"incompatible":true}));
    r.guardar_evento(&e).unwrap();
    assert!(crate::sqlite_repo::migrar_libro_anterior(&anterior, &nueva).is_err());
    assert!(!nueva.exists());
    assert_eq!(r.obtener_eventos().unwrap(), vec![e]);
    drop(r);
    std::fs::remove_dir_all(dir).unwrap();
}
