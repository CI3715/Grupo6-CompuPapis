//! Libro personal para el contexto de cuentas

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::cuenta::{Cuenta, ErrorCuenta, TipoCuenta};
use super::moneda::CodigoMoneda;
use super::monto::Monto;

// Nombre constante que identifica al evento de creacion de cuenta
pub const TIPO_EVENTO_CUENTA_CREADA: &str = "CuentaCreada";

// Errores de validacion que identifica al evento de creacion de cuenta
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErrorEvento {
    // Rechaza eventos cuyo identificador unico este vacio
    #[error("El campo 'id_evento' es invalido: no puede estar vacio")]
    IdEventoVacio,

    // Rechaza eventos sin fecha o momento de registro
    #[error("El campo 'momento_registro' es invalido: no puede estar vacio")]
    MomentoRegistroVacio,

    // Rechaza eventos que no indiquen el dispositivo de origen
    #[error("El campo 'dispositivo_origen' es invalido: no puede estar vacio")]
    DispositivoOrigenVacio,

    // Rechaza eventos cuyo identificador de cuenta este vacio
    #[error("El campo 'id_cuenta' es invalido: no puede estar vacio")]
    IdCuentaVacio,

    // Propaga cualquier error de validacion de los datos de la cuenta
    #[error(transparent)]
    CuentaInvalida(#[from] ErrorCuenta),
}

// Evento registrado cuando el usuario local crea una cuenta
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CuentaCreada {
    // Identificador unico e inmutable del evento
    id_evento: String,
    // Descripcion del evento registrado en el libro
    tipo_evento: String,
    // Momento en que se registro el evento
    momento_registro: String,
    // Dispositivo local donde se origino el evento
    dispositivo_origen: String,
    // Identificador de la cuenta creada
    id_cuenta: String,
    // Nombre validado de la cuenta creada
    nombre: String,
    // Clasificacion de la cuenta
    tipo: TipoCuenta,
    // Moneda asociada a la cuenta
    moneda: CodigoMoneda,
    // Cantidad decimal de apertura de cuenta
    saldo_inicial: Decimal,
}

impl CuentaCreada {
    // Evento inmutable validando los metadatos y datos de cuenta
    #[allow(clippy::too_many_arguments)]
    pub fn nuevo(
        id_evento: String,
        momento_registro: String,
        dispositivo_origen: String,
        id_cuenta: String,
        nombre: String,
        tipo: TipoCuenta,
        moneda: CodigoMoneda,
        saldo_inicial: Decimal,
    ) -> Result<Self, ErrorEvento> {
        // Verifica que los metadatos obligatorios del evento no esten vacios
        let id_evento_limpio = id_evento.trim();
        if id_evento_limpio.is_empty() {
            return Err(ErrorEvento::IdEventoVacio);
        }

        let momento_limpio = momento_registro.trim();
        if momento_limpio.is_empty() {
            return Err(ErrorEvento::MomentoRegistroVacio);
        }

        let dispositivo_limpio = dispositivo_origen.trim();
        if dispositivo_limpio.is_empty() {
            return Err(ErrorEvento::DispositivoOrigenVacio);
        }

        let id_cuenta_limpio = id_cuenta.trim();
        if id_cuenta_limpio.is_empty() {
            return Err(ErrorEvento::IdCuentaVacio);
        }

        // Valida los datos de la cuenta reusando las reglas del dominio
        let monto_inicial = Monto::nuevo(saldo_inicial, moneda).map_err(ErrorCuenta::from)?;
        let cuenta_validada =
            Cuenta::nueva(id_cuenta_limpio.to_string(), nombre, tipo, monto_inicial)?;

        Ok(Self {
            id_evento: id_evento_limpio.to_string(),
            tipo_evento: String::from(TIPO_EVENTO_CUENTA_CREADA),
            momento_registro: momento_limpio.to_string(),
            dispositivo_origen: dispositivo_limpio.to_string(),
            id_cuenta: cuenta_validada.id().to_string(),
            nombre: cuenta_validada.nombre().to_string(),
            tipo: cuenta_validada.tipo(),
            moneda: cuenta_validada.moneda(),
            saldo_inicial: cuenta_validada.saldo_inicial().cantidad(),
        })
    }

    // Crea el evento inmutable a partir de una entidad Cuenta valida
    pub fn desde_cuenta(
        id_evento: String,
        momento_registro: String,
        dispositivo_origen: String,
        cuenta: &Cuenta,
    ) -> Result<Self, ErrorEvento> {
        Self::nuevo(
            id_evento,
            momento_registro,
            dispositivo_origen,
            cuenta.id().to_string(),
            cuenta.nombre().to_string(),
            cuenta.tipo(),
            cuenta.moneda(),
            cuenta.saldo_inicial().cantidad(),
        )
    }

    // Construye la entidad Cuenta a partir de los datos del evento
    pub fn a_cuenta(&self) -> Result<Cuenta, ErrorCuenta> {
        // Construye el par Monto antes de recrear la cuenta
        let monto_inicial = Monto::nuevo(self.saldo_inicial, self.moneda)?;
        Cuenta::nueva(
            self.id_cuenta.clone(),
            self.nombre.clone(),
            self.tipo,
            monto_inicial,
        )
    }

    // Devuelve el identificador del evento
    pub fn id_evento(&self) -> &str {
        &self.id_evento
    }

    // Devuelve el tipo del evento
    pub fn tipo_evento(&self) -> &str {
        &self.tipo_evento
    }

    // Devuelve el momento de registro del evento
    pub fn momento_registro(&self) -> &str {
        &self.momento_registro
    }

    // Devuelve el identificador del dispositivo de origen
    pub fn dispositivo_origen(&self) -> &str {
        &self.dispositivo_origen
    }

    // Devuelve el identificador de la cuenta creada
    pub fn id_cuenta(&self) -> &str {
        &self.id_cuenta
    }

    // Devuelve el nombre de la cuenta creada
    pub fn nombre(&self) -> &str {
        &self.nombre
    }

    // Devuelve el tipo de la cuenta creada
    pub fn tipo(&self) -> TipoCuenta {
        self.tipo
    }

    // Devuelve la moneda de la cuenta creada
    pub fn moneda(&self) -> CodigoMoneda {
        self.moneda
    }

    // Devuelve el saldo inicial
    pub fn saldo_inicial(&self) -> Decimal {
        self.saldo_inicial
    }
}

#[cfg(test)]
mod tests {
    // Pruebas unitarias para verificar la creacion e inmutabilidad de CuentaCreada
    use super::*;
    use std::str::FromStr;

    #[test]
    fn crea_evento_cuenta_creada_valido_y_permite_lectura() {
        // Verifica que el evento conserve todos sus atributos inmutables
        let saldo = Decimal::from_str("300.00").unwrap();
        let evento = CuentaCreada::nuevo(
            String::from("evento-01"),
            String::from("2026-10-21"),
            String::from("dispositivo-01"),
            String::from("cta-01"),
            String::from("Cuenta nomina"),
            TipoCuenta::Banco,
            CodigoMoneda::USD,
            saldo,
        )
        .unwrap();

        assert_eq!(evento.id_evento(), "evento-01");
        assert_eq!(evento.tipo_evento(), TIPO_EVENTO_CUENTA_CREADA);
        assert_eq!(evento.momento_registro(), "2026-10-21");
        assert_eq!(evento.dispositivo_origen(), "dispositivo-01");
        assert_eq!(evento.id_cuenta(), "cta-01");
        assert_eq!(evento.nombre(), "Cuenta nomina");
        assert_eq!(evento.tipo(), TipoCuenta::Banco);
        assert_eq!(evento.moneda(), CodigoMoneda::USD);
        assert_eq!(evento.saldo_inicial(), saldo);
    }

    #[test]
    fn construye_evento_desde_cuenta_y_reconstruye_entidad() {
        // Verifica conversion entre la entidad Cuenta y CuentaCreada
        let monto = Monto::nuevo(Decimal::from_str("1250.50").unwrap(), CodigoMoneda::VES).unwrap();
        let cuenta = Cuenta::nueva(
            String::from("cta-02"),
            String::from("Cartera"),
            TipoCuenta::Efectivo,
            monto,
        )
        .unwrap();

        let evento = CuentaCreada::desde_cuenta(
            String::from("evento-02"),
            String::from("2026-10-21"),
            String::from("dispositivo-01"),
            &cuenta,
        )
        .unwrap();

        let cuenta_reconstruida = evento.a_cuenta().unwrap();
        assert_eq!(cuenta_reconstruida, cuenta);
    }

    #[test]
    fn rechaza_evento_con_identificador_o_metadatos_vacios() {
        // Verifica que no se creen eventos sin identificador unico
        let resultado = CuentaCreada::nuevo(
            String::from("  "),
            String::from("2026-10-21"),
            String::from("pc-ubuntu"),
            String::from("cta-03"),
            String::from("Billetera USDT"),
            TipoCuenta::BilleteraDigital,
            CodigoMoneda::USDT,
            Decimal::ZERO,
        );

        assert_eq!(resultado, Err(ErrorEvento::IdEventoVacio));
    }

    #[test]
    fn verifica_que_cuenta_creada_implemente_serialize_y_deserialize() {
        // Comprueba en compilacion que el evento soporta serializacion con serde
        fn exigir_serde<T: Serialize + for<'de> Deserialize<'de>>() {}
        exigir_serde::<CuentaCreada>();
    }
}
