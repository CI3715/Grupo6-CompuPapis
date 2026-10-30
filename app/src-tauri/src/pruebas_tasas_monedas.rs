// Importa casos de uso y estructuras del dominio para pruebas en memoria
use crate::domain::{
    RegistrarMovimiento, RepositorioEnMemoria, ejecutar_corregir_movimiento, ejecutar_crear_cuenta,
    ejecutar_listar_cuentas, ejecutar_listar_monedas, ejecutar_obtener_detalle_cuenta,
    ejecutar_obtener_moneda_base, ejecutar_registrar_movimiento, evaluar_movimiento,
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
        ejecutar_crear_cuenta(&mut repo, "Caja ABC", "efectivo", "ABC", "10.500", "prueba")
            .unwrap();

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

// Reloj fijo en memoria para ejecutar pruebas deterministas sin depender del sistema
struct RelojPruebaPatrimonio;

impl crate::tasas_monedas::Reloj for RelojPruebaPatrimonio {
    // Devuelve una fecha fija en UTC para validar reglas temporales
    fn ahora(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339("2026-10-26T12:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc)
    }
}

// Generador secuencial en memoria que garantiza identificadores únicos por evento
struct IdPruebaPatrimonio {
    contador: std::cell::Cell<u32>,
}

impl IdPruebaPatrimonio {
    // Inicializa el generador de identificadores desde cero
    fn nuevo() -> Self {
        Self {
            contador: std::cell::Cell::new(0),
        }
    }
}

impl crate::tasas_monedas::GeneradorId for IdPruebaPatrimonio {
    // Incrementa el contador interno para asignar un identificador único nuevo
    fn nuevo_id(&self) -> String {
        let actual = self.contador.get() + 1;
        self.contador.set(actual);
        format!("ev-pat-{}", actual)
    }
}

// Reproduce el ejemplo de la sección 2.3 y el escenario ESC-01 en memoria
#[test]
fn test_esc_01_y_ejemplo_seccion_2_3_en_memoria() {
    use crate::domain::{
        RegistrarMovimiento, RepositorioEnMemoria, ejecutar_crear_cuenta,
        ejecutar_registrar_movimiento,
    };
    use crate::tasas_monedas::{
        ejecutar_cambiar_moneda_base, ejecutar_eliminar_tasa, ejecutar_registrar_tasa,
    };
    use chrono::NaiveDate;

    let mut repo = RepositorioEnMemoria::default();
    let reloj = RelojPruebaPatrimonio;
    let gen_id = IdPruebaPatrimonio::nuevo();
    let fecha_consulta = NaiveDate::from_ymd_opt(2026, 10, 23).unwrap();

    // Crea las tres cuentas en monedas distintas y registra movimientos iniciales
    let cta_ves = ejecutar_crear_cuenta(
        &mut repo,
        "Efectivo bolívares",
        "efectivo",
        "VES",
        "80000.00",
        "dev-local",
    )
    .unwrap();
    ejecutar_registrar_movimiento(
        &mut repo,
        RegistrarMovimiento {
            cuenta_id: cta_ves.id(),
            tipo_str: "ingreso",
            monto_str: "8700.00",
            fecha_str: "2026-10-19",
            categoria_str: "Salario",
            nota_opt: None,
            dispositivo_id: "dev-local",
        },
    )
    .unwrap();

    let cta_usd = ejecutar_crear_cuenta(
        &mut repo,
        "Banco en dólares",
        "banco",
        "USD",
        "150.00",
        "dev-local",
    )
    .unwrap();
    ejecutar_registrar_movimiento(
        &mut repo,
        RegistrarMovimiento {
            cuenta_id: cta_usd.id(),
            tipo_str: "ingreso",
            monto_str: "50.00",
            fecha_str: "2026-10-19",
            categoria_str: "Ventas",
            nota_opt: None,
            dispositivo_id: "dev-local",
        },
    )
    .unwrap();

    let cta_usdt = ejecutar_crear_cuenta(
        &mut repo,
        "Billetera USDT",
        "billetera digital",
        "USDT",
        "30.00",
        "dev-local",
    )
    .unwrap();
    ejecutar_registrar_movimiento(
        &mut repo,
        RegistrarMovimiento {
            cuenta_id: cta_usdt.id(),
            tipo_str: "ingreso",
            monto_str: "20.00",
            fecha_str: "2026-10-19",
            categoria_str: "Otros ingresos",
            nota_opt: None,
            dispositivo_id: "dev-local",
        },
    )
    .unwrap();

    // Registra las tasas de cambio hacia VES con sus fechas y fuentes
    ejecutar_registrar_tasa(
        &mut repo,
        "USD",
        "VES",
        "887.00",
        "2026-10-20",
        "BCV",
        "dev-local",
        &reloj,
        &gen_id,
    )
    .unwrap();
    ejecutar_registrar_tasa(
        &mut repo,
        "USDT",
        "VES",
        "1000.00",
        "2026-10-21",
        "casa de cambio",
        "dev-local",
        &reloj,
        &gen_id,
    )
    .unwrap();

    // Cambia la moneda base a VES para consolidar el patrimonio inicial
    ejecutar_cambiar_moneda_base(&mut repo, "VES", "dev-local", &reloj, &gen_id).unwrap();

    // Verifica saldos, equivalentes y el total conslidado de 316100.00 VES
    let pat_ves = consultar_fixture_en_fecha(&repo, fecha_consulta).unwrap();
    assert_eq!(pat_ves.moneda_base, "VES");
    assert_eq!(pat_ves.total.as_deref(), Some("316100.00"));
    assert!(pat_ves.faltantes.is_empty());
    assert_eq!(
        pat_ves
            .subtotales
            .iter()
            .filter(|s| s.tasa.is_some())
            .count(),
        2
    );
    assert_eq!(
        pat_ves.filas[0]
            .en_base
            .as_deref()
            .map(|v| v.parse::<rust_decimal::Decimal>().unwrap()),
        Some("88700.00".parse::<rust_decimal::Decimal>().unwrap())
    );
    assert_eq!(
        pat_ves.filas[1]
            .en_base
            .as_deref()
            .map(|v| v.parse::<rust_decimal::Decimal>().unwrap()),
        Some("177400.00".parse::<rust_decimal::Decimal>().unwrap())
    );
    assert_eq!(
        pat_ves.filas[2]
            .en_base
            .as_deref()
            .map(|v| v.parse::<rust_decimal::Decimal>().unwrap()),
        Some("50000.00".parse::<rust_decimal::Decimal>().unwrap())
    );

    // Cambia la moneda base a USD sin alterar saldos de las cuentas
    ejecutar_cambiar_moneda_base(&mut repo, "USD", "dev-local", &reloj, &gen_id).unwrap();
    let pat_usd_incompleto = consultar_fixture_en_fecha(&repo, fecha_consulta).unwrap();

    // Comprueba que falte la tasa USDT->USD y no muestre total consolidado
    assert!(pat_usd_incompleto.total.is_none());
    assert_eq!(
        aviso_faltante(&pat_usd_incompleto).as_deref(),
        Some("Falta una tasa de USDT a USD con fecha igual o anterior al 23/10/2026")
    );
    assert_eq!(pat_usd_incompleto.subtotales.len(), 3);

    // Verifica que los bolívares sí se convierten aplicando la tasa inversa exacta
    assert_eq!(pat_usd_incompleto.filas[0].saldo, "88700.00");
    assert_eq!(
        pat_usd_incompleto.filas[0]
            .en_base
            .as_deref()
            .map(|v| v.parse::<rust_decimal::Decimal>().unwrap()),
        Some("100.00".parse::<rust_decimal::Decimal>().unwrap())
    );
    assert!(pat_usd_incompleto.filas[0].inversa);
    assert!(pat_usd_incompleto.filas[2].en_base.is_none());

    // Registra la tasa faltante de USDT a USD y consolida 350.00 USD
    let tasa_usdt_usd = ejecutar_registrar_tasa(
        &mut repo,
        "USDT",
        "USD",
        "1.00",
        "2026-10-23",
        "casa de cambio",
        "dev-local",
        &reloj,
        &gen_id,
    )
    .unwrap();

    let pat_usd_completo = consultar_fixture_en_fecha(&repo, fecha_consulta).unwrap();
    assert_eq!(pat_usd_completo.total.as_deref(), Some("350.00"));
    assert!(pat_usd_completo.faltantes.is_empty());

    // Elimina la tasa necesaria para verificar que deje de consolidadr
    ejecutar_eliminar_tasa(&mut repo, &tasa_usdt_usd.id, "dev-local", &reloj, &gen_id).unwrap();
    let pat_tras_eliminar = consultar_fixture_en_fecha(&repo, fecha_consulta).unwrap();
    assert!(pat_tras_eliminar.total.is_none());
    assert_eq!(
        aviso_faltante(&pat_tras_eliminar).as_deref(),
        Some("Falta una tasa de USDT a USD con fecha igual o anterior al 23/10/2026")
    );
}

// Verifica que consultas en fechas pasadas usen saldos y tasas de ese día
#[test]
fn test_patrimonio_en_fechas_pasadas_y_desempate_mismo_dia() {
    use crate::domain::{
        RegistrarMovimiento, RepositorioEnMemoria, ejecutar_crear_cuenta,
        ejecutar_registrar_movimiento,
    };
    use crate::tasas_monedas::{ejecutar_cambiar_moneda_base, ejecutar_registrar_tasa};
    use chrono::NaiveDate;

    let mut repo = RepositorioEnMemoria::default();
    let reloj = RelojPruebaPatrimonio;
    let gen_id = IdPruebaPatrimonio::nuevo();

    // Registra movimientos en distintas fechas para evaluar el corte histórico
    let cta_usd = ejecutar_crear_cuenta(
        &mut repo,
        "Ahorros USD",
        "banco",
        "USD",
        "100.00",
        "dev-local",
    )
    .unwrap();
    ejecutar_registrar_movimiento(
        &mut repo,
        RegistrarMovimiento {
            cuenta_id: cta_usd.id(),
            tipo_str: "ingreso",
            monto_str: "50.00",
            fecha_str: "2026-10-21",
            categoria_str: "Salario",
            nota_opt: None,
            dispositivo_id: "dev-local",
        },
    )
    .unwrap();

    // Registra varias tasas en fechas distintas y dos en el mismo día
    ejecutar_registrar_tasa(
        &mut repo,
        "USD",
        "VES",
        "850.00",
        "2026-10-15",
        "BCV",
        "dev-local",
        &reloj,
        &gen_id,
    )
    .unwrap();
    ejecutar_registrar_tasa(
        &mut repo,
        "USD",
        "VES",
        "885.00",
        "2026-10-20",
        "BCV mañana",
        "dev-local",
        &reloj,
        &gen_id,
    )
    .unwrap();
    ejecutar_registrar_tasa(
        &mut repo,
        "USD",
        "VES",
        "887.00",
        "2026-10-20",
        "BCV tarde",
        "dev-local",
        &reloj,
        &gen_id,
    )
    .unwrap();
    ejecutar_registrar_tasa(
        &mut repo,
        "USD",
        "VES",
        "900.00",
        "2026-10-25",
        "BCV cierre",
        "dev-local",
        &reloj,
        &gen_id,
    )
    .unwrap();

    ejecutar_cambiar_moneda_base(&mut repo, "VES", "dev-local", &reloj, &gen_id).unwrap();

    // En 18/10/2026 usa el saldo inicial y la tasa del 15/10/2026
    let pat_18 =
        consultar_fixture_en_fecha(&repo, NaiveDate::from_ymd_opt(2026, 10, 18).unwrap()).unwrap();
    assert_eq!(pat_18.total.as_deref(), Some("85000.00"));
    assert_eq!(pat_18.filas[0].tasa.as_ref().unwrap().fecha, "2026-10-15");

    // En 22/10/2026 incluye el segundo movimiento y la última tasa del 10/10/2026
    let pat_22 =
        consultar_fixture_en_fecha(&repo, NaiveDate::from_ymd_opt(2026, 10, 22).unwrap()).unwrap();
    assert_eq!(pat_22.total.as_deref(), Some("133050.00"));
    assert_eq!(pat_22.filas[0].tasa.as_ref().unwrap().fuente, "BCV tarde");

    // En una fecha previa a toda tasa registrada informa el par faltante
    let pat_10 =
        consultar_fixture_en_fecha(&repo, NaiveDate::from_ymd_opt(2026, 10, 10).unwrap()).unwrap();
    assert!(pat_10.total.is_none());
    assert_eq!(
        aviso_faltante(&pat_10).as_deref(),
        Some("Falta una tasa de USD a VES con fecha igual o anterior al 10/10/2026")
    );
}

// Demuestra que el redondeo ocurre una sola vez al final
#[test]
fn test_redondeo_unico_al_final_inv_03() {
    use crate::domain::{RepositorioEnMemoria, ejecutar_crear_cuenta};
    use crate::tasas_monedas::ejecutar_registrar_tasa;
    use chrono::NaiveDate;

    let mut repo = RepositorioEnMemoria::default();
    let reloj = RelojPruebaPatrimonio;
    let gen_id = IdPruebaPatrimonio::nuevo();
    let fecha = NaiveDate::from_ymd_opt(2026, 10, 23).unwrap();

    ejecutar_crear_cuenta(&mut repo, "Cuenta EUR", "banco", "EUR", "1.00", "dev-local").unwrap();
    ejecutar_crear_cuenta(
        &mut repo,
        "Cuenta USDT",
        "billetera digital",
        "USDT",
        "1.00",
        "dev-local",
    )
    .unwrap();

    // Registra tasa con tres decimales cuyos equivalentes individuales redondearían hacia abajo
    ejecutar_registrar_tasa(
        &mut repo,
        "EUR",
        "USD",
        "1.004",
        "2026-10-20",
        "BCE",
        "dev-local",
        &reloj,
        &gen_id,
    )
    .unwrap();
    ejecutar_registrar_tasa(
        &mut repo,
        "USDT",
        "USD",
        "1.004",
        "2026-10-20",
        "mercado",
        "dev-local",
        &reloj,
        &gen_id,
    )
    .unwrap();

    let pat = consultar_fixture_en_fecha(&repo, fecha).unwrap();

    // Los equivalentes conservan 1.004 sin redondeo de presentación.
    // La suma exacta (1.004 + 1.004 = 2.008) redondea al final a 2.01
    assert_eq!(
        pat.filas[0]
            .en_base
            .as_deref()
            .map(|v| v.parse::<rust_decimal::Decimal>().unwrap()),
        Some("1.004".parse::<rust_decimal::Decimal>().unwrap())
    );
    assert_eq!(
        pat.filas[1]
            .en_base
            .as_deref()
            .map(|v| v.parse::<rust_decimal::Decimal>().unwrap()),
        Some("1.004".parse::<rust_decimal::Decimal>().unwrap())
    );
    assert_eq!(pat.total.as_deref(), Some("2.01"));
}

/// Las cuentas del fixture existían antes de las consultas de octubre. La
/// creación normal usa el reloj del sistema: copiamos SOLO estos eventos de
/// prueba con fecha explícita; no se reescribe ningún libro SQLite.
pub(crate) fn consultar_fixture_en_fecha(
    repo: &impl crate::domain::RepositorioEventos,
    fecha: chrono::NaiveDate,
) -> Result<crate::domain::Patrimonio, crate::domain::ErrorDominio> {
    use crate::domain::RepositorioEventos;
    let mut fixture = RepositorioEnMemoria::default();
    for mut evento in repo.obtener_eventos().unwrap() {
        if evento.tipo == "CuentaCreada" {
            evento.momento_registro = chrono::DateTime::parse_from_rfc3339("2026-10-01T12:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc);
        }
        fixture.guardar_evento(&evento).unwrap();
    }
    crate::domain::ejecutar_consultar_patrimonio(
        &fixture,
        Some(&fecha.to_string()),
        chrono::NaiveDate::from_ymd_opt(2026, 10, 26).unwrap(),
    )
}

pub(crate) fn aviso_faltante(p: &crate::domain::Patrimonio) -> Option<String> {
    p.faltantes.first().map(|par| {
        crate::domain::formatear_aviso_tasa_faltante(
            &par.origen,
            &par.destino,
            chrono::NaiveDate::parse_from_str(&p.fecha, "%Y-%m-%d").unwrap(),
        )
    })
}
