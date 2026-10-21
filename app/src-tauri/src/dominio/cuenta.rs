// Entidad Cuenta y tipos asociados del contexto Libro personal

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

use super::moneda::CodigoMoneda;
use super::monto::Monto;

// Tipos de cuenta admitidos
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TipoCuenta {
    // Dinero fisico en efectivo
    Efectivo,
    // Cuenta bancaria tradicional
    Banco,
    // Billetera electronica o digital
    BilleteraDigital,
}

// Errores de validacion de dominio al construir la cuenta
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErrorCuenta {
    // Rechaza nombres vacios o compuestos unicamente por espacios
    #[error("El campo 'nombre' es invalido: el nombre no puede estar vacio")]
    NombreVacio,

    // Rechaza tipos de cuenta que no pertenecen al alcance actual
    #[error("El campo 'tipo' es invalido: el tipo de cuenta {0} no es reconocido")]
    TipoInvalido(String),

    // Rechaza saldos iniciales menores a cero
    #[error("El campo 'saldo_inicial' es invalido: no puede ser negativo")]
    SaldoInicialNegativo,
}

impl TipoCuenta {
    // Devuelve la representacion textual del tipo de cuenta
    pub fn como_string(&self) -> &'static str {
        match self {
            TipoCuenta::Efectivo => "Efectivo",
            TipoCuenta::Banco => "Banco",
            TipoCuenta::BilleteraDigital => "Billetera digital",
        }
    }

    // Convierte texto en un TipoCuenta valido sin distinguir mayusculas
    pub fn desde_string(tipo: &str) -> Result<Self, ErrorCuenta> {
        // Normaliza el texto eliminando espacios laterales y pasando a minusculas
        match tipo.trim().to_lowercase().as_str() {
            "efectivo" => Ok(TipoCuenta::Efectivo),
            "banco" => Ok(TipoCuenta::Banco),
            "billetera digital" | "billeteradigital" | "billetera_digital" => {
                Ok(TipoCuenta::BilleteraDigital)
            }
            _ => Err(ErrorCuenta::TipoInvalido(tipo.trim().to_string())),
        }
    }
}

impl fmt::Display for TipoCuenta {
    // Da formato al tipo de cuenta
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.como_string())
    }
}

// Lugar donde el usuario guarda dinero
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cuenta {
    // Identificador unico de la cuenta
    id: String,
    // Nombre asignado por el usuario a la cuenta
    nombre: String,
    // Clasificacion de la cuenta
    tipo: TipoCuenta,
    // Monto de apertura que define la moneda de la cuenta
    saldo_inicial: Monto,
    // Saldo vigente de la cuenta en su misma moneda
    saldo_actual: Monto,
}

impl Cuenta {
    // Crea una cuenta validando su nombre y que el saldo no sea negativo
    pub fn nueva(
        id: String,
        nombre: String,
        tipo: TipoCuenta,
        saldo_inicial: Monto,
    ) -> Result<Self, ErrorCuenta> {
        // Limpia espacios extremos para validar que el nombre tenga contenido
        let nombre_limpio = nombre.trim();
        if nombre_limpio.is_empty() {
            return Err(ErrorCuenta::NombreVacio);
        }

        // Verifica que el saldo inicial sea mayor o igual a cero
        if saldo_inicial.cantidad() < Decimal::ZERO {
            return Err(ErrorCuenta::SaldoInicialNegativo);
        }

        // El saldo actual inicia igual al saldo inicial
        Ok(Self {
            id,
            nombre: nombre_limpio.to_string(),
            tipo,
            saldo_inicial,
            saldo_actual: saldo_inicial,
        })
    }

    // Devuelve el id de la cuenta
    pub fn id(&self) -> &str {
        &self.id
    }

    // Devuelve el nombre validado de la cuenta
    pub fn nombre(&self) -> &str {
        &self.nombre
    }

    // Devuelve el tipo de la cuenta
    pub fn tipo(&self) -> TipoCuenta {
        self.tipo
    }

    // Devuelve la moneda de la cuenta
    pub fn moneda(&self) -> CodigoMoneda {
        self.saldo_inicial.moneda()
    }

    // Devuelve el monto del saldo inicial de la cuenta
    pub fn saldo_inicial(&self) -> Monto {
        self.saldo_inicial
    }

    // Devuelve el monto del saldo actual de la cuenta
    pub fn saldo_actual(&self) -> Monto {
        self.saldo_actual
    }
}

#[cfg(test)]
mod tests {
    // Prueba unitarias de dominio
    use std::str::FromStr;
    use supper::*;

    #[test]
    fn crea_cuenta_valida_y_conserva_moneda_inmutable() {
        // Verifica creacion exitosa y lectura de moneda
        let saldo = Monto::nuevo(Decimal::from_str("1250.50").unwrap(), CodigoMoneda::VES).unwrap();
        let cuenta = Cuenta::nueva(
            String::from("cta-01"),
            String::from("   Cartera   "),
            TipoCuenta::Efectivo,
            saldo,
        )
        .unwrap();

        assert_eq!(cuenta.id(), "cta-01");
        assert_eq!(cuenta.nombre(), "Cartera");
        assert_eq!(cuenta.tipo(), TipoCuenta::Efectivo);
        assert_eq!(cuenta.moneda(), CodigoMoneda::VES);
        assert_eq!(cuenta.saldo_inicial(), saldo);
        assert_eq!(cuenta.saldo_actual(), saldo);
    }

    #[test]
    fn rechaza_cuenta_con_nombre_vacio_o_solo_espacios() {
        // Verifica rechazo con el nombre compuesto de espacios
        let saldo = Monto::nuevo(Decimal::ZERO, CodigoMoneda::USD).unwrap();
        let resultado = Cuenta::nueva(
            String::from("cta-02"),
            String::from("   "),
            TipoCuenta::Banco,
            saldo,
        )
        .unwrap();

        assert_eq!(resultado, Err(ErrorCuenta::NombreVacio));
    }

    #[test]
    fn rechaza_cuenta_con_saldo_inicial_negativo() {
        // Verifica rechazo con el saldo inicial menor a cero
        let saldo_negativo =
            Monto::nuevo(Decimal::from_str("-10.00").unwrap(), CodigoMoneda::USDT).unwrap();
        let resultado = Cuenta::nueva(
            String::from("cta-03"),
            String::from("Billetera cripto"),
            TipoCuenta::BilleteraDigital,
            saldo_negativo,
        )
        .unwrap();

        assert_eq!(resultado, Err(ErrorCuenta::SaldoInicialNegativo));
    }

    #[test]
    fn permite_crear_cuenta_con_saldo_inicial_cero() {
        // Verifica que un saldo inicial de cero sea valido
        let saldo_cero = Monto::nuevo(Decimal::ZERO, CodigoMoneda::EUR).unwrap();
        let resultado = Cuenta::nueva(
            String::from("cta-04"),
            String::from("Ahorros euro"),
            TipoCuenta::Banco,
            saldo_cero,
        )
        .unwrap();

        assert_eq!(cuenta.saldo_actual().cantidad(), Decimal::ZERO);
        assert_eq!(cuenta.moneda(), CodigoMoneda::EUR);
    }
}
