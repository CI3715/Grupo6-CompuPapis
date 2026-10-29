// Importa casos de uso y estructuras del dominio para pruebas en memoria
use crate::domain::{
    RegistrarMovimiento, RepositorioEnMemoria, ejecutar_corregir_movimiento, ejecutar_crear_cuenta,
    ejecutar_listar_cuentas, ejecutar_listar_monedas, ejecutar_obtener_detalle_cuenta,
    ejecutar_registrar_movimiento, evaluar_movimiento, ejecutar_obtener_moneda_base,
};

// Importa casos de uso de escritura para monedas propias y moneda base
use crate::tasas_monedas::{ejecutar_cambiar_moneda_base, ejecutar_crear_moneda};

// Verifica creación de moneda propia de 3 decimales y apertura de cuenta
#[test]
fn probar_cuenta_con_moneda_propia() {
    let mut repo = RepositorioEnMemoria::default();

    // Registra una moneda propia con escale personalizada de tres decimales
    let moneda = ejecutar_crear_moneda(
        &mut repo,
        "ABC",
        "Moneda de prueba",
        "A",
        3,
        "prueba",
        &crate::RelojSistema,
        &crate::GeneradorUuid,
    )
    .unwrap();

    assert_eq!(moneda.codigo, "ABC");

    // Comprueba que el catálogo incluya las cuatro de fábrica más la nueva
    let monedas = ejecutar_listar_monedas(&repo).unwrap();
    assert_eq!(monedas.len(), 5);

    // Abre una cuenta usando la moneda recién creada con tres decimales
    let cuenta = ejecutar_crear_cuenta(
        &mut repo,
        "Cuenta de prueba",
        "efectivo",
        "ABC",
        "12.345",
        "prueba",
    )
    .unwrap();

    assert_eq!(cuenta.moneda(), "ABC");
    assert_eq!(cuenta.saldo().cantidad().to_string(), "12.345");

    // Verifica que el listado y subtotal respecten la escala de tres decimales
    let lista = ejecutar_listar_cuentas(&repo).unwrap();

    assert_eq!(lista.cuentas.len(), 1);
    assert_eq!(lista.cuentas[0].moneda(), "ABC");
    assert_eq!(lista.subtotales.len(), 1);
    assert_eq!(lista.subtotales[0].codigo, "ABC");
    assert_eq!(lista.subtotales[0].subtotal, "12.345");
}

// Comprueba exactitud de ingresos, gastos, ajustes y subtotales con tres decimales
#[test]
fn probar_movimientos_moneda_propia() {
    let mut repo = RepositorioEnMemoria::default();
    let fecha = "2026-10-20";

    // Crea la moneda propia y la cuenta inicial para operar movimientos
    ejecutar_crear_moneda(
        &mut repo,
        "ABC",
        "Moneda de prueba",
        "A",
        3,
        "prueba",
        &crate::RelojSistema,
        &crate::GeneradorUuid,
    )
    .unwrap();

    let cuenta = ejecutar_crear_cuenta(
        &mut repo,
        "Cuenta ABC",
        "efectivo",
        "ABC",
        "12.345",
        "prueba",
    )
    .unwrap();

    // Registra un ingreso exacto de tres decimales y verifica el saldo
    let resp = ejecutar_registrar_movimiento(
        &mut repo,
        RegistrarMovimiento {
            cuenta_id: cuenta.id(),
            tipo_str: "ingreso",
            monto_str: "2.105",
            fecha_str: fecha,
            categoria_str: "Salario",
            nota_opt: None,
            dispositivo_id: "prueba",
        },
    )
    .unwrap();

    assert_eq!(resp.saldo_actual, "14.450");

    // Registra un gasto con tres decimales y comprueba la resta exacta
    let resp = ejecutar_registrar_movimiento(
        &mut repo,
        RegistrarMovimiento {
            cuenta_id: cuenta.id(),
            tipo_str: "gasto",
            monto_str: "0.050",
            fecha_str: fecha,
            categoria_str: "Alimentación",
            nota_opt: None,
            dispositivo_id: "prueba",
        },
    )
    .unwrap();

    assert_eq!(resp.saldo_actual, "14.400");

    // Corrige el gasto anterior mediante un ajuste y verifica el nuevo saldo
    let mov_id = resp.movimientos[0].id.clone();

    let resp =
        ejecutar_corregir_movimiento(&mut repo, cuenta.id(), &mov_id, "0.025", "prueba").unwrap();

    assert_eq!(resp.saldo_actual, "14.425");

    // Comprueba que el detalle conserve la composición exacta con tres decimales
    let detalle = ejecutar_obtener_detalle_cuenta(&repo, cuenta.id()).unwrap();

    assert_eq!(detalle.saldo_actual, "14.425");
    assert_eq!(detalle.composicion.ingresos, "2.105");
    assert_eq!(detalle.composicion.gastos, "0.050");
    assert_eq!(detalle.composicion.ajustes, "0.025");

    // Evalúa un gasto de un milésimo y verifica el subtotal acumulado
    let resp = evaluar_movimiento(&repo, cuenta.id(), "gasto", "0.001").unwrap();

    assert_eq!(resp.saldo_resultante, "14.424");

    let lista = ejecutar_listar_cuentas(&repo).unwrap();

    assert_eq!(lista.subtotales[0].subtotal, "14.425");
}

// Verifica rechazo de códigos inválidos, duplicados y monedas de fábrica
#[test]
fn probar_validaciones_moneda_propia() {
    let mut repo = RepositorioEnMemoria::default();

    // Registra una moneda propia válida para probar el rechazo de duplicados
    ejecutar_crear_moneda(
        &mut repo,
        "CLP",
        "Peso chileno",
        "$",
        0,
        "prueba",
        &crate::RelojSistema,
        &crate::GeneradorUuid,
    )
    .unwrap();

    // Protege las cuatro monedas de fábrica contra intentos de recreación o sobreescritura
    for codigo_fabrica in ["VES", "USD", "EUR", "USDT"] {
        let err = ejecutar_crear_moneda(
            &mut repo,
            codigo_fabrica,
            "Moneda repetida",
            "$",
            2,
            "prueba",
            &crate::RelojSistema,
            &crate::GeneradorUuid,
        )
        .unwrap_err();
        assert_eq!(err.campo, "codigo");
    }

    // Evalúa entrada inválidas verificando que fallen en el campo esperado
    let casos = [
        ("AB", "Corto", "$", 2, "codigo"),
        ("ABCDEF", "Largo", "$", 2, "codigo"),
        ("abc", "Minusculas", "$", 2, "codigo"),
        ("AB1", "Con numero", "$", 2, "codigo"),
        ("CLP", "Duplicada propia", "$", 2, "codigo"),
        ("COP", "   ", "$", 2, "nombre"),
        ("COP", "Peso colombiano", "   ", 2, "simbolo"),
        ("BTC", "Bitcoin", "₿", 9, "decimales"), 
    ];

    for (codigo, nombre, simbolo, decimales, campo_esperado) in casos {
         let err = ejecutar_crear_moneda(
            &mut repo,
            codigo,
            nombre,
            simbolo,
            decimales,
            "prueba",
            &crate::RelojSistema,
            &crate::GeneradorUuid,
        )
        .unwrap_err();
        assert_eq!(err.campo, campo_esperado);
    }
}

// Verifica que cambiar la moneda base no altere saldos ni movimientos
#[test]
fn probar_cambio_moneda_base_preserva_saldos() {
    let mut repo = RepositorioEnMemoria::default();
    let fecha = "2026-10-20";

    // Comprueba que la moneda base inicial por defecto sea USD
    assert_eq!(ejecutar_obtener_moneda_base(&repo).unwrap(), "USD");

    ejecutar_crear_moneda(
        &mut repo,
        "ABC",
        "Moneda propia",
        "A",
        3,
        "prueba",
        &crate::RelojSistema,
        &crate::GeneradorUuid,
    )
    .unwrap();

    // Crea cuentas y movimientos cuya composición no debe alterarse al cambiar base
    let cta_usd =
        ejecutar_crear_cuenta(&mut repo, "Banco USD", "banco", "USD", "200.00", "prueba").unwrap();
    let cta_abc =
        ejecutar_crear_cuenta(&mut repo, "Caja ABC", "efectivo", "ABC", "10.500", "prueba").unwrap();
    
    ejecutar_registrar_movimiento(
        &mut repo,
        RegistrarMovimiento {
            cuenta_id: cta_usd.id(),
            tipo_str: "ingreso",
            monto_str: "50.00",
            fecha_str: fecha,
            categoria_str: "Salario",
            nota_opt: None,
            dispositivo_id: "prueba",
        },
    )
    .unwrap();

    // Cambia la moneda base a VES y luego a la moneda propia
    let base_ves = ejecutar_cambiar_moneda_base(
        &mut repo,
        "VES",
        "prueba",
        &crate::RelojSistema,
        &crate::GeneradorUuid,
    )
    .unwrap();
    assert_eq!(base_ves, "VES");
    assert_eq!(ejecutar_obtener_moneda_base(&repo).unwrap(), "VES");

    let base_abc = ejecutar_cambiar_moneda_base(
        &mut repo,
        "ABC",
        "prueba",
        &crate::RelojSistema,
        &crate::GeneradorUuid,
    )
    .unwrap();
    assert_eq!(base_abc, "ABC");
    assert_eq!(ejecutar_obtener_moneda_base(&repo).unwrap(), "ABC");

    // Confirma que los saldos, movimientos y subtotales permanezcan inteacto tras el cambio de base
    let det_usd = ejecutar_obtener_detalle_cuenta(&repo, cta_usd.id()).unwrap();
    assert_eq!(det_usd.saldo_actual, "250.00");
    assert_eq!(det_usd.movimientos.len(), 1);

    let det_abc = ejecutar_obtener_detalle_cuenta(&repo, cta_abc.id()).unwrap();
    assert_eq!(det_abc.saldo_actual, "10.500");

    let resumen = ejecutar_listar_cuentas(&repo).unwrap();
    assert_eq!(resumen.cuentas.len(), 2);
}