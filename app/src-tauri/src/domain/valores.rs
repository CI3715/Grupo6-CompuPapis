use super::*;

// ==========================================
// ERROR DE DOMINIO (RNF-18: Indica campo exacto)
// ==========================================
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorDominio {
    pub campo: String,
    pub mensaje: String,
}

impl ErrorDominio {
    pub fn nuevo(campo: &str, mensaje: &str) -> Self {
        Self {
            campo: campo.to_string(),
            mensaje: mensaje.to_string(),
        }
    }
}

// ==========================================
// CATÁLOGO DE MONEDAS (RF-CAM-01)
// ==========================================

// Representa moneda registrada con su escala decimal
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Moneda {
    pub codigo: String,
    pub nombre: String,
    pub simbolo: String,
    pub decimales: u32,
}

// Valida formato, unicidad y límite de decimales de una moneda nueva
pub fn validar_moneda(
    codigo: &str,
    nombre: &str,
    simbolo: &str,
    decimales: u32,
    monedas: &[Moneda],
) -> Result<Moneda, ErrorDominio> {
    // Revisa que el código tenga entre 3 y 5 caracteres
    let n = codigo.len();

    if !(3..=5).contains(&n) {
        return Err(ErrorDominio::nuevo(
            "codigo",
            "El código tiene que tener de 3 a 5 letras.",
        ));
    }

    // Comprueba que el código solo contenga letras mayúsculas
    for c in codigo.chars() {
        if !c.is_ascii_uppercase() {
            return Err(ErrorDominio::nuevo("codigo", "Usa solo letras mayúsculas."));
        }
    }

    // Rechaza códigos duplicados y protege las monedas del catálogo actual
    for m in monedas {
        if m.codigo == codigo {
            return Err(ErrorDominio::nuevo(
                "codigo",
                "Ya hay una moneda con ese codigo.",
            ));
        }
    }

    // Valida que el nombre y el símbolo no estén vacíos
    if nombre.trim().is_empty() {
        return Err(ErrorDominio::nuevo("nombre", "Debes poner un nombre."));
    }

    if simbolo.trim().is_empty() {
        return Err(ErrorDominio::nuevo("simbolo", "Debes poner un símbolo."));
    }

    // Limita la escala máxima permitida a 8 decimales
    if decimales > 8 {
        return Err(ErrorDominio::nuevo(
            "decimales",
            "Solo puede tener hasta 8 decimales.",
        ));
    }

    let nueva = Moneda {
        codigo: codigo.to_string(),
        nombre: nombre.trim().to_string(),
        simbolo: simbolo.trim().to_string(),
        decimales,
    };

    Ok(nueva)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TasaCambio {
    pub id: String,
    pub origen: String,
    pub destino: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub valor: Decimal,
    pub fecha: String,
    pub fuente: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatosTasaEliminada {
    pub tasa_id: String,
}

/// Datos del evento que registra el cambio de moneda base
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatosMonedaBaseCambiada {
    pub codigo: String,
}

#[allow(clippy::too_many_arguments)] // Contrato existente, reloj y catálogo explícitos.
pub fn validar_tasa(
    id: &str,
    origen: &str,
    destino: &str,
    valor: &str,
    fecha: &str,
    fuente: &str,
    monedas: &[Moneda],
    hoy: NaiveDate,
) -> Result<TasaCambio, ErrorDominio> {
    if origen.trim().is_empty() {
        return Err(ErrorDominio::nuevo(
            "origen",
            "Falta elegir la moneda de origen.",
        ));
    }

    if destino.trim().is_empty() {
        return Err(ErrorDominio::nuevo(
            "destino",
            "Falta elegir la moneda de destino.",
        ));
    }

    // Buscamos las 2 monedas
    let m1 = match buscar_moneda_en(origen, monedas) {
        Ok(m) => m,
        Err(_) => return Err(ErrorDominio::nuevo("origen", "Esa moneda no existe.")),
    };

    let m2 = match buscar_moneda_en(destino, monedas) {
        Ok(m) => m,
        Err(_) => return Err(ErrorDominio::nuevo("destino", "Esa moneda no existe.")),
    };

    if m1.codigo == m2.codigo {
        return Err(ErrorDominio::nuevo(
            "destino",
            "No se puede usar la misma moneda.",
        ));
    }

    let texto = valor.trim().replace(',', ".");

    if texto.is_empty() {
        return Err(ErrorDominio::nuevo("valor", "Falta poner el valor."));
    }

    if texto
        .split_once('.')
        .is_some_and(|(_, fraccion)| fraccion.len() > 8)
    {
        return Err(ErrorDominio::nuevo("valor", "Máximo 8 decimales."));
    }

    let num = match Decimal::from_str_exact(&texto) {
        Ok(n) => n,
        Err(_) => return Err(ErrorDominio::nuevo("valor", "El valor esta mal.")),
    };

    if num <= Decimal::ZERO {
        return Err(ErrorDominio::nuevo("valor", "Tiene que ser mayor que 0."));
    }

    if num.scale() > 8 {
        return Err(ErrorDominio::nuevo("valor", "Maximo 8 decimales."));
    }

    if fecha.trim().is_empty() {
        return Err(ErrorDominio::nuevo("fecha", "Falta poner la fecha."));
    }

    let dia = match NaiveDate::parse_from_str(fecha.trim(), "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => return Err(ErrorDominio::nuevo("fecha", "Revisa la fecha.")),
    };

    if dia > hoy {
        return Err(ErrorDominio::nuevo(
            "fecha",
            "La fecha no puede ser futura.",
        ));
    }

    if fuente.trim().is_empty() {
        return Err(ErrorDominio::nuevo("fuente", "Falta poner la fuente."));
    }

    Ok(TasaCambio {
        id: id.to_string(),
        origen: m1.codigo,
        destino: m2.codigo,
        valor: num,
        fecha: dia.to_string(),
        fuente: fuente.trim().to_string(),
    })
}

pub fn catalogo_monedas() -> Vec<Moneda> {
    vec![
        Moneda {
            codigo: "VES".into(),
            nombre: "Bolívar".into(),
            simbolo: "Bs.".into(),
            decimales: 2,
        },
        Moneda {
            codigo: "USD".into(),
            nombre: "Dólar estadounidense".into(),
            simbolo: "$".into(),
            decimales: 2,
        },
        Moneda {
            codigo: "EUR".into(),
            nombre: "Euro".into(),
            simbolo: "€".into(),
            decimales: 2,
        },
        Moneda {
            codigo: "USDT".into(),
            nombre: "Tether (dólar digital)".into(),
            simbolo: "USDT".into(),
            decimales: 2,
        },
    ]
}

pub fn buscar_moneda(codigo: &str) -> Result<Moneda, ErrorDominio> {
    catalogo_monedas()
        .into_iter()
        .find(|m| m.codigo == codigo.trim().to_uppercase())
        .ok_or_else(|| ErrorDominio::nuevo("moneda", "Esa moneda no está en la lista."))
}

// Busca una moneda por código dentro de una lista dinámica de monedas
pub fn buscar_moneda_en(codigo: &str, monedas: &[Moneda]) -> Result<Moneda, ErrorDominio> {
    // Compara el código normalizado en mayúsculas y el catálogo recibido
    for m in monedas {
        if m.codigo == codigo.trim().to_uppercase() {
            return Ok(m.clone());
        }
    }

    // Retorna error en el campo moneda si no existe en el catálogo
    Err(ErrorDominio::nuevo(
        "moneda",
        "Esa moneda no está en la lista.",
    ))
}

// ==========================================
// MONTO (INV-01, INV-02, INV-03)
// ==========================================
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Monto {
    pub(super) cantidad: Decimal,
    pub(super) moneda: String,
}

impl Monto {
    pub fn nuevo(cantidad: Decimal, moneda_codigo: &str) -> Result<Self, ErrorDominio> {
        let moneda = buscar_moneda(moneda_codigo)?;
        if cantidad.scale() > moneda.decimales {
            return Err(ErrorDominio::nuevo(
                "monto",
                &format!(
                    "El monto no puede tener más de {} decimales para {}.",
                    moneda.decimales, moneda.codigo
                ),
            ));
        }
        Ok(Self {
            cantidad: cantidad.round_dp(moneda.decimales),
            moneda: moneda.codigo,
        })
    }

    pub fn desde_str_con_campo(
        valor_str: &str,
        moneda_codigo: &str,
        campo_err: &str,
        permitir_cero: bool,
    ) -> Result<Self, ErrorDominio> {
        let limpio = valor_str.trim().replace(',', ".");
        if limpio.is_empty() {
            return Err(ErrorDominio::nuevo(campo_err, "El monto es obligatorio."));
        }
        let dec = Decimal::from_str(&limpio).map_err(|_| {
            ErrorDominio::nuevo(campo_err, "El monto debe ser un número decimal válido.")
        })?;

        if permitir_cero {
            if dec < Decimal::ZERO {
                return Err(ErrorDominio::nuevo(
                    campo_err,
                    "El monto no puede ser negativo.",
                ));
            }
        } else if dec <= Decimal::ZERO {
            return Err(ErrorDominio::nuevo(
                campo_err,
                "El monto debe ser mayor que cero.",
            ));
        }

        let moneda = buscar_moneda(moneda_codigo)?;
        if dec.scale() > moneda.decimales {
            return Err(ErrorDominio::nuevo(
                campo_err,
                &format!(
                    "El monto no puede tener más de {} decimales para {}.",
                    moneda.decimales, moneda.codigo
                ),
            ));
        }

        Ok(Self {
            cantidad: dec.round_dp(moneda.decimales),
            moneda: moneda.codigo,
        })
    }

    pub fn desde_str(valor_str: &str, moneda_codigo: &str) -> Result<Self, ErrorDominio> {
        Self::desde_str_con_campo(valor_str, moneda_codigo, "saldo_inicial", true)
    }

    pub fn desde_str_con_moneda(
        valor_str: &str,
        moneda: &Moneda,
        campo_err: &str,
        permitir_cero: bool,
    ) -> Result<Self, ErrorDominio> {
        // Normaliza el separador decimal reemplazando comas por puntos
        let texto = valor_str.trim().replace(',', ".");

        if texto.is_empty() {
            return Err(ErrorDominio::nuevo(campo_err, "Falta poner el monto."));
        }

        // Parsea el texto a Decimal exacto evitando usar como flotante
        let monto = match Decimal::from_str(&texto) {
            Ok(n) => n,
            Err(_) => {
                return Err(ErrorDominio::nuevo(
                    campo_err,
                    "El monto tiene que ser un número.",
                ));
            }
        };

        // Rechaza valores negativos o iguales a cero cuando no están permitidos
        if monto < Decimal::ZERO {
            return Err(ErrorDominio::nuevo(
                campo_err,
                "El monto no puede ser negativo.",
            ));
        }

        if monto == Decimal::ZERO && !permitir_cero {
            return Err(ErrorDominio::nuevo(
                campo_err,
                "El monto debe ser mayor que cero.",
            ));
        }

        // Verifica que el monto no supere los decimales de su moneda
        if monto.scale() > moneda.decimales {
            return Err(ErrorDominio::nuevo(
                campo_err,
                &format!(
                    "Para {} puedes usar hasta {} decimales.",
                    moneda.codigo, moneda.decimales
                ),
            ));
        }

        // Asocia cantidad redondeada y código de moneda como par indivisible
        Ok(Self {
            cantidad: monto.round_dp(moneda.decimales),
            moneda: moneda.codigo.clone(),
        })
    }
    pub fn cantidad(&self) -> Decimal {
        self.cantidad
    }

    pub fn moneda(&self) -> &str {
        &self.moneda
    }

    pub fn sumar(&self, otro: &Monto) -> Result<Monto, ErrorDominio> {
        if self.moneda != otro.moneda {
            return Err(ErrorDominio::nuevo(
                "moneda",
                "INV-02: No se pueden sumar montos de monedas distintas.",
            ));
        }
        Self::nuevo(sumar_exacto(self.cantidad, otro.cantidad)?, &self.moneda)
    }
}

// ==========================================
// CUENTA Y CATEGORÍA (RF-LIB-01, RF-LIB-07)
// ==========================================
