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
    //pub datos: DatosCuentaCreada,
    pub datos: serde_json::Value, // acepta cualquier estructura en JSON
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatosMovimientoRegistrado {
    pub movimiento_id: String,
    pub cuenta_id: String,
    pub tipo_movimiento: String,
    pub monto: Decimal,
    pub categoria: String,
    pub nota: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatosMovimientoCorregido {
    pub ajuste_id: String,
    pub movimiento_original_id: String,
    pub cuenta_id: String,
    pub monto_ajuste: Decimal,
}

pub trait RepositorioEventos {
    fn guardar_evento(&mut self, evento: &EventoLibro) -> Result<(), String>;
    fn obtener_eventos(&self) -> Result<Vec<EventoLibro>, String>;
}

pub fn proyectar_cuentas(eventos: &[EventoLibro]) -> Vec<Cuenta> {
    let mut eventos_aplicados = HashSet::new();
    let mut mapa_cuentas: BTreeMap<String, Cuenta> = BTreeMap::new();
    let mut saldos_acumulados: BTreeMap<String, Decimal> = BTreeMap::new();

    for ev in eventos {
        if !eventos_aplicados.insert(ev.id.clone()) {
            continue;
        }

        if ev.tipo == "CuentaCreada" {
            if let Ok(datos) = serde_json::from_value::<DatosCuentaCreada>(ev.datos.clone()) {
                let monto_inicial = datos.saldo_inicial.cantidad();
                saldos_acumulados.insert(datos.cuenta_id.clone(), monto_inicial);
                
                mapa_cuentas.insert(
                    datos.cuenta_id.clone(),
                    Cuenta {
                        id: datos.cuenta_id,
                        nombre: datos.nombre,
                        tipo: datos.tipo,
                        moneda: datos.saldo_inicial.moneda().to_string(),
                        saldo: datos.saldo_inicial,
                    },
                );
            }
        } else if ev.tipo == "MovimientoRegistrado" {
            if let Ok(datos) = serde_json::from_value::<DatosMovimientoRegistrado>(ev.datos.clone()) {
                if let Some(saldo_actual) = saldos_acumulados.get_mut(&datos.cuenta_id) {
                    if datos.tipo_movimiento == "ingreso" {
                        *saldo_actual += datos.monto;
                    } else if datos.tipo_movimiento == "gasto" {
                        *saldo_actual -= datos.monto;
                    }
                }
            }
        } else if ev.tipo == "MovimientoCorregido" {
            if let Ok(datos) = serde_json::from_value::<DatosMovimientoCorregido>(ev.datos.clone()) {
                if let Some(saldo_actual) = saldos_acumulados.get_mut(&datos.cuenta_id) {
                    *saldo_actual += datos.monto_ajuste;
                }
            }
        }
    }

    for (cuenta_id, cuenta) in mapa_cuentas.iter_mut() {
        if let Some(&nuevo_total) = saldos_acumulados.get(cuenta_id) {
            if let Ok(monto_actualizado) = Monto::nuevo(nuevo_total, cuenta.moneda()) {
                cuenta.saldo = monto_actualizado;
            }
        }
    }

    mapa_cuentas.into_values().collect()
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
        datos: serde_json::to_value(DatosCuentaCreada {
            cuenta_id: cuenta_id.clone(),
            nombre: nombre_limpio.to_string(),
            tipo: tipo.clone(),
            saldo_inicial: monto_inicial.clone(),
        }).unwrap(),
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

pub fn ejecutar_corregir_movimiento<R: RepositorioEventos>(
    repo: &mut R,
    cuenta_id: &str,
    movimiento_original_id: &str,
    monto_correcto_str: &str,
    _monto_original: Decimal, 
    dispositivo_id: &str,
) -> Result<(), ErrorDominio> {
    let monto_correcto = Decimal::from_str(monto_correcto_str.trim().replace(',', ".").as_str())
        .map_err(|_| ErrorDominio::nuevo("monto", "El monto correcto debe ser un número válido."))?;

    if monto_correcto < Decimal::ZERO {
        return Err(ErrorDominio::nuevo("monto", "El monto corregido no puede ser negativo."));
    }

    // 1. Buscamos en el repositorio los eventos para validar que el movimiento original exista y saber su tipo
    let eventos = repo
        .obtener_eventos()
        .map_err(|_| ErrorDominio::nuevo("general", "No se pudo leer el almacenamiento local."))?;

    let mut monto_original = Decimal::ZERO;
    let mut tipo_movimiento = String::new();
    let mut encontrado = false;

    for ev in &eventos {
        if ev.tipo == "MovimientoRegistrado" {
            if let Ok(datos) = serde_json::from_value::<DatosMovimientoRegistrado>(ev.datos.clone()) {
                if datos.movimiento_id == movimiento_original_id && datos.cuenta_id == cuenta_id {
                    monto_original = datos.monto;
                    tipo_movimiento = datos.tipo_movimiento;
                    encontrado = true;
                    break;
                }
            }
        }
    }

    if !encontrado {
        return Err(ErrorDominio::nuevo("movimiento", "El movimiento a corregir no existe."));
    }

    // 2. Calculamos la diferencia con el signo algebraico correcto para el saldo:
    // - Si es un Ingreso: (Nuevo - Original) Ej: 700 - 800 = -100 (resta al saldo)
    // - Si es un Gasto: (Original - Nuevo) -Ej: 500 - 50 = +450 (devuelve saldo a la cuenta)
    let delta = if tipo_movimiento == "ingreso" {
        monto_correcto - monto_original
    } else {
        monto_original - monto_correcto
    };

    let ajuste_id = uuid::Uuid::new_v4().to_string();
    let evento = EventoLibro {
        id: uuid::Uuid::new_v4().to_string(),
        tipo: "MovimientoCorregido".to_string(),
        momento_registro: Utc::now(),
        dispositivo_origen: dispositivo_id.to_string(),
        datos: serde_json::to_value(DatosMovimientoCorregido {
            ajuste_id,
            movimiento_original_id: movimiento_original_id.to_string(),
            cuenta_id: cuenta_id.to_string(),
            monto_ajuste: delta,
        }).unwrap(),
    };

    repo.guardar_evento(&evento)
        .map_err(|_| ErrorDominio::nuevo("general", "No se pudo registrar la corrección en el almacenamiento local."))?;

    Ok(())
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

// Prueba unitaria Maikel - Entrega 3
#[test]
fn probar_error_correccion_ingreso_reportado() {
    let mut repo = RepositorioEnMemoria::default();

    // Paso 1: Saldo inicial de 1.000,00 USD
    let cuenta = ejecutar_crear_cuenta(
        &mut repo, "Billetera dolares", "billetera digital", "USD", "1000.00", "prueba"
    ).unwrap();
    let cuenta_id = cuenta.id().to_string();

    // Paso 2: Registrar un ingreso de 800,00 
    // (Saldo intermedio esperado: 1000 + 800 = 1800.00)
    let mov_ingreso_id = "ingreso-original".to_string();
    let ev_ingreso = EventoLibro {
        id: uuid::Uuid::new_v4().to_string(),
        tipo: "MovimientoRegistrado".to_string(),
        momento_registro: Utc::now(),
        dispositivo_origen: "prueba".to_string(),
        datos: serde_json::to_value(DatosMovimientoRegistrado {
            movimiento_id: mov_ingreso_id.clone(),
            cuenta_id: cuenta_id.clone(),
            tipo_movimiento: "ingreso".to_string(),
            monto: Decimal::from_str("800.00").unwrap(),
            categoria: "Salario".to_string(),
            nota: Some("Pago inicial".into()),
        }).unwrap(),
    };
    repo.guardar_evento(&ev_ingreso).unwrap();

    // Paso 3: Corregir ese ingreso de 800,00 a 700,00 
    ejecutar_corregir_movimiento(
        &mut repo,
        &cuenta_id,
        &mov_ingreso_id,
        "700.00",
        Decimal::from_str("800.00").unwrap(),
        "prueba"
    ).unwrap();

    // Paso 4: Proyectar y verificar el saldo final exacto
    let cuentas_finales = proyectar_cuentas(&repo.obtener_eventos().unwrap());
    
    // CASO CRÍTICO: 
    // - 1000 de saldo inicial.
    // - Entraron 800 (iba en 1800).
    // - Se corrigió el ingreso a 700 (le quitaste 100 al ingreso).
    // - El saldo final obligatorio debe ser 1700.00.
    assert_eq!(cuentas_finales[0].saldo().cantidad().to_string(), "1700.00");
}

// Prueba unitaria Maikel - Entrega 3 - Correción de un gasto
#[test]
fn probar_correccion_gasto_reportado() {
    let mut repo = RepositorioEnMemoria::default();

    // Paso 1: Saldo inicial de 1.000,00 USD
    let cuenta = ejecutar_crear_cuenta(
        &mut repo, "Billetera banco", "banco", "USD", "1000.00", "prueba"
    ).unwrap();
    let cuenta_id = cuenta.id().to_string();

    // Paso 2: Registrar un gasto erróneo de 500,00 
    // (Saldo intermedio esperado: 1000 - 500 = 500.00)
    let mov_gasto_id = "gasto-original".to_string();
    let ev_gasto = EventoLibro {
        id: uuid::Uuid::new_v4().to_string(),
        tipo: "MovimientoRegistrado".to_string(),
        momento_registro: Utc::now(),
        dispositivo_origen: "prueba".to_string(),
        datos: serde_json::to_value(DatosMovimientoRegistrado {
            movimiento_id: mov_gasto_id.clone(),
            cuenta_id: cuenta_id.clone(),
            tipo_movimiento: "gasto".to_string(),
            monto: Decimal::from_str("500.00").unwrap(),
            categoria: "Supermercado".to_string(),
            nota: Some("Mercado caro".into()),
        }).unwrap(),
    };
    repo.guardar_evento(&ev_gasto).unwrap();

    // Paso 3: Corregir ese gasto de 500,00 a 50,00 
    // (Como el gasto disminuyó, la cuenta debe recuperar los 450.00 restados de más)
    ejecutar_corregir_movimiento(
        &mut repo,
        &cuenta_id,
        &mov_gasto_id,
        "50.00",
        Decimal::from_str("500.00").unwrap(),
        "prueba"
    ).unwrap();

    // Paso 4: Proyectar y verificar el saldo final exacto
    let cuentas_finales = proyectar_cuentas(&repo.obtener_eventos().unwrap());
    
    // CASO CRÍTICO DE GASTO: 
    // - Se tenia 1000 de saldo inicial.
    // - Se gastó 500 (iba en 500).
    // - Se corrigió el gasto a 50 (le devolviste 450 a la cuenta).
    // - El saldo final obligatorio debe ser 950.00.
    assert_eq!(cuentas_finales[0].saldo().cantidad().to_string(), "950.00");
}