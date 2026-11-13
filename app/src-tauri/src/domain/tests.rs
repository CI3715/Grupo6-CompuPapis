use super::*;
use crate::pruebas_tasas_monedas::{aviso_faltante, consultar_fixture_en_fecha};

#[test]
fn test_ejemplo_seccion_2_2_correccion_e_inv_04_inv_05() {
    let mut repo = RepositorioEnMemoria::default();
    let hoy = chrono::Local::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();

    // Paso 1: Cuenta "Billetera dólares" en USD con saldo inicial 1.000,00
    let cta = ejecutar_crear_cuenta(
        &mut repo,
        "Billetera dólares",
        "billetera digital",
        "USD",
        "1000.00",
        "dev-1",
    )
    .unwrap();
    assert_eq!(cta.saldo().cantidad().to_string(), "1000.00");

    // Paso 2: Ingreso Salario +800,00 -> Saldo 1.800,00
    let det = ejecutar_registrar_movimiento(
        &mut repo,
        RegistrarMovimiento {
            cuenta_id: cta.id(),
            tipo_str: "ingreso",
            monto_str: "800.00",
            fecha_str: &hoy,
            categoria_str: "Salario",
            nota_opt: None,
            dispositivo_id: "dev-1",
        },
    )
    .unwrap();
    assert_eq!(det.saldo_actual, "1800.00");

    // Paso 3: Gasto Alimentación (era 50, se anotó 500) -> -500,00 -> Saldo 1.300,00
    let det = ejecutar_registrar_movimiento(
        &mut repo,
        RegistrarMovimiento {
            cuenta_id: cta.id(),
            tipo_str: "gasto",
            monto_str: "500.00",
            fecha_str: &hoy,
            categoria_str: "Alimentación",
            nota_opt: Some("Compra mercado".into()),
            dispositivo_id: "dev-1",
        },
    )
    .unwrap();
    assert_eq!(det.saldo_actual, "1300.00");
    let id_gasto_erroneo = det.movimientos[0].id.clone();

    // Paso 4: Ajuste del paso 3 (monto correcto: 50,00) -> +450,00 -> Saldo 1.750,00
    let det =
        ejecutar_corregir_movimiento(&mut repo, cta.id(), &id_gasto_erroneo, "50.00", "dev-1")
            .unwrap();
    assert_eq!(det.saldo_actual, "1750.00");
    assert_eq!(det.composicion.inicial, "1000.00");
    assert_eq!(det.composicion.ingresos, "800.00");
    assert_eq!(det.composicion.gastos, "500.00");
    assert_eq!(det.composicion.ajustes, "450.00");

    // INV-04: El movimiento original sigue en el historial marcado como corregido y existe el ajuste que lo referencia
    assert_eq!(det.movimientos.len(), 3);
    let mov_ajuste = &det.movimientos[0];
    let mov_original = det
        .movimientos
        .iter()
        .find(|m| m.id == id_gasto_erroneo)
        .unwrap();
    assert!(mov_original.corregido);
    assert!(mov_ajuste.es_ajuste);
    assert_eq!(
        mov_ajuste.referencia_id.as_deref(),
        Some(id_gasto_erroneo.as_str())
    );
    assert_eq!(mov_ajuste.monto, "450.00");
}

#[test]
fn test_validaciones_registrar_movimiento() {
    let mut repo = RepositorioEnMemoria::default();
    let hoy = chrono::Local::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    let cta =
        ejecutar_crear_cuenta(&mut repo, "Banco USD", "banco", "USD", "100.00", "dev-1").unwrap();

    // 1. Monto cero o negativo -> falla campo "monto"
    let err_cero = ejecutar_registrar_movimiento(
        &mut repo,
        RegistrarMovimiento {
            cuenta_id: cta.id(),
            tipo_str: "gasto",
            monto_str: "0",
            fecha_str: &hoy,
            categoria_str: "Alimentación",
            nota_opt: None,
            dispositivo_id: "dev-1",
        },
    )
    .unwrap_err();
    assert_eq!(err_cero.campo, "monto");

    let err_neg = ejecutar_registrar_movimiento(
        &mut repo,
        RegistrarMovimiento {
            cuenta_id: cta.id(),
            tipo_str: "ingreso",
            monto_str: "-10.00",
            fecha_str: &hoy,
            categoria_str: "Salario",
            nota_opt: None,
            dispositivo_id: "dev-1",
        },
    )
    .unwrap_err();
    assert_eq!(err_neg.campo, "monto");

    // 2. Más decimales que la moneda (10.555 USD) -> falla campo "monto"
    let err_dec = ejecutar_registrar_movimiento(
        &mut repo,
        RegistrarMovimiento {
            cuenta_id: cta.id(),
            tipo_str: "gasto",
            monto_str: "10.555",
            fecha_str: &hoy,
            categoria_str: "Alimentación",
            nota_opt: None,
            dispositivo_id: "dev-1",
        },
    )
    .unwrap_err();
    assert_eq!(err_dec.campo, "monto");

    // 3. Categoría vacía, inexistente o de otro tipo -> falla campo "categoria"
    let err_cat_vacia = ejecutar_registrar_movimiento(
        &mut repo,
        RegistrarMovimiento {
            cuenta_id: cta.id(),
            tipo_str: "gasto",
            monto_str: "10.00",
            fecha_str: &hoy,
            categoria_str: "  ",
            nota_opt: None,
            dispositivo_id: "dev-1",
        },
    )
    .unwrap_err();
    assert_eq!(err_cat_vacia.campo, "categoria");

    let err_cat_inex = ejecutar_registrar_movimiento(
        &mut repo,
        RegistrarMovimiento {
            cuenta_id: cta.id(),
            tipo_str: "gasto",
            monto_str: "10.00",
            fecha_str: &hoy,
            categoria_str: "Inexistente",
            nota_opt: None,
            dispositivo_id: "dev-1",
        },
    )
    .unwrap_err();
    assert_eq!(err_cat_inex.campo, "categoria");

    let err_cat_cruzada = ejecutar_registrar_movimiento(
        &mut repo,
        RegistrarMovimiento {
            cuenta_id: cta.id(),
            tipo_str: "gasto",
            monto_str: "10.00",
            fecha_str: &hoy,
            categoria_str: "Salario",
            nota_opt: None,
            dispositivo_id: "dev-1",
        },
    )
    .unwrap_err();
    assert_eq!(err_cat_cruzada.campo, "categoria");

    // 4. Fecha futura -> falla campo "fecha"
    let err_futura = ejecutar_registrar_movimiento(
        &mut repo,
        RegistrarMovimiento {
            cuenta_id: cta.id(),
            tipo_str: "gasto",
            monto_str: "10.00",
            fecha_str: "2099-12-31",
            categoria_str: "Alimentación",
            nota_opt: None,
            dispositivo_id: "dev-1",
        },
    )
    .unwrap_err();
    assert_eq!(err_futura.campo, "fecha");
}

#[test]
fn test_validaciones_corregir_movimiento_y_compensacion_total() {
    let mut repo = RepositorioEnMemoria::default();
    let hoy = chrono::Local::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    let cta = ejecutar_crear_cuenta(
        &mut repo,
        "Efectivo VES",
        "efectivo",
        "VES",
        "500.00",
        "dev-1",
    )
    .unwrap();

    let det = ejecutar_registrar_movimiento(
        &mut repo,
        RegistrarMovimiento {
            cuenta_id: cta.id(),
            tipo_str: "gasto",
            monto_str: "200.00",
            fecha_str: &hoy,
            categoria_str: "Transporte",
            nota_opt: None,
            dispositivo_id: "dev-1",
        },
    )
    .unwrap();
    assert_eq!(det.saldo_actual, "300.00");
    let id_gasto = det.movimientos[0].id.clone();

    // Monto negativo al corregir es rechazado
    let err_neg = ejecutar_corregir_movimiento(&mut repo, cta.id(), &id_gasto, "-10.00", "dev-1")
        .unwrap_err();
    assert_eq!(err_neg.campo, "monto");

    // Compensar por completo con monto correcto = 0
    let det_cero =
        ejecutar_corregir_movimiento(&mut repo, cta.id(), &id_gasto, "0.00", "dev-1").unwrap();
    assert_eq!(det_cero.saldo_actual, "500.00");
    let id_ajuste = det_cero.movimientos[0].id.clone();

    // Intentar corregir un ajuste directamente es rechazado
    let err_ajuste =
        ejecutar_corregir_movimiento(&mut repo, cta.id(), &id_ajuste, "50.00", "dev-1")
            .unwrap_err();
    assert_eq!(err_ajuste.campo, "movimiento");

    // Corregir el movimiento original por segunda vez (de 0.00 vigente a 80.00)
    let det_segunda =
        ejecutar_corregir_movimiento(&mut repo, cta.id(), &id_gasto, "80.00", "dev-1").unwrap();
    assert_eq!(det_segunda.saldo_actual, "420.00");
}

// Verifica la selección de tasa vigente, desempate diario, inversa y par faltante
#[test]
fn test_seleccion_tasa_vigente_inversa_y_par_faltante() {
    let tasas = vec![
        TasaCambio {
            id: "t1".into(),
            origen: "USD".into(),
            destino: "VES".into(),
            valor: Decimal::from_str("880.00").unwrap(),
            fecha: "2026-10-19".into(),
            fuente: "BCV".into(),
        },
        TasaCambio {
            id: "t2".into(),
            origen: "USD".into(),
            destino: "VES".into(),
            valor: Decimal::from_str("885.00").unwrap(),
            fecha: "2026-10-20".into(),
            fuente: "BCV mañana".into(),
        },
        TasaCambio {
            id: "t3".into(),
            origen: "USD".into(),
            destino: "VES".into(),
            valor: Decimal::from_str("887.00").unwrap(),
            fecha: "2026-10-20".into(),
            fuente: "BCV tarde".into(),
        },
        TasaCambio {
            id: "t4".into(),
            origen: "USDT".into(),
            destino: "VES".into(),
            valor: Decimal::from_str("1000.00").unwrap(),
            fecha: "2026-10-21".into(),
            fuente: "casa de cambio".into(),
        },
    ];

    let fecha_consulta = NaiveDate::from_ymd_opt(2026, 10, 23).unwrap();

    // Selecciona la última tasa directa registrada del día más reciente
    let directa = buscar_tasa_aplicable("USD", "VES", fecha_consulta, &tasas).unwrap();
    assert!(!directa.inversa);
    assert_eq!(
        directa.tasa.as_ref().unwrap().valor,
        Decimal::from_str("887.00").unwrap()
    );
    assert_eq!(directa.tasa.as_ref().unwrap().fuente, "BCV tarde");

    // Aplica la inversa exacta de VES a USD
    let (en_usd, vista_inv, inversa) = convertir_monto_a_base(
        Decimal::from_str("88700.00").unwrap(),
        "VES",
        "USD",
        fecha_consulta,
        &tasas,
    )
    .unwrap();
    assert!(inversa);
    assert_eq!(vista_inv.unwrap().id, "t3");
    assert_eq!(en_usd.round_dp(2), Decimal::from_str("100.00").unwrap());

    // Rechaza tasas cruzadas (USDT -> VES -> USD) e informa el par faltante
    let err_faltante = buscar_tasa_aplicable("USDT", "USD", fecha_consulta, &tasas).unwrap_err();
    assert_eq!(err_faltante.campo, "tasa");
    assert_eq!(
        err_faltante.mensaje,
        "Falta una tasa de USDT a USD con fecha igual o anterior al 23/10/2026"
    );
}

// Verifica el ejemplo de la sección 2.3, aviso de tasa faltante y redondeo único
#[test]
fn test_consultar_patrimonio_seccion_2_3_y_redondeo_final() {
    let mut repo = RepositorioEnMemoria::default();
    let fecha = NaiveDate::from_ymd_opt(2026, 10, 23).unwrap();
    // Crea las tres cuentas del ejemplo de la especificación
    ejecutar_crear_cuenta(
        &mut repo,
        "Efectivo bolívares",
        "efectivo",
        "VES",
        "88700.00",
        "d1",
    )
    .unwrap();
    ejecutar_crear_cuenta(
        &mut repo,
        "Banco en dólares",
        "banco",
        "USD",
        "200.00",
        "d1",
    )
    .unwrap();
    ejecutar_crear_cuenta(
        &mut repo,
        "Billetera USDT",
        "billetera digital",
        "USDT",
        "50.00",
        "d1",
    )
    .unwrap();

    // Registra tasas de USD->VES y USDT->VES con sus fechas y fuentes
    let t_usd = TasaCambio {
        id: "t-usd-ves".into(),
        origen: "USD".into(),
        destino: "VES".into(),
        valor: Decimal::from_str("887.00").unwrap(),
        fecha: "2026-10-20".into(),
        fuente: "BCV".into(),
    };
    repo.guardar_evento(&EventoLibro {
        id: "t-usd-ves".into(),
        tipo: "TasaRegistrada".into(),
        momento_registro: Utc::now(),
        dispositivo_origen: "d1".into(),
        datos_json: serde_json::to_string(&t_usd).unwrap(),
    })
    .unwrap();

    let t_usdt = TasaCambio {
        id: "t-usdt-ves".into(),
        origen: "USDT".into(),
        destino: "VES".into(),
        valor: Decimal::from_str("1000.00").unwrap(),
        fecha: "2026-10-21".into(),
        fuente: "casa de cambio".into(),
    };
    repo.guardar_evento(&EventoLibro {
        id: "t-usdt-ves".into(),
        tipo: "TasaRegistrada".into(),
        momento_registro: Utc::now(),
        dispositivo_origen: "d1".into(),
        datos_json: serde_json::to_string(&t_usdt).unwrap(),
    })
    .unwrap();

    // Cambia la moneda base a VES y verifica el total de 316100.00 VES
    repo.guardar_evento(&EventoLibro {
        id: "base-ves".into(),
        tipo: "MonedaBaseCambiada".into(),
        momento_registro: Utc::now(),
        dispositivo_origen: "d1".into(),
        datos_json: serde_json::to_string(&DatosMonedaBaseCambiada {
            codigo: "VES".into(),
        })
        .unwrap(),
    })
    .unwrap();

    let pat_ves = consultar_fixture_en_fecha(&repo, fecha).unwrap();
    assert_eq!(pat_ves.total.as_deref(), Some("316100.00"));
    assert!(pat_ves.faltantes.is_empty());

    // Cambia la base a USD; falta USDT->USD, omite total y muestra aviso
    repo.guardar_evento(&EventoLibro {
        id: "base-usd".into(),
        tipo: "MonedaBaseCambiada".into(),
        momento_registro: Utc::now(),
        dispositivo_origen: "d1".into(),
        datos_json: serde_json::to_string(&DatosMonedaBaseCambiada {
            codigo: "USD".into(),
        })
        .unwrap(),
    })
    .unwrap();

    let pat_usd_incompleto = consultar_fixture_en_fecha(&repo, fecha).unwrap();
    assert!(pat_usd_incompleto.total.is_none());
    assert_eq!(
        aviso_faltante(&pat_usd_incompleto).as_deref(),
        Some("Falta una tasa de USDT a USD con fecha igual o anterior al 23/10/2026")
    );
    assert_eq!(pat_usd_incompleto.subtotales.len(), 3);

    // Registra 1 USDT = 1.00 USD y verifica que el total pase a 350.00 USD
    let t_usdt_usd = TasaCambio {
        id: "t-usdt-usd".into(),
        origen: "USDT".into(),
        destino: "USD".into(),
        valor: Decimal::from_str("1.00").unwrap(),
        fecha: "2026-10-23".into(),
        fuente: "casa de cambio".into(),
    };
    repo.guardar_evento(&EventoLibro {
        id: "t-usdt-usd".into(),
        tipo: "TasaRegistrada".into(),
        momento_registro: Utc::now(),
        dispositivo_origen: "d1".into(),
        datos_json: serde_json::to_string(&t_usdt_usd).unwrap(),
    })
    .unwrap();

    let pat_usd_completo = consultar_fixture_en_fecha(&repo, fecha).unwrap();
    assert_eq!(pat_usd_completo.total.as_deref(), Some("350.00"));
    assert!(pat_usd_completo.faltantes.is_empty());
}

#[test]
fn probar_crear_cuentas() {
    let mut repo = RepositorioEnMemoria::default();

    // Creamos una cuenta
    let cuenta =
        ejecutar_crear_cuenta(&mut repo, "Cartera", "efectivo", "VES", "100.25", "prueba").unwrap();

    // Vemos si los datos estan bien
    assert_eq!(cuenta.moneda(), "VES");
    assert_eq!(cuenta.saldo().moneda(), "VES");
    assert_eq!(cuenta.saldo().cantidad().to_string(), "100.25");

    // Revisamos los casos que deben dar error
    let casos = [
        ("", "efectivo", "VES", "100", "nombre"),
        ("   ", "efectivo", "VES", "100", "nombre"),
        ("CARTERA", "efectivo", "VES", "100", "nombre"),
        ("Otra", "tarjeta", "VES", "100", "tipo"),
        ("Otra", "efectivo", "ABC", "100", "moneda"),
        ("Otra", "efectivo", "VES", "-10", "saldo_inicial"),
        ("Otra", "efectivo", "VES", "10.555", "saldo_inicial"),
    ];

    for (nombre, tipo, moneda, saldo, campo) in casos {
        let resp = ejecutar_crear_cuenta(&mut repo, nombre, tipo, moneda, saldo, "prueba");

        assert!(resp.is_err());
        assert_eq!(resp.unwrap_err().campo, campo);
    }

    // Solo se debe guardar la primera cuenta
    assert_eq!(repo.obtener_eventos().unwrap().len(), 1);

    // Creamos las otras cuentas
    ejecutar_crear_cuenta(&mut repo, "Banco", "banco", "USD", "0.10", "prueba").unwrap();

    ejecutar_crear_cuenta(&mut repo, "Ahorros", "banco", "USD", "0.20", "prueba").unwrap();

    ejecutar_crear_cuenta(
        &mut repo,
        "Billetera",
        "billetera digital",
        "USDT",
        "0",
        "prueba",
    )
    .unwrap();

    // Vemos las cuentas y los subtotales
    let lista = ejecutar_listar_cuentas(&repo).unwrap();

    assert_eq!(lista.cuentas.len(), 4);
    assert_eq!(lista.subtotales.len(), 3);

    let usd = lista
        .subtotales
        .iter()
        .find(|dato| dato.codigo == "USD")
        .unwrap();

    assert_eq!(usd.subtotal, "0.30");

    // Vemos si la cuenta sigue usando VES
    let cartera = lista
        .cuentas
        .iter()
        .find(|c| c.nombre() == "Cartera")
        .unwrap();

    assert_eq!(cartera.moneda(), "VES");
    assert_eq!(cartera.saldo().moneda(), "VES");

    // Agregamos el mismo evento otra vez
    let eventos = repo.obtener_eventos().unwrap();
    let mut repetidos = eventos.clone();
    repetidos.push(eventos[0].clone());

    assert_eq!(proyectar_libro(&repetidos).unwrap().cuentas.len(), 4);
}
