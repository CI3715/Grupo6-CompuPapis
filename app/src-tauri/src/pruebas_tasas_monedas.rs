use crate::domain::{
    ejecutar_crear_cuenta, ejecutar_listar_cuentas, ejecutar_listar_monedas,
    ejecutar_corregir_movimiento, ejecutar_obtener_detalle_cuenta,
    ejecutar_registrar_movimiento, evaluar_movimiento,
    RegistrarMovimiento,
    RepositorioEnMemoria,
};
use crate::tasas_monedas::ejecutar_crear_moneda;

#[test]
fn probar_cuenta_con_moneda_propia() {
    let mut repo = RepositorioEnMemoria::default();

    // Creamos una moneda nueva
    let moneda = ejecutar_crear_moneda(
        &mut repo, "ABC", "Moneda de prueba", "A", 3, "prueba", &crate::RelojSistema, &crate::GeneradorUuid,
    ).unwrap();

    assert_eq!(moneda.codigo, "ABC");

    let monedas = ejecutar_listar_monedas(&repo).unwrap();
    assert_eq!(monedas.len(), 5);

    // Vemos si podemos usarla en una cuenta
    let cuenta = ejecutar_crear_cuenta(
        &mut repo, "Cuenta de prueba", "efectivo", "ABC", "12.345", "prueba",
    ).unwrap();

    assert_eq!(cuenta.moneda(), "ABC");
    assert_eq!(cuenta.saldo().cantidad().to_string(), "12.345");

    let lista = ejecutar_listar_cuentas(&repo).unwrap();

    assert_eq!(lista.cuentas.len(), 1);
    assert_eq!(lista.cuentas[0].moneda(), "ABC");
    assert_eq!(lista.subtotales.len(), 1);
    assert_eq!(lista.subtotales[0].codigo, "ABC");
    assert_eq!(lista.subtotales[0].subtotal, "12.345");
}

#[test]
fn probar_movimientos_moneda_propia() {
    let mut repo = RepositorioEnMemoria::default();
    let fecha = "2026-10-20";

    ejecutar_crear_moneda(
        &mut repo, "ABC", "Moneda de prueba", "A", 3, "prueba", &crate::RelojSistema, &crate::GeneradorUuid,
    ).unwrap();

    let cuenta = ejecutar_crear_cuenta(
        &mut repo, "Cuenta ABC", "efectivo", "ABC", "12.345", "prueba",
    ).unwrap();

    // Probamos primero con un ingreso
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
    ).unwrap();

    assert_eq!(resp.saldo_actual, "14.450");

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
    ).unwrap();

    assert_eq!(resp.saldo_actual, "14.400");

    // Corregimos el gasto que acabamos de hacer
    let mov_id = resp.movimientos[0].id.clone();

    let resp = ejecutar_corregir_movimiento(
        &mut repo, cuenta.id(), &mov_id, "0.025", "prueba",
    ).unwrap();

    assert_eq!(resp.saldo_actual, "14.425");

    let detalle = ejecutar_obtener_detalle_cuenta(&repo, cuenta.id()).unwrap();

    assert_eq!(detalle.saldo_actual, "14.425");
    assert_eq!(detalle.composicion.ingresos, "2.105");
    assert_eq!(detalle.composicion.gastos, "0.050");
    assert_eq!(detalle.composicion.ajustes, "0.025");

    let resp = evaluar_movimiento(
        &repo, cuenta.id(), "gasto", "0.001",
    ).unwrap();

    assert_eq!(resp.saldo_resultante, "14.424");

    let lista = ejecutar_listar_cuentas(&repo).unwrap();

    assert_eq!(lista.subtotales[0].subtotal, "14.425");
}