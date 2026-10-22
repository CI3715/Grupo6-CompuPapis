//! Objeto de valor Monto para calculos monetarios

use rust_decimal::Decimal;
use thiserror::Error;

use super::moneda::{CodigoMoneda, Moneda};

// Errores de dominio al construir o validar un monto
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErrorMonto {
    // La cantidad supera los decimales permitidos para la moneda
    #[error(
        "La moneda '{moneda:?}' admite maximo {maximo} decimales, pero se recibieron {recibidos}"
    )]
    EscalaInvalida {
        moneda: CodigoMoneda,
        maximo: u32,
        recibidos: u32,
    },
}

// Par indivisiblede cantidad decimal exacta y su moneda asociada
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Monto {
    // Cantidad decimal exacta del monto
    cantidad: Decimal,
    // Codigo de moneda asociada al monto
    moneda: CodigoMoneda,
}

impl Monto {
    // Crea un nuevo monto validando que los decimales respeten su moneda
    pub fn nuevo(cantidad: Decimal, moneda: CodigoMoneda) -> Result<Self, ErrorMonto> {
        // Obtiene los decimales permitidos segun el catalogo de la moneda
        let max_decimales = u32::from(Moneda::desde_codigo(moneda).decimales());
        let decimales_recibidos = cantidad.scale();

        // Compara la escala del decimal contra el limite de la moneda
        if decimales_recibidos > max_decimales {
            return Err(ErrorMonto::EscalaInvalida {
                moneda,
                maximo: max_decimales,
                recibidos: decimales_recibidos,
            });
        }
        Ok(Self { cantidad, moneda })
    }

    // Devuelve la cantidad decimal del monto
    pub fn cantidad(&self) -> Decimal {
        self.cantidad
    }

    // Devuelve el codigo de moneda del monto
    pub fn moneda(&self) -> CodigoMoneda {
        self.moneda
    }
}

#[cfg(test)]
mod tests {
    // Pruebas unitarias de dominio para verificar los requisitos
    use super::*;
    use std::str::FromStr;

    #[test]
    fn crea_monto_valido_con_decimales_permitidos() {
        // Verifica creacion exitosa con decimales recibidos menor o igual al limite
        let cantidad = Decimal::from_str("10.55").unwrap();
        let monto = Monto::nuevo(cantidad, CodigoMoneda::USD).unwrap();

        assert_eq!(monto.cantidad(), cantidad);
        assert_eq!(monto.moneda(), CodigoMoneda::USD);
    }

    #[test]
    fn rechaza_monto_con_exceso_de_decimales() {
        // Verifica rechazo cuando la cantidad supera los decimales de USD
        let cantidad = Decimal::from_str("10.555").unwrap();
        let resultado = Monto::nuevo(cantidad, CodigoMoneda::USD);

        assert_eq!(
            resultado,
            Err(ErrorMonto::EscalaInvalida {
                moneda: CodigoMoneda::USD,
                maximo: 2,
                recibidos: 3,
            })
        );
    }

    #[test]
    fn permite_monto_cero_o_entero_sin_decimales() {
        // Verifica que una cantidad entera o cero sea valida
        let cantidad = Decimal::from_str("0").unwrap();
        let monto = Monto::nuevo(cantidad, CodigoMoneda::VES).unwrap();

        assert_eq!(monto.cantidad(), Decimal::ZERO);
        assert_eq!(monto.moneda(), CodigoMoneda::VES);
    }
}
