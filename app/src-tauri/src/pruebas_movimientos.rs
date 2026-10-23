use crate::domain::{
    ejecutar_crear_categoria,
    ejecutar_crear_cuenta,
    ejecutar_listar_categorias,
    ejecutar_listar_cuentas,
    ejecutar_registrar_movimiento,
    RepositorioEnMemoria,
    RepositorioEventos,
    TipoMovimiento,
};

use chrono::{Duration, Utc};

#[test]
fn probar_categorias() {
    let mut repo = RepositorioEnMemoria::default();
    let cats = ejecutar_listar_categorias(&repo).unwrap();

    let mut gastos = 0;
    let mut ingresos = 0;

    for c in &cats {
        if c.tipo == TipoMovimiento::Gasto {
            gastos += 1;
        } else {
            ingresos += 1;
        }
    }

    assert_eq!(cats.len(), 11);
    assert_eq!(gastos, 7);
    assert_eq!(ingresos, 4);

    let cat = ejecutar_crear_categoria(
        &mut repo, "  Viajes  ", "gasto", "pruebas"
    ).unwrap();

    assert_eq!(cat.nombre, "Viajes");

    // Probamos si nos deja crear otra igual
    let error = ejecutar_crear_categoria(
        &mut repo, "VIAJES", "gasto", "pruebas"
    ).unwrap_err();

    assert_eq!(error.campo, "categoria");

    ejecutar_crear_categoria(
        &mut repo, "Viajes", "ingreso", "pruebas"
    ).unwrap();

    let cats = ejecutar_listar_categorias(&repo).unwrap();
    assert_eq!(cats.len(), 13);

    let eventos = repo.obtener_eventos().unwrap();

    assert_eq!(eventos.len(), 2);
    assert_eq!(eventos[0].tipo, "CategoriaCreada");
    assert_eq!(eventos[1].tipo, "CategoriaCreada");
}

#[test]
fn probar_movimientos() {
    let mut repo = RepositorioEnMemoria::default();
    let hoy = Utc::now().date_naive().to_string();

    let cuenta = ejecutar_crear_cuenta(
        &mut repo, "Efectivo", "efectivo", "USD",
        "100.00", "pruebas"
    ).unwrap();

    // Usamos el mismo nombre para los 2 tipos
    ejecutar_crear_categoria(
        &mut repo, "Viajes", "gasto", "pruebas"
    ).unwrap();

    ejecutar_crear_categoria(
        &mut repo, "Viajes", "ingreso", "pruebas"
    ).unwrap();

    //Registramos los 2 movimientos y revisamos el saldo
    let ingreso = ejecutar_registrar_movimiento(
        &mut repo,
        cuenta.id(),
        "ingreso",
        "20.00",
        &hoy,
        "Viajes",
        None,
        "pruebas",
    ).unwrap();

    assert_eq!(ingreso.saldo_actual, "120.00");

    let gasto = ejecutar_registrar_movimiento(
        &mut repo,
        cuenta.id(),
        "gasto",
        "5.00",
        &hoy,
        "Viajes",
        None,
        "pruebas",
    ).unwrap();

    assert_eq!(gasto.saldo_actual, "115.00");

    let eventos = repo.obtener_eventos().unwrap();

    assert_eq!(eventos.len(), 5);
    assert_eq!(eventos[3].tipo, "MovimientoRegistrado");
    assert_eq!(eventos[4].tipo, "MovimientoRegistrado");
}

#[test]
fn probar_datos_invalidos() {
    let mut repo = RepositorioEnMemoria::default();

    let hoy = Utc::now().date_naive();
    let fecha = hoy.to_string();
    let futuro = (hoy + Duration::days(1)).to_string();

    let cuenta = ejecutar_crear_cuenta(
        &mut repo, "Banco", "banco", "USD",
        "100.00", "pruebas"
    ).unwrap();

    // Probamos varios datos que no deberían aceptarse
    let casos = [
        ("otro", "10", fecha.as_str(), "Salario", "tipo"),
        ("gasto", "0", fecha.as_str(), "Alimentación", "monto"),
        ("gasto", "-5", fecha.as_str(), "Alimentación", "monto"),
        ("gasto", "10.555", fecha.as_str(), "Alimentación", "monto"),
        ("gasto", "10", "fecha-mala", "Alimentación", "fecha"),
        ("gasto", "10", futuro.as_str(), "Alimentación", "fecha"),
        ("gasto", "10", fecha.as_str(), "", "categoria"),
        ("gasto", "10", fecha.as_str(), "No existe", "categoria"),
        ("gasto", "10", fecha.as_str(), "Salario", "categoria"),
    ];

    for (tipo, monto, fecha_mov, cat, campo) in casos {
        let error = ejecutar_registrar_movimiento(
            &mut repo,
            cuenta.id(),
            tipo,
            monto,
            fecha_mov,
            cat,
            None,
            "pruebas",
        ).unwrap_err();

        assert_eq!(error.campo, campo);
    }

    let error = ejecutar_registrar_movimiento(
        &mut repo,
        "cuenta-inexistente",
        "gasto",
        "10",
        &fecha,
        "Alimentación",
        None,
        "pruebas",
    ).unwrap_err();

    assert_eq!(error.campo, "cuenta");

    // Ninguno de esos movimientos debería guardarse
    let eventos = repo.obtener_eventos().unwrap();

    assert_eq!(eventos.len(), 1);
    assert_eq!(eventos[0].tipo, "CuentaCreada");
}

#[test]
fn probar_saldos_monedas() {
    let mut repo = RepositorioEnMemoria::default();
    let hoy = Utc::now().date_naive().to_string();

    let usd = ejecutar_crear_cuenta(
        &mut repo, "Dolares", "efectivo", "USD", "100.00", "pruebas"
    ).unwrap();

    let ves = ejecutar_crear_cuenta(
        &mut repo, "Bolivares", "banco", "VES", "200.00", "pruebas"
    ).unwrap();

    // Guardamos los datos que vamos a usar
    let datos = [
        (usd.id(), "gasto", "15.00", "Alimentación"),
        (ves.id(), "gasto", "50.00", "Transporte"),
        (usd.id(), "ingreso", "25.00", "Salario"),
    ];

    for (id, tipo, monto, cat) in datos {
        ejecutar_registrar_movimiento(
            &mut repo, id, tipo, monto, &hoy, cat, None, "pruebas"
        ).unwrap();
    }

    let lista = ejecutar_listar_cuentas(&repo).unwrap();
    assert_eq!(lista.subtotales.len(), 2);

    // Revisamos los saldos
    for c in lista.subtotales {
        if c.codigo == "USD" {
            assert_eq!(c.subtotal, "110.00");
        } else {
            assert_eq!(c.codigo, "VES");
            assert_eq!(c.subtotal, "150.00");
        }
    }
}

#[test]
fn probar_evento_repetido() {
    let mut repo = RepositorioEnMemoria::default();

    ejecutar_crear_cuenta(
        &mut repo, "Efectivo", "efectivo", "USD", "100.00", "pruebas"
    ).unwrap();

    let eventos = repo.obtener_eventos().unwrap();

    // Guardamos de nuevo el mismo evento
    repo.guardar_evento(&eventos[0]).unwrap();

    assert_eq!(repo.obtener_eventos().unwrap().len(), 1);
}