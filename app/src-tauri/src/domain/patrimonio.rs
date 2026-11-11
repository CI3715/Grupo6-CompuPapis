use super::*;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluacionMovimiento {
    pub deja_saldo_negativo: bool,
    pub saldo_resultante: String,
}

pub fn evaluar_movimiento<R: RepositorioEventos>(
    repo: &R,
    cuenta_id: &str,
    tipo: &str,
    monto: &str,
) -> Result<EvaluacionMovimiento, ErrorDominio> {
    let eventos = repo
        .obtener_eventos()
        .map_err(|_| ErrorDominio::nuevo("general", "No se pudo leer el libro local."))?;
    let libro = proyectar_libro(&eventos)?;
    let cuenta = libro
        .cuentas
        .get(cuenta_id)
        .ok_or_else(|| ErrorDominio::nuevo("cuenta", "La cuenta seleccionada no existe."))?;

    let tipo = TipoMovimiento::desde_str(tipo)?;
    // Calcula y formatea el saldo resultante con la escala de la moneda
    let moneda = buscar_moneda_en(&cuenta.moneda, &libro.monedas)?;

    let monto = Monto::desde_str_con_moneda(monto, &moneda, "monto", false)?;
    let efecto = match tipo {
        TipoMovimiento::Ingreso => monto.cantidad(),
        TipoMovimiento::Gasto => -monto.cantidad(),
    };
    let saldo = sumar_exacto(cuenta.saldo_actual()?, efecto)?;
    validar_efecto_subtotal(&libro, &cuenta.moneda, efecto)?;
    Ok(EvaluacionMovimiento {
        deja_saldo_negativo: tipo == TipoMovimiento::Gasto && saldo < Decimal::ZERO,
        saldo_resultante: format!("{:.*}", moneda.decimales as usize, saldo),
    })
}

/// El adaptador obtiene hoy del reloj local del dispositivo; las pruebas inyectan una fecha civil.
pub fn validar_fecha_movimiento(fecha: &str, hoy: NaiveDate) -> Result<NaiveDate, ErrorDominio> {
    let fecha = NaiveDate::parse_from_str(fecha.trim(), "%Y-%m-%d").map_err(|_| {
        ErrorDominio::nuevo("fecha", "La fecha debe tener el formato válido AAAA-MM-DD.")
    })?;
    if fecha > hoy {
        return Err(ErrorDominio::nuevo(
            "fecha",
            "No se permiten movimientos con fecha futura.",
        ));
    }
    Ok(fecha)
}

// Dependencia mínima de aplicación para consultar_patrimonio. Reutiliza el libro,
// catálogo y moneda base existentes; no guarda conversiones ni saldos derivados.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilaPatrimonio {
    pub cuenta_id: String,
    pub nombre: String,
    pub moneda: String,
    pub saldo: String,
    pub tasa: Option<TasaCambio>,
    pub inversa: bool,
    pub en_base: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtotalPatrimonio {
    pub codigo: String,
    pub simbolo: String,
    pub subtotal: String,
    pub tasa: Option<TasaCambio>,
    pub inversa: bool,
    pub en_base: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ParFaltante {
    pub origen: String,
    pub destino: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Patrimonio {
    pub moneda_base: String,
    pub fecha: String,
    pub filas: Vec<FilaPatrimonio>,
    pub subtotales: Vec<SubtotalPatrimonio>,
    pub total: Option<String>,
    pub faltantes: Vec<ParFaltante>,
}

pub fn ejecutar_consultar_patrimonio<R: RepositorioEventos>(
    repo: &R,
    fecha: Option<&str>,
    hoy: NaiveDate,
) -> Result<Patrimonio, ErrorDominio> {
    let dia = match fecha {
        Some(f) => validar_fecha_movimiento(f, hoy)?,
        None => hoy,
    };
    let fecha = dia.to_string();
    let eventos = repo.obtener_eventos().map_err(|_| error_libro())?;
    let libro = proyectar_libro(&eventos)?;
    let base = buscar_moneda_en(&libro.moneda_base, &libro.monedas)?;
    let resumen = calcular_saldos_en_fecha(&libro, dia)?;
    let convertir = |cantidad: Decimal,
                     moneda: &str|
     -> Result<(Option<Decimal>, Option<TasaCambio>, bool), ErrorDominio> {
        match convertir_monto_a_base(cantidad, moneda, &base.codigo, dia, &libro.tasas) {
            Ok((valor, tasa, inversa)) => Ok((Some(valor), tasa, inversa)),
            Err(e) if e.campo == "tasa" => Ok((None, None, false)),
            Err(e) => Err(e),
        }
    };
    let mut filas = Vec::new();
    for cuenta in resumen.cuentas {
        let moneda = buscar_moneda_en(cuenta.moneda(), &libro.monedas)?;
        let (en_base, tasa, inversa) = convertir(cuenta.saldo().cantidad(), cuenta.moneda())?;
        filas.push(FilaPatrimonio {
            cuenta_id: cuenta.id().to_string(),
            nombre: cuenta.nombre().to_string(),
            moneda: cuenta.moneda().to_string(),
            saldo: format!(
                "{:.*}",
                moneda.decimales as usize,
                cuenta.saldo().cantidad()
            ),
            tasa,
            inversa,
            en_base: en_base.map(|v| v.to_string()),
        });
    }
    let mut subtotales = Vec::new();
    let mut faltantes = Vec::new();
    let mut total = Decimal::ZERO;
    for subtotal in resumen.subtotales {
        let cantidad = Decimal::from_str_exact(&subtotal.subtotal).map_err(|_| error_libro())?;
        let (en_base, tasa, inversa) = convertir(cantidad, &subtotal.codigo)?;
        if let Some(valor) = en_base {
            total = sumar_exacto(total, valor)?;
        } else {
            faltantes.push(ParFaltante {
                origen: subtotal.codigo.clone(),
                destino: base.codigo.clone(),
            });
        }
        subtotales.push(SubtotalPatrimonio {
            codigo: subtotal.codigo,
            simbolo: subtotal.simbolo,
            subtotal: subtotal.subtotal,
            tasa,
            inversa,
            en_base: en_base.map(|v| v.to_string()),
        });
    }
    Ok(Patrimonio {
        moneda_base: base.codigo,
        fecha,
        filas,
        subtotales,
        total: if faltantes.is_empty() {
            Some(format!(
                "{:.*}",
                base.decimales as usize,
                total.round_dp(base.decimales)
            ))
        } else {
            None
        },
        faltantes,
    })
}
