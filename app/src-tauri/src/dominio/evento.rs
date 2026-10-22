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

// Contenedor del libro personal que reconstruye cuentas desde eventos
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LibroPersonal {
    // Historial de eventos aplicados en orden de llegada
    eventos: Vec<CuentaCreada>,
    // Lista de cuentas vigentes proyectada desde los eventos
    cuentas: Vec<Cuenta>,
}

impl LibroPersonal {
    // Crea un libro personal vacio sin eventos ni cuentas
    pub fn nuevo() -> Self {
        Self::default()
    }

    // Construye el libro personal aplicando una secuencia de eventos
    pub fn desde_eventos(eventos: &[CuentaCreada]) -> Result<Self, ErrorEvento> {
        let mut libro = Self::nuevo();
        libro.aplicar_eventos(eventos)?;
        Ok(libro)
    }

    // Verifica si un evento ya fue registrado usando su identificador
    pub fn contiene_evento(&self, id_evento: &str) -> bool {
        let id_limpio = id_evento.trim();
        self.eventos
            .iter()
            .any(|registrado| registrado.id_evento() == id_limpio)
    }

    // Aplica un evento CuentaCreada asegurando idempotencia
    pub fn aplicar_cuenta_creada(&mut self, evento: CuentaCreada) -> Result<(), ErrorEvento> {
        // Ignora el evento si su identificador ya fue aplicado antes
        if self.contiene_evento(evento.id_evento()) {
            return Ok(());
        }

        // Ignora creacion si el identificador de cuenta ya existe
        if self
            .cuentas
            .iter()
            .any(|cuenta| cuenta.mismo_id(evento.id_cuenta()))
        {
            return Ok(());
        }

        // Construye y valida la cuenta antes de modificar el estado
        let nueva_cuenta = evento.a_cuenta()?;
        nueva_cuenta.validar_unicidad(&self.cuentas)?;

        self.cuentas.push(nueva_cuenta);
        self.eventos.push(evento);
        Ok(())
    }

    // Aplica en orden una lista de eventos CuentaCreada
    pub fn aplicar_eventos(&mut self, eventos: &[CuentaCreada]) -> Result<(), ErrorEvento> {
        for evento in eventos {
            self.aplicar_cuenta_creada(evento.clone())?;
        }
        Ok(())
    }

    // Genera la lista de cuentas vigentes desde una secuencia de eventos
    pub fn proyectar_cuentas(eventos: &[CuentaCreada]) -> Result<Vec<Cuenta>, ErrorEvento> {
        let libro = Self::desde_eventos(eventos)?;
        Ok(libro.cuentas)
    }

    // Devuelve una vista de solo lectura de los eventos registrados
    pub fn eventos(&self) -> &[CuentaCreada] {
        &self.eventos
    }

    // Devuelve una vista de solo lectura de las cuentas vigentes
    pub fn cuentas(&self) -> &[Cuenta] {
        &self.cuentas
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
        let err_id = CuentaCreada::nuevo(
            String::from("  "),
            String::from("2026-10-21"),
            String::from("pc-ubuntu"),
            String::from("cta-03"),
            String::from("Billetera USDT"),
            TipoCuenta::BilleteraDigital,
            CodigoMoneda::USDT,
            Decimal::ZERO,
        )
        .unwrap_err();
        assert_eq!(err_id, ErrorEvento::IdEventoVacio);
        assert!(err_id.to_string().contains("id_evento"));

        // Verifica que no se creen eventos sin momento registrado
        let err_momento = CuentaCreada::nuevo(
            String::from("evento-03"),
            String::from(" "),
            String::from("pc-ubuntu"),
            String::from("cta-03"),
            String::from("Billetera USDT"),
            TipoCuenta::BilleteraDigital,
            CodigoMoneda::USDT,
            Decimal::ZERO,
        )
        .unwrap_err();
        assert_eq!(err_momento, ErrorEvento::MomentoRegistroVacio);
        assert!(err_momento.to_string().contains("momento_registro"));

        // Verifica que no se creen eventos sin dispositivo
        let err_dispositivo = CuentaCreada::nuevo(
            String::from("evento-03"),
            String::from("2026-10-21"),
            String::from(""),
            String::from("cta-03"),
            String::from("Billetera USDT"),
            TipoCuenta::BilleteraDigital,
            CodigoMoneda::USDT,
            Decimal::ZERO,
        )
        .unwrap_err();
        assert_eq!(err_dispositivo, ErrorEvento::DispositivoOrigenVacio);
        assert!(err_dispositivo.to_string().contains("dispositivo_origen"));

        // Verifica que no se creen eventos sin identificador de cuenta
        let err_cuenta = CuentaCreada::nuevo(
            String::from("evento-03"),
            String::from("2026-10-21"),
            String::from("pc-ubuntu"),
            String::from(" "),
            String::from("Billetera USDT"),
            TipoCuenta::BilleteraDigital,
            CodigoMoneda::USDT,
            Decimal::ZERO,
        )
        .unwrap_err();
        assert_eq!(err_cuenta, ErrorEvento::IdCuentaVacio);
        assert!(err_cuenta.to_string().contains("id_cuenta"));
    }

    #[test]
    fn propaga_errores_de_validacion_de_cuenta_en_evento() {
        // Verifica que errores de cuenta en el evento identifiquen el campo
        let err_decimales = CuentaCreada::nuevo(
            String::from("evento-04"),
            String::from("2026-10-21"),
            String::from("pc-ubuntu"),
            String::from("cta-04"),
            String::from("Ahorros"),
            TipoCuenta::Banco,
            CodigoMoneda::USD,
            Decimal::from_str("10.555").unwrap(),
        )
        .unwrap_err();
        assert!(err_decimales.to_string().contains("saldo_inicial"));

        let err_nombre = CuentaCreada::nuevo(
            String::from("evento-05"),
            String::from("2026-10-21"),
            String::from("pc-ubuntu"),
            String::from("cta-05"),
            String::from(" "),
            TipoCuenta::Efectivo,
            CodigoMoneda::VES,
            Decimal::ZERO,
        )
        .unwrap_err();
        assert!(err_nombre.to_string().contains("nombre"));
    }

    #[test]
    fn verifica_que_cuenta_creada_implemente_serialize_y_deserialize() {
        // Comprueba en compilacion que el evento soporta serializacion con serde
        fn exigir_serde<T: Serialize + for<'de> Deserialize<'de>>() {}
        exigir_serde::<CuentaCreada>();
    }

    #[test]
    fn proyecta_lista_de_cuentas_vigentes_desde_secuencia_de_eventos() {
        // Verifica que proyectar varios eventos reconstruya las cuentas vigentes
        let ev1 = CuentaCreada::nuevo(
            String::from("evento-01"),
            String::from("2026-10-21"),
            String::from("pc-local"),
            String::from("cta-01"),
            String::from("Cartera"),
            TipoCuenta::Efectivo,
            CodigoMoneda::VES,
            Decimal::from_str("1250.50").unwrap(),
        )
        .unwrap();

        let ev2 = CuentaCreada::nuevo(
            String::from("evento-02"),
            String::from("2026-10-21"),
            String::from("pc-local"),
            String::from("cta-02"),
            String::from("Cuenta nomina"),
            TipoCuenta::Banco,
            CodigoMoneda::USD,
            Decimal::from_str("300.00").unwrap(),
        )
        .unwrap();

        let ev3 = CuentaCreada::nuevo(
            String::from("evento-03"),
            String::from("2026-10-21"),
            String::from("pc-local"),
            String::from("cta-03"),
            String::from("Billetera cripto"),
            TipoCuenta::BilleteraDigital,
            CodigoMoneda::USDT,
            Decimal::from_str("50.00").unwrap(),
        )
        .unwrap();

        let cuentas = LibroPersonal::proyectar_cuentas(&[ev1, ev2, ev3]).unwrap();

        assert_eq!(cuentas.len(), 3);
        assert_eq!(cuentas[0].nombre(), "Cartera");
        assert_eq!(cuentas[1].nombre(), "Cuenta nomina");
        assert_eq!(cuentas[2].nombre(), "Billetera cripto");
    }

    #[test]
    fn garantiza_idempotencia_al_aplicar_mismo_evento_dos_veces() {
        // Verifica que aplicar dos veces el mismo evento no duplique estado
        let saldo_usd = Decimal::from_str("300.00").unwrap();
        let saldo_ves = Decimal::from_str("1250.50").unwrap();

        let evento_usd = CuentaCreada::nuevo(
            String::from("evento-unico-01"),
            String::from("2026-10-21"),
            String::from("pc-local"),
            String::from("cta-01"),
            String::from("Cuenta nomina"),
            TipoCuenta::Banco,
            CodigoMoneda::USD,
            saldo_usd,
        )
        .unwrap();

        let evento_ves = CuentaCreada::nuevo(
            String::from("evento-unico-02"),
            String::from("2026-10-21"),
            String::from("pc-local"),
            String::from("cta-02"),
            String::from("Cartera"),
            TipoCuenta::Efectivo,
            CodigoMoneda::VES,
            saldo_ves,
        )
        .unwrap();

        // Aplica cada evento una vez en el primer libro
        let mut libro_una_vez = LibroPersonal::nuevo();
        libro_una_vez
            .aplicar_eventos(&[evento_usd.clone(), evento_ves.clone()])
            .unwrap();

        // Aplica dos veces los mismos eventos en el segundo libro
        let mut libro_dos_veces = LibroPersonal::nuevo();
        libro_dos_veces
            .aplicar_cuenta_creada(evento_usd.clone())
            .unwrap();
        libro_dos_veces
            .aplicar_cuenta_creada(evento_usd.clone())
            .unwrap();
        libro_dos_veces
            .aplicar_eventos(&[evento_ves.clone(), evento_usd, evento_ves])
            .unwrap();

        assert_eq!(libro_dos_veces, libro_una_vez);
        assert_eq!(libro_dos_veces.cuentas(), libro_una_vez.cuentas());
        assert_eq!(libro_dos_veces.cuentas().len(), 2);
        assert_eq!(libro_dos_veces.eventos().len(), 2);
        assert_eq!(
            libro_dos_veces.cuentas()[0].saldo_actual().cantidad(),
            saldo_usd
        );
        assert_eq!(
            libro_dos_veces.cuentas()[1].saldo_actual().cantidad(),
            saldo_ves
        );
    }

    #[test]
    fn rechaza_evento_nuevo_con_nombre_de_cuenta_duplicado() {
        // Verifica que un evento nuevo con nombre repetido sea rechazado
        let ev1 = CuentaCreada::nuevo(
            String::from("evento-01"),
            String::from("2026-10-21"),
            String::from("pc-local"),
            String::from("cta-01"),
            String::from("Cartera"),
            TipoCuenta::Efectivo,
            CodigoMoneda::VES,
            Decimal::ZERO,
        )
        .unwrap();

        let ev2_nombre_repetido = CuentaCreada::nuevo(
            String::from("evento-02"),
            String::from("2026-10-21"),
            String::from("pc-local"),
            String::from("cta-02"),
            String::from(" cArTeRa "),
            TipoCuenta::Banco,
            CodigoMoneda::USD,
            Decimal::ZERO,
        )
        .unwrap();

        let mut libro = LibroPersonal::desde_eventos(&[ev1]).unwrap();
        let error = libro
            .aplicar_cuenta_creada(ev2_nombre_repetido)
            .unwrap_err();

        assert_eq!(
            error,
            ErrorEvento::CuentaInvalida(ErrorCuenta::NombreDuplicado(String::from("cArTeRa")))
        );
        assert!(error.to_string().contains("nombre"));
        assert_eq!(libro.cuentas().len(), 1);
        assert_eq!(libro.eventos().len(), 1);
    }
}
