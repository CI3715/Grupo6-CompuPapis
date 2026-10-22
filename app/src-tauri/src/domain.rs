use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::str::FromStr;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Moneda {
    pub codigo: String,
    pub nombre: String,
    pub simbolo: String,
    pub decimales: u32,
}

pub fn catalogo_monedas() -> Vec<Moneda> {
    vec![
        Moneda { codigo: "VES".into(), nombre: "Bolívar".into(), simbolo: "Bs.".into(), decimales: 2 },
        Moneda { codigo: "USD".into(), nombre: "Dólar estadounidense".into(), simbolo: "$".into(), decimales: 2 },
        Moneda { codigo: "EUR".into(), nombre: "Euro".into(), simbolo: "€".into(), decimales: 2 },
        Moneda { codigo: "USDT".into(), nombre: "Tether (dólar digital)".into(), simbolo: "USDT".into(), decimales: 2 },
    ]
}

pub fn buscar_moneda(codigo: &str) -> Result<Moneda, ErrorDominio> {
    catalogo_monedas()
        .into_iter()
        .find(|m| m.codigo == codigo.trim().to_uppercase())
        .ok_or_else(|| ErrorDominio::nuevo("moneda", "La moneda seleccionada no pertenece al catálogo válido."))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Monto {
    cantidad: Decimal,
    moneda: String,
}

impl Monto {
    pub fn nuevo(cantidad: Decimal, moneda_codigo: &str) -> Result<Self, ErrorDominio> {
        let moneda = buscar_moneda(moneda_codigo)?;
        if cantidad.scale() > moneda.decimales {
            return Err(ErrorDominio::nuevo(
                "saldo_inicial",
                &format!("El monto no puede tener más de {} decimales para {}.", moneda.decimales, moneda.codigo),
            ));
        }
        Ok(Self {
            cantidad: cantidad.round_dp(moneda.decimales),
            moneda: moneda.codigo,
        })
    }

    pub fn desde_str(valor_str: &str, moneda_codigo: &str) -> Result<Self, ErrorDominio> {
        let limpio = valor_str.trim().replace(',', ".");
        if limpio.is_empty() {
            return Err(ErrorDominio::nuevo("saldo_inicial", "El saldo inicial es obligatorio."));
        }
        let dec = Decimal::from_str(&limpio).map_err(|_| {
            ErrorDominio::nuevo("saldo_inicial", "El saldo inicial debe ser un número decimal válido.")
        })?;

        if dec < Decimal::ZERO {
            return Err(ErrorDominio::nuevo("saldo_inicial", "El saldo inicial no puede ser negativo."));
        }

        Self::nuevo(dec, moneda_codigo)
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
        Self::nuevo(self.cantidad + otro.cantidad, &self.moneda)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TipoCuenta {
    #[serde(rename = "efectivo")]
    Efectivo,
    #[serde(rename = "banco")]
    Banco,
    #[serde(rename = "billetera digital")]
    BilleteraDigital,
}

impl TipoCuenta {
    pub fn desde_str(s: &str) -> Result<Self, ErrorDominio> {
        match s.trim().to_lowercase().as_str() {
            "efectivo" => Ok(TipoCuenta::Efectivo),
            "banco" => Ok(TipoCuenta::Banco),
            "billetera digital" | "billetera" => Ok(TipoCuenta::BilleteraDigital),
            _ => Err(ErrorDominio::nuevo(
                "tipo",
                "El tipo de cuenta debe ser efectivo, banco o billetera digital.",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Cuenta {
    id: String,
    nombre: String,
    tipo: TipoCuenta,
    moneda: String,
    saldo: Monto,
}

impl Cuenta {
    pub fn id(&self) -> &str { &self.id }
    pub fn nombre(&self) -> &str { &self.nombre }
    pub fn tipo(&self) -> &TipoCuenta { &self.tipo }
    pub fn moneda(&self) -> &str { &self.moneda }
    pub fn saldo(&self) -> &Monto { &self.saldo }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatosCuentaCreada {
    pub cuenta_id: String,
    pub nombre: String,
    pub tipo: TipoCuenta,
    pub saldo_inicial: Monto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventoLibro {
    pub id: String,
    pub tipo: String,
    pub momento_registro: DateTime<Utc>,
    pub dispositivo_origen: String,
    pub datos: DatosCuentaCreada,
}

pub trait RepositorioEventos {
    fn guardar_evento(&mut self, evento: &EventoLibro) -> Result<(), String>;
    fn obtener_eventos(&self) -> Result<Vec<EventoLibro>, String>;
}

pub fn proyectar_cuentas(eventos: &[EventoLibro]) -> Vec<Cuenta> {
    let mut eventos_aplicados = HashSet::new();
    let mut cuentas = Vec::new();

    for ev in eventos {
        if !eventos_aplicados.insert(ev.id.clone()) {
            continue;
        }
        if ev.tipo == "CuentaCreada" {
            cuentas.push(Cuenta {
                id: ev.datos.cuenta_id.clone(),
                nombre: ev.datos.nombre.clone(),
                tipo: ev.datos.tipo.clone(),
                moneda: ev.datos.saldo_inicial.moneda().to_string(),
                saldo: ev.datos.saldo_inicial.clone(),
            });
        }
    }
    cuentas
}

pub fn ejecutar_crear_cuenta<R: RepositorioEventos>(
    repo: &mut R,
    nombre: &str,
    tipo_str: &str,
    moneda_str: &str,
    saldo_inicial_str: &str,
    dispositivo_id: &str,
) -> Result<Cuenta, ErrorDominio> {
    let nombre_limpio = nombre.trim();
    if nombre_limpio.is_empty() {
        return Err(ErrorDominio::nuevo("nombre", "El nombre de la cuenta no puede estar vacío."));
    }

    let tipo = TipoCuenta::desde_str(tipo_str)?;
    let _moneda = buscar_moneda(moneda_str)?;
    let monto_inicial = Monto::desde_str(saldo_inicial_str, moneda_str)?;

    let eventos = repo
        .obtener_eventos()
        .map_err(|_| ErrorDominio::nuevo("general", "No se pudo leer el almacenamiento local."))?;
    let cuentas_actuales = proyectar_cuentas(&eventos);

    if cuentas_actuales
        .iter()
        .any(|c| c.nombre().to_lowercase() == nombre_limpio.to_lowercase())
    {
        return Err(ErrorDominio::nuevo("nombre", "Ya existe una cuenta con ese nombre."));
    }

    let cuenta_id = uuid::Uuid::new_v4().to_string();
    let evento = EventoLibro {
        id: uuid::Uuid::new_v4().to_string(),
        tipo: "CuentaCreada".to_string(),
        momento_registro: Utc::now(),
        dispositivo_origen: dispositivo_id.to_string(),
        datos: DatosCuentaCreada {
            cuenta_id: cuenta_id.clone(),
            nombre: nombre_limpio.to_string(),
            tipo: tipo.clone(),
            saldo_inicial: monto_inicial.clone(),
        },
    };

    repo.guardar_evento(&evento)
        .map_err(|_| ErrorDominio::nuevo("general", "No se pudo guardar la cuenta en el dispositivo."))?;

    Ok(Cuenta {
        id: cuenta_id,
        nombre: nombre_limpio.to_string(),
        tipo,
        moneda: monto_inicial.moneda().to_string(),
        saldo: monto_inicial,
    })
}

#[derive(Debug, Serialize)]
pub struct SubtotalMoneda {
    pub codigo: String,
    pub simbolo: String,
    pub subtotal: String,
}

#[derive(Debug, Serialize)]
pub struct ResumenCuentas {
    pub cuentas: Vec<Cuenta>,
    pub subtotales: Vec<SubtotalMoneda>,
}

pub fn ejecutar_listar_cuentas<R: RepositorioEventos>(repo: &R) -> Result<ResumenCuentas, ErrorDominio> {
    let eventos = repo
        .obtener_eventos()
        .map_err(|_| ErrorDominio::nuevo("general", "No se pudieron cargar las cuentas locales."))?;
    let cuentas = proyectar_cuentas(&eventos);

    let mut mapa_subtotales: BTreeMap<String, Monto> = BTreeMap::new();
    for cuenta in &cuentas {
        let codigo = cuenta.moneda().to_string();
        if let Some(acumulado) = mapa_subtotales.get(&codigo) {
            let nuevo = acumulado.sumar(cuenta.saldo())?;
            mapa_subtotales.insert(codigo, nuevo);
        } else {
            mapa_subtotales.insert(codigo, cuenta.saldo().clone());
        }
    }

    let mut subtotales = Vec::new();
    for (codigo, monto) in mapa_subtotales {
        let info_moneda = buscar_moneda(&codigo)?;
        subtotales.push(SubtotalMoneda {
            codigo,
            simbolo: info_moneda.simbolo,
            subtotal: format!("{:.2}", monto.cantidad()),
        });
    }

    Ok(ResumenCuentas { cuentas, subtotales })
}

#[derive(Default)]
pub struct RepositorioEnMemoria {
    eventos: Vec<EventoLibro>,
}

impl RepositorioEventos for RepositorioEnMemoria {
    fn guardar_evento(&mut self, evento: &EventoLibro) -> Result<(), String> {
        if !self.eventos.iter().any(|e| e.id == evento.id) {
            self.eventos.push(evento.clone());
        }
        Ok(())
    }

    fn obtener_eventos(&self) -> Result<Vec<EventoLibro>, String> {
        Ok(self.eventos.clone())
    }
}

#[test]
fn probar_crear_cuentas() {
    let mut repo = RepositorioEnMemoria::default();

    // Creamos una cuenta
    let cuenta = ejecutar_crear_cuenta(
        &mut repo, "Cartera", "efectivo", "VES", "100.25", "prueba"
    ).unwrap();

    // Vemos si los datos estan bien
    assert_eq!(cuenta.moneda(), "VES");
    assert_eq!(cuenta.saldo().moneda(), "VES");
    assert_eq!(cuenta.saldo().cantidad().to_string(), "100.25");

    // Revisamos los casos que deben dar error
    let casos = [
        ("", "efectivo", "VES", "100", "nombre"),
        ("   ", "efectivo", "VES", "100", "nombre"),
        ("CARTERA", "efectivo", "VES", "100", "nombre"),
        ("Otra", "tarjeta", "VES", "100", "tipo"),
        ("Otra", "efectivo", "ABC", "100", "moneda"),
        ("Otra", "efectivo", "VES", "-10", "saldo_inicial"),
        ("Otra", "efectivo", "VES", "10.555", "saldo_inicial"),
    ];

    for (nombre, tipo, moneda, saldo, campo) in casos {
        let resp = ejecutar_crear_cuenta(
            &mut repo, nombre, tipo, moneda, saldo, "prueba"
        );

        assert!(resp.is_err());
        assert_eq!(resp.unwrap_err().campo, campo);
    }

    // Solo se debe guardar la primera cuenta
    assert_eq!(repo.obtener_eventos().unwrap().len(), 1);

    // Creamos las otras cuentas
    ejecutar_crear_cuenta(
        &mut repo, "Banco", "banco", "USD", "0.10", "prueba"
    ).unwrap();

    ejecutar_crear_cuenta(
        &mut repo, "Ahorros", "banco", "USD", "0.20", "prueba"
    ).unwrap();

    ejecutar_crear_cuenta(
        &mut repo, "Billetera", "billetera digital", "USDT", "0", "prueba"
    ).unwrap();

    // Vemos las cuentas y los subtotales
    let lista = ejecutar_listar_cuentas(&repo).unwrap();

    assert_eq!(lista.cuentas.len(), 4);
    assert_eq!(lista.subtotales.len(), 3);

    let usd = lista.subtotales.iter()
        .find(|dato| dato.codigo == "USD").unwrap();

    assert_eq!(usd.subtotal, "0.30");

    // Vemos si la cuenta sigue usando VES
    let cartera = lista.cuentas.iter()
        .find(|c| c.nombre() == "Cartera").unwrap();

    assert_eq!(cartera.moneda(), "VES");
    assert_eq!(cartera.saldo().moneda(), "VES");

    // Agregamos el mismo evento otra vez
    let eventos = repo.obtener_eventos().unwrap();
    let mut repetidos = eventos.clone();
    repetidos.push(eventos[0].clone());

    assert_eq!(proyectar_cuentas(&repetidos).len(), 4);
}