//! Tipos y catalogo por defecto de monedas soportadas por la aplicacion

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

// Codigos de monedas soportadas por la aplicacion
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CodigoMoneda {
    // Bolivar venezolano (Bs.)
    VES,
    // Dolar estadounidense ($)
    USD,
    // Euro (€)
    EUR,
    // Tether o dolar digital (USDT)
    USDT,
}

// Errores de validacion a la hora de consultar o construir una moneda
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErrorMoneda {
    // Rechaza los codigos de monedas fuera del catalogo
    #[error("El campo 'moneda' es invalido : la moneda '{0}' no esta en el catalogo")]
    MonedaDesconocida(String),
}

impl CodigoMoneda {
    // Devuelve el codigo textual de la moneda
    pub fn como_string(&self) -> &'static str {
        match self {
            CodigoMoneda::VES => "VES",
            CodigoMoneda::USD => "USD",
            CodigoMoneda::EUR => "EUR",
            CodigoMoneda::USDT => "USDT",
        }
    }

    // Convierte una cadena en un codigo de moneda, si es valido
    pub fn desde_string(codigo: &str) -> Result<Self, ErrorMoneda> {
        match codigo.trim().to_uppercase().as_str() {
            "VES" => Ok(CodigoMoneda::VES),
            "USD" => Ok(CodigoMoneda::USD),
            "EUR" => Ok(CodigoMoneda::EUR),
            "USDT" => Ok(CodigoMoneda::USDT),
            _ => Err(ErrorMoneda::MonedaDesconocida(codigo.trim().to_string())),
        }
    }
}

impl fmt::Display for CodigoMoneda {
    // Formatea el codigo de moneda para mostrarlo como texto
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.como_string())
    }
}

// Representa una moneda con su codigo, nombre, simbolo y decimales
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Moneda {
    pub codigo: CodigoMoneda,
    pub nombre: String,
    pub simbolo: String,
    pub decimales: u8,
}

impl Moneda {
    // Construye los datos de fabrica para un codigo de moneda
    pub fn desde_codigo(codigo: CodigoMoneda) -> Self {
        match codigo {
            CodigoMoneda::VES => Moneda {
                codigo: CodigoMoneda::VES,
                nombre: String::from("Bolívar venezolano"),
                simbolo: String::from("Bs."),
                decimales: 2,
            },
            CodigoMoneda::USD => Moneda {
                codigo: CodigoMoneda::USD,
                nombre: String::from("Dólar estadounidense"),
                simbolo: String::from("$"),
                decimales: 2,
            },
            CodigoMoneda::EUR => Moneda {
                codigo: CodigoMoneda::EUR,
                nombre: String::from("Euro"),
                simbolo: String::from("€"),
                decimales: 2,
            },
            CodigoMoneda::USDT => Moneda {
                codigo: CodigoMoneda::USDT,
                nombre: String::from("Tether (dólar digital)"),
                simbolo: String::from("USDT"),
                decimales: 2,
            },
        }
    }

    // Devuelve el catalogo completo de monedas soportadas por la aplicacion
    pub fn catalogo_base() -> Vec<Self> {
        vec![
            Self::desde_codigo(CodigoMoneda::VES),
            Self::desde_codigo(CodigoMoneda::USD),
            Self::desde_codigo(CodigoMoneda::EUR),
            Self::desde_codigo(CodigoMoneda::USDT),
        ]
    }

    // Busca una moneda en el catalogo por su codigo, si existe
    pub fn buscar_por_codigo(codigo: &str) -> Result<Self, ErrorMoneda> {
        let codigo_validado = CodigoMoneda::desde_string(codigo)?;
        Ok(Self::desde_codigo(codigo_validado))
    }

    // Obtiene el codigo indentificador de la moneda
    pub fn codigo(&self) -> CodigoMoneda {
        self.codigo
    }

    // Obtiene el nombre completo de la moneda
    pub fn nombre(&self) -> &str {
        &self.nombre
    }

    // Obtiene el simbolo de la moneda
    pub fn simbolo(&self) -> &str {
        &self.simbolo
    }

    // Obtiene la cantidad de decimales soportados por la moneda
    pub fn decimales(&self) -> u8 {
        self.decimales
    }
}
