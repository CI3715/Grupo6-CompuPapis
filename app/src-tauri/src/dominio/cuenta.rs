// Entidad Cuenta y tipos asociados del contexto Libro personal

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

use super::moneda::{CodigoMoneda, ErrorMoneda};
use super::monto::{ErrorMonto, Monto};

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

    // Rechaza nombres repetidos sin distinguir mayusculas ni espacios laterales
    #[error("El campo 'nombre' es invalido: ya existe una cuenta con el nombre '{0}'")]
    NombreDuplicado(String),

    // Rechaza tipos de cuenta que no existen en el catalogo admitido
    #[error("El campo 'tipo' es invalido: el tipo de cuenta {0} no es reconocido")]
    TipoInvalido(String),

    // Rechaza codigos de moneda que no existen en el catalogo admitido
    #[error("El campo 'moneda' es invalido: la moneda {0} no es reconocida")]
    MonedaInvalida(String),

    // Rechaza saldos iniciales menores a cero
    #[error("El campo 'saldo_inicial' es invalido: no puede ser negativo")]
    SaldoInicialNegativo,

    // Rechaza saldos iniciales con mas decimales que los de su moneda
    #[error(
        "El campo 'saldo_inicial' es invalido: la moneda '{moneda}' admite maximo {maximo} decimales, pero se recibieron {recibidos}"
    )]
    ExcesoDecimalesSaldoInicial {
        moneda: CodigoMoneda,
        maximo: u32,
        recibidos: u32,
    },
}

impl From<ErrorMoneda> for ErrorCuenta {
    // Convierte un error de moneda en error del campo 'moneda'
    fn from(error: ErrorMoneda) -> Self {
        match error {
            ErrorMoneda::MonedaDesconocida(codigo) => ErrorCuenta::MonedaInvalida(codigo),
        }
    }
}

impl From<ErrorMonto> for ErrorCuenta {
    // Convierte un error de escala en error del campo 'saldo_inicial'
    fn from(error: ErrorMonto) -> Self {
        match error {
            ErrorMonto::EscalaInvalida {
                moneda,
                maximo,
                recibidos,
            } => ErrorCuenta::ExcesoDecimalesSaldoInicial {
                moneda,
                maximo,
                recibidos,
            },
        }
    }
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
            id: id.trim().to_string(),
            nombre: nombre_limpio.to_string(),
            tipo,
            saldo_inicial,
            saldo_actual: saldo_inicial,
        })
    }

    // Verifica que el nombre no se repita ignorando mayusculas y espacios
    pub fn validar_nombre_unico(
        nombre: &str,
        cuentas_existentes: &[Self],
    ) -> Result<(), ErrorCuenta> {
        // Normaliza el nombre nuevo quitando espacios laterales y usando minusculas
        let nombre_normalizado = nombre.trim().to_lowercase();
        if nombre_normalizado.is_empty() {
            return Err(ErrorCuenta::NombreVacio);
        }

        // Compara cada cuenta existente usando la misma normalizacion
        let existe_duplicada = cuentas_existentes
            .iter()
            .any(|cuenta| cuenta.nombre().trim().to_lowercase() == nombre_normalizado);

        if existe_duplicada {
            return Err(ErrorCuenta::NombreDuplicado(nombre.trim().to_string()));
        }

        Ok(())
    }

    // Valida que el nombre de esta cuenta no exista en la lista
    pub fn validar_unicidad(&self, cuentas_existentes: &[Self]) -> Result<(), ErrorCuenta> {
        Self::validar_nombre_unico(&self.nombre, cuentas_existentes)
    }

    // Compara si la cuenta tiene el identificador que le corresponde
    pub fn mismo_id(&self, id_cuenta: &str) -> bool {
        self.id == id_cuenta.trim()
    }

    // Construye una cuenta validando todos los criterios y unicidad del nombre
    pub fn crear_validada(
        id: String,
        nombre: String,
        tipo: &str,
        moneda: &str,
        saldo_inicial: Decimal,
        cuentas_existentes: &[Self],
    ) -> Result<Self, ErrorCuenta> {
        // Valida que el nombre no este vacio ni duplicado
        Self::validar_nombre_unico(&nombre, cuentas_existentes)?;

        // Valida el tipo de cuena y el codigo de moneda
        let tipo_validado = TipoCuenta::desde_string(tipo)?;
        let codigo_moneda = CodigoMoneda::desde_string(moneda)?;

        // Valida que el saldo inicial no sea negativo antes de crear Monto
        if saldo_inicial < Decimal::ZERO {
            return Err(ErrorCuenta::SaldoInicialNegativo);
        }

        // Valida que la escala decimal respecte los decimales de la moneda
        let monto_inicial = Monto::nuevo(saldo_inicial, codigo_moneda)?;

        Self::nueva(id, nombre, tipo_validado, monto_inicial)
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
    use super::*;
    use std::str::FromStr;

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
        );

        assert_eq!(resultado, Err(ErrorCuenta::NombreVacio));
    }

    #[test]
    fn rechaza_cuenta_con_nombre_duplicado_ignorando_mayusculas_y_espacios() {
        // Verifica "cartera" y "Cartera" se detecten como duplicados
        let existente = Cuenta::crear_validada(
            String::from("cta-01"),
            String::from("Cartera"),
            "Efectivo",
            "VES",
            Decimal::ZERO,
            &[],
        )
        .unwrap();

        let resultado = Cuenta::crear_validada(
            String::from("cta-02"),
            String::from("  cArTeRa  "),
            "Banco",
            "USD",
            Decimal::ZERO,
            &[existente],
        );

        assert_eq!(
            resultado,
            Err(ErrorCuenta::NombreDuplicado(String::from("cArTeRa")))
        );
    }

    #[test]
    fn rechaza_cuenta_con_moneda_fuera_del_catalogo() {
        // Verifica rechazo de codigos desconocidos
        let resultado = Cuenta::crear_validada(
            String::from("cta-03"),
            String::from("Cuenta extranjera"),
            "Banco",
            "COP",
            Decimal::ZERO,
            &[],
        );

        assert_eq!(
            resultado,
            Err(ErrorCuenta::MonedaInvalida(String::from("COP")))
        );
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
        );

        assert_eq!(resultado, Err(ErrorCuenta::SaldoInicialNegativo));
    }

    #[test]
    fn rechaza_cuenta_con_exceso_de_decimales_en_saldo_inicial() {
        // Verifica rechazo de numeros con mas decimales de los permitidos
        let cantidad = Decimal::from_str("10.555").unwrap();
        let resultado = Cuenta::crear_validada(
            String::from("cta-05"),
            String::from("Ahorros dolares"),
            "Banco",
            "USD",
            cantidad,
            &[],
        );

        assert_eq!(
            resultado,
            Err(ErrorCuenta::ExcesoDecimalesSaldoInicial {
                moneda: CodigoMoneda::USD,
                maximo: 2,
                recibidos: 3,
            })
        );
    }

    #[test]
    fn permite_crear_cuenta_con_saldo_inicial_cero() {
        // Verifica que un saldo inicial de cero sea valido
        let saldo_cero = Monto::nuevo(Decimal::ZERO, CodigoMoneda::EUR).unwrap();
        let cuenta = Cuenta::nueva(
            String::from("cta-04"),
            String::from("Ahorros euro"),
            TipoCuenta::Banco,
            saldo_cero,
        )
        .unwrap();

        assert_eq!(cuenta.saldo_actual().cantidad(), Decimal::ZERO);
        assert_eq!(cuenta.moneda(), CodigoMoneda::EUR);
    }

    #[test]
    fn verifica_unicidad_e_identificador_de_cuenta_en_lista() {
        // Verifica comparacion de id y validacion de unicidad de instancia
        let existente = Cuenta::crear_validada(
            String::from("cta-01"),
            String::from("Cartera"),
            "Efectivo",
            "VES",
            Decimal::ZERO,
            &[],
        )
        .unwrap();

        let duplicada = Cuenta::nueva(
            String::from("cta-02"),
            String::from("cartera"),
            TipoCuenta::Banco,
            Monto::nuevo(Decimal::ZERO, CodigoMoneda::USD).unwrap(),
        )
        .unwrap();

        assert!(existente.mismo_id(" cta-01 "));
        assert_eq!(
            duplicada.validar_unicidad(&[existente]),
            Err(ErrorCuenta::NombreDuplicado(String::from("cartera")))
        );
    }
}
