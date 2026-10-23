use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::str::FromStr;

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

// ==========================================
// MONTO (INV-01, INV-02, INV-03)
// ==========================================
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
                "monto",
                &format!("El monto no puede tener más de {} decimales para {}.", moneda.decimales, moneda.codigo),
            ));
        }
        Ok(Self {
            cantidad: cantidad.round_dp(moneda.decimales),
            moneda: moneda.codigo,
        })
    }

    pub fn desde_str_con_campo(valor_str: &str, moneda_codigo: &str, campo_err: &str, permitir_cero: bool) -> Result<Self, ErrorDominio> {
        let limpio = valor_str.trim().replace(',', ".");
        if limpio.is_empty() {
            return Err(ErrorDominio::nuevo(campo_err, "El monto es obligatorio."));
        }
        let dec = Decimal::from_str(&limpio).map_err(|_| {
            ErrorDominio::nuevo(campo_err, "El monto debe ser un número decimal válido.")
        })?;

        if permitir_cero {
            if dec < Decimal::ZERO {
                return Err(ErrorDominio::nuevo(campo_err, "El monto no puede ser negativo."));
            }
        } else if dec <= Decimal::ZERO {
            return Err(ErrorDominio::nuevo(campo_err, "El monto debe ser mayor que cero."));
        }

        let moneda = buscar_moneda(moneda_codigo)?;
        if dec.scale() > moneda.decimales {
            return Err(ErrorDominio::nuevo(
                campo_err,
                &format!("El monto no puede tener más de {} decimales para {}.", moneda.decimales, moneda.codigo),
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

// ==========================================
// CUENTA Y CATEGORÍA (RF-LIB-01, RF-LIB-07)
// ==========================================
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
            _ => Err(ErrorDominio::nuevo("tipo", "El tipo de cuenta debe ser efectivo, banco o billetera digital.")),
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TipoMovimiento {
    #[serde(rename = "ingreso")]
    Ingreso,
    #[serde(rename = "gasto")]
    Gasto,
}

impl TipoMovimiento {
    pub fn desde_str(s: &str) -> Result<Self, ErrorDominio> {
        match s.trim().to_lowercase().as_str() {
            "ingreso" => Ok(TipoMovimiento::Ingreso),
            "gasto" => Ok(TipoMovimiento::Gasto),
            _ => Err(ErrorDominio::nuevo("tipo", "El tipo de movimiento debe ser ingreso o gasto.")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Categoria {
    pub nombre: String,
    pub tipo: TipoMovimiento,
}

pub fn categorias_de_fabrica() -> Vec<Categoria> {
    let gastos = [
        "Alimentación", "Transporte", "Servicios", "Salud",
        "Educación", "Entretenimiento", "Otros gastos",
    ];
    let ingresos = ["Salario", "Ventas", "Regalos", "Otros ingresos"];

    let mut lista = Vec::new();
    for g in gastos {
        lista.push(Categoria { nombre: g.to_string(), tipo: TipoMovimiento::Gasto });
    }
    for i in ingresos {
        lista.push(Categoria { nombre: i.to_string(), tipo: TipoMovimiento::Ingreso });
    }
    lista
}

// ==========================================
// EVENTOS INMUTABLES DEL LIBRO (Sección 2.4, INV-04, INV-13)
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatosCuentaCreada {
    pub cuenta_id: String,
    pub nombre: String,
    pub tipo: TipoCuenta,
    pub saldo_inicial: Monto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatosCategoriaCreada {
    pub nombre: String,
    pub tipo: TipoMovimiento,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatosMovimientoRegistrado {
    pub movimiento_id: String,
    pub cuenta_id: String,
    pub tipo: TipoMovimiento,
    pub monto: Monto,
    pub fecha: String, // YYYY-MM-DD
    pub categoria: String,
    pub nota: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatosMovimientoCorregido {
    pub ajuste_id: String,
    pub movimiento_original_id: String,
    pub cuenta_id: String,
    pub monto_correcto: Monto,
    pub diferencia: Decimal, // Con signo sobre el saldo (+ sube el saldo, - baja el saldo)
    pub fecha: String,
    pub nota: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventoLibro {
    pub id: String,
    pub tipo: String,
    pub momento_registro: DateTime<Utc>,
    pub dispositivo_origen: String,
    pub datos_json: String,
}

pub trait RepositorioEventos {
    fn guardar_evento(&mut self, evento: &EventoLibro) -> Result<(), String>;
    fn obtener_eventos(&self) -> Result<Vec<EventoLibro>, String>;
}

// ==========================================
// PROYECCIÓN DEL LIBRO (INV-04, INV-05, INV-13)
// ==========================================
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MovimientoVista {
    pub id: String,
    pub fecha: String,
    pub tipo: String, // "ingreso" | "gasto" | "ajuste"
    pub categoria: String,
    pub nota: Option<String>,
    pub monto: String, // Con signo de efecto en cuenta, ej: "800.00", "-500.00", "450.00"
    pub monto_vigente: String, // Para mostrar al corregir
    pub corregido: bool,
    pub es_ajuste: bool,
    pub referencia_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposicionSaldo {
    pub inicial: String,
    pub ingresos: String,
    pub gastos: String,
    pub ajustes: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetalleCuenta {
    pub saldo_actual: String,
    pub composicion: ComposicionSaldo,
    pub movimientos: Vec<MovimientoVista>,
}

struct EstadoCuentaInterno {
    id: String,
    nombre: String,
    tipo: TipoCuenta,
    moneda: String,
    inicial: Decimal,
    ingresos: Decimal,
    gastos: Decimal,
    ajustes: Decimal,
    movimientos: Vec<MovimientoVista>,
    // Mapa movimiento_original_id -> (TipoMovimiento, monto_vigente_positivo, categoria, fecha)
    originales: HashMap<String, (TipoMovimiento, Decimal, String, String)>,
}

impl EstadoCuentaInterno {
    // INV-05: Saldo actual = saldo inicial + ingresos - gastos ± ajustes
    fn saldo_actual(&self) -> Decimal {
        self.inicial + self.ingresos - self.gastos + self.ajustes
    }
}

struct ProyeccionLibro {
    cuentas_orden: Vec<String>,
    cuentas: HashMap<String, EstadoCuentaInterno>,
    categorias: Vec<Categoria>,
}

fn proyectar_libro(eventos: &[EventoLibro]) -> ProyeccionLibro {
    let mut eventos_aplicados = HashSet::new();
    let mut cuentas_orden = Vec::new();
    let mut cuentas: HashMap<String, EstadoCuentaInterno> = HashMap::new();
    let mut categorias = categorias_de_fabrica();

    for ev in eventos {
        // INV-13: Aplicar dos veces el mismo evento produce el mismo estado
        if !eventos_aplicados.insert(ev.id.clone()) {
            continue;
        }

        match ev.tipo.as_str() {
            "CuentaCreada" => {
                if let Ok(d) = serde_json::from_str::<DatosCuentaCreada>(&ev.datos_json) {
                    let moneda = d.saldo_inicial.moneda().to_string();
                    let inicial = d.saldo_inicial.cantidad();
                    cuentas_orden.push(d.cuenta_id.clone());
                    cuentas.insert(
                        d.cuenta_id.clone(),
                        EstadoCuentaInterno {
                            id: d.cuenta_id,
                            nombre: d.nombre,
                            tipo: d.tipo,
                            moneda,
                            inicial,
                            ingresos: Decimal::ZERO,
                            gastos: Decimal::ZERO,
                            ajustes: Decimal::ZERO,
                            movimientos: Vec::new(),
                            originales: HashMap::new(),
                        },
                    );
                }
            }
            "CategoriaCreada" => {
                if let Ok(d) = serde_json::from_str::<DatosCategoriaCreada>(&ev.datos_json) {
                    if !categorias.iter().any(|c| c.tipo == d.tipo && c.nombre.to_lowercase() == d.nombre.to_lowercase()) {
                        categorias.push(Categoria { nombre: d.nombre, tipo: d.tipo });
                    }
                }
            }
            "MovimientoRegistrado" => {
                if let Ok(d) = serde_json::from_str::<DatosMovimientoRegistrado>(&ev.datos_json) {
                    if let Some(cta) = cuentas.get_mut(&d.cuenta_id) {
                        let cant = d.monto.cantidad();
                        let (tipo_str, monto_con_signo) = match d.tipo {
                            TipoMovimiento::Ingreso => {
                                cta.ingresos += cant;
                                ("ingreso".to_string(), format!("{:.2}", cant))
                            }
                            TipoMovimiento::Gasto => {
                                cta.gastos += cant;
                                ("gasto".to_string(), format!("-{:.2}", cant))
                            }
                        };

                        cta.originales.insert(
                            d.movimiento_id.clone(),
                            (d.tipo.clone(), cant, d.categoria.clone(), d.fecha.clone()),
                        );

                        cta.movimientos.push(MovimientoVista {
                            id: d.movimiento_id,
                            fecha: d.fecha,
                            tipo: tipo_str,
                            categoria: d.categoria,
                            nota: d.nota,
                            monto: monto_con_signo,
                            monto_vigente: format!("{:.2}", cant),
                            corregido: false,
                            es_ajuste: false,
                            referencia_id: None,
                        });
                    }
                }
            }
            "MovimientoCorregido" => {
                if let Ok(d) = serde_json::from_str::<DatosMovimientoCorregido>(&ev.datos_json) {
                    if let Some(cta) = cuentas.get_mut(&d.cuenta_id) {
                        cta.ajustes += d.diferencia;
                        let nuevo_vigente = d.monto_correcto.cantidad();

                        let mut cat_orig = "Ajuste".to_string();
                        if let Some(orig) = cta.originales.get_mut(&d.movimiento_original_id) {
                            orig.1 = nuevo_vigente;
                            cat_orig = orig.2.clone();
                        }

                        for mov in &mut cta.movimientos {
                            if mov.id == d.movimiento_original_id {
                                mov.corregido = true;
                                mov.monto_vigente = format!("{:.2}", nuevo_vigente);
                            }
                        }

                        cta.movimientos.push(MovimientoVista {
                            id: d.ajuste_id,
                            fecha: d.fecha,
                            tipo: "ajuste".to_string(),
                            categoria: cat_orig,
                            nota: Some(d.nota),
                            monto: format!("{:.2}", d.diferencia),
                            monto_vigente: format!("{:.2}", nuevo_vigente),
                            corregido: false,
                            es_ajuste: true,
                            referencia_id: Some(d.movimiento_original_id),
                        });
                    }
                }
            }
            _ => {}
        }
    }

    ProyeccionLibro {
        cuentas_orden,
        cuentas,
        categorias,
    }
}

// ==========================================
// CASOS DE USO
// ==========================================
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
    let proyeccion = proyectar_libro(&eventos);

    if proyeccion
        .cuentas
        .values()
        .any(|c| c.nombre.to_lowercase() == nombre_limpio.to_lowercase())
    {
        return Err(ErrorDominio::nuevo("nombre", "Ya existe una cuenta con ese nombre."));
    }

    let cuenta_id = uuid::Uuid::new_v4().to_string();
    let datos = DatosCuentaCreada {
        cuenta_id: cuenta_id.clone(),
        nombre: nombre_limpio.to_string(),
        tipo: tipo.clone(),
        saldo_inicial: monto_inicial.clone(),
    };

    let evento = EventoLibro {
        id: uuid::Uuid::new_v4().to_string(),
        tipo: "CuentaCreada".to_string(),
        momento_registro: Utc::now(),
        dispositivo_origen: dispositivo_id.to_string(),
        datos_json: serde_json::to_string(&datos).unwrap(),
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

pub fn ejecutar_listar_categorias<R: RepositorioEventos>(repo: &R) -> Result<Vec<Categoria>, ErrorDominio> {
    let eventos = repo
        .obtener_eventos()
        .map_err(|_| ErrorDominio::nuevo("general", "No se pudieron cargar las categorías."))?;
    Ok(proyectar_libro(&eventos).categorias)
}

pub fn ejecutar_crear_categoria<R: RepositorioEventos>(
    repo: &mut R,
    nombre: &str,
    tipo_str: &str,
    dispositivo_id: &str,
) -> Result<Categoria, ErrorDominio> {
    let nombre_limpio = nombre.trim();
    if nombre_limpio.is_empty() {
        return Err(ErrorDominio::nuevo("categoria", "El nombre de la categoría no puede estar vacío."));
    }
    let tipo = TipoMovimiento::desde_str(tipo_str)?;

    let eventos = repo
        .obtener_eventos()
        .map_err(|_| ErrorDominio::nuevo("general", "No se pudo leer el almacenamiento local."))?;
    let proyeccion = proyectar_libro(&eventos);

    if proyeccion
        .categorias
        .iter()
        .any(|c| c.tipo == tipo && c.nombre.to_lowercase() == nombre_limpio.to_lowercase())
    {
        return Err(ErrorDominio::nuevo(
            "categoria",
            "Ya existe una categoría con ese nombre para este tipo de movimiento.",
        ));
    }

    let datos = DatosCategoriaCreada {
        nombre: nombre_limpio.to_string(),
        tipo: tipo.clone(),
    };

    let evento = EventoLibro {
        id: uuid::Uuid::new_v4().to_string(),
        tipo: "CategoriaCreada".to_string(),
        momento_registro: Utc::now(),
        dispositivo_origen: dispositivo_id.to_string(),
        datos_json: serde_json::to_string(&datos).unwrap(),
    };

    repo.guardar_evento(&evento)
        .map_err(|_| ErrorDominio::nuevo("general", "No se pudo guardar la categoría."))?;

    Ok(Categoria {
        nombre: nombre_limpio.to_string(),
        tipo,
    })
}

pub fn ejecutar_registrar_movimiento<R: RepositorioEventos>(
    repo: &mut R,
    cuenta_id: &str,
    tipo_str: &str,
    monto_str: &str,
    fecha_str: &str,
    categoria_str: &str,
    nota_opt: Option<String>,
    dispositivo_id: &str,
) -> Result<DetalleCuenta, ErrorDominio> {
    let eventos = repo
        .obtener_eventos()
        .map_err(|_| ErrorDominio::nuevo("general", "No se pudo leer el libro local."))?;
    let proyeccion = proyectar_libro(&eventos);

    let cta = proyeccion
        .cuentas
        .get(cuenta_id)
        .ok_or_else(|| ErrorDominio::nuevo("cuenta", "La cuenta seleccionada no existe."))?;

    // 1. Validar tipo
    let tipo = TipoMovimiento::desde_str(tipo_str)?;

    // 2. Validar monto (> 0 y con decimales de la moneda de la cuenta)
    let monto = Monto::desde_str_con_campo(monto_str, &cta.moneda, "monto", false)?;

    // 3. Validar fecha (formato YYYY-MM-DD y no futura)
    let fecha_limpia = fecha_str.trim();
    let fecha_parsed = NaiveDate::parse_from_str(fecha_limpia, "%Y-%m-%d")
        .map_err(|_| ErrorDominio::nuevo("fecha", "La fecha debe tener el formato válido AAAA-MM-DD."))?;
    let hoy = Utc::now().date_naive();
    if fecha_parsed > hoy {
        return Err(ErrorDominio::nuevo("fecha", "No se permiten movimientos con fecha futura."));
    }

    // 4. Validar categoría (no vacía, existente y del mismo tipo)
    let cat_limpia = categoria_str.trim();
    if cat_limpia.is_empty() {
        return Err(ErrorDominio::nuevo("categoria", "La categoría es obligatoria."));
    }
    let cat_encontrada = proyeccion
        .categorias
        .iter()
        .find(|c| c.nombre.to_lowercase() == cat_limpia.to_lowercase());

    match cat_encontrada {
        None => {
            return Err(ErrorDominio::nuevo("categoria", "La categoría seleccionada no existe."));
        }
        Some(c) if c.tipo != tipo => {
            return Err(ErrorDominio::nuevo(
                "categoria",
                "La categoría seleccionada no corresponde al tipo de movimiento.",
            ));
        }
        _ => {}
    }

    let cat_nombre_oficial = cat_encontrada.unwrap().nombre.clone();
    let nota_limpia = nota_opt
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty());

    let datos = DatosMovimientoRegistrado {
        movimiento_id: uuid::Uuid::new_v4().to_string(),
        cuenta_id: cuenta_id.to_string(),
        tipo,
        monto,
        fecha: fecha_limpia.to_string(),
        categoria: cat_nombre_oficial,
        nota: nota_limpia,
    };

    let evento = EventoLibro {
        id: uuid::Uuid::new_v4().to_string(),
        tipo: "MovimientoRegistrado".to_string(),
        momento_registro: Utc::now(),
        dispositivo_origen: dispositivo_id.to_string(),
        datos_json: serde_json::to_string(&datos).unwrap(),
    };

    repo.guardar_evento(&evento)
        .map_err(|_| ErrorDominio::nuevo("general", "No se pudo guardar el movimiento."))?;

    ejecutar_obtener_detalle_cuenta(repo, cuenta_id)
}

pub fn ejecutar_corregir_movimiento<R: RepositorioEventos>(
    repo: &mut R,
    cuenta_id: &str,
    movimiento_id: &str,
    monto_correcto_str: &str,
    dispositivo_id: &str,
) -> Result<DetalleCuenta, ErrorDominio> {
    let eventos = repo
        .obtener_eventos()
        .map_err(|_| ErrorDominio::nuevo("general", "No se pudo leer el libro local."))?;
    let proyeccion = proyectar_libro(&eventos);

    let cta = proyeccion
        .cuentas
        .get(cuenta_id)
        .ok_or_else(|| ErrorDominio::nuevo("cuenta", "La cuenta indicada no existe."))?;

    // Verificar que sea un movimiento original y no un ajuste
    let (tipo_orig, monto_vigente, _cat_orig, _fecha_orig) = cta
        .originales
        .get(movimiento_id)
        .ok_or_else(|| ErrorDominio::nuevo("movimiento", "Solo se pueden corregir movimientos originales, no ajustes."))?;

    // El monto correcto puede ser 0 (para anular un movimiento que no debió existir) o positivo
    let monto_correcto = Monto::desde_str_con_campo(monto_correcto_str, &cta.moneda, "monto", true)?;
    let nuevo_valor = monto_correcto.cantidad();

    if nuevo_valor == *monto_vigente {
        return Err(ErrorDominio::nuevo(
            "monto",
            "El monto correcto indicado es igual al monto vigente actual.",
        ));
    }

    // Calcular efecto sobre el saldo según el tipo original:
    // - Si era Ingreso: efecto anterior = +monto_vigente, nuevo = +nuevo_valor => diferencia = nuevo_valor - monto_vigente
    // - Si era Gasto:   efecto anterior = -monto_vigente, nuevo = -nuevo_valor => diferencia = monto_vigente - nuevo_valor
    let diferencia = match tipo_orig {
        TipoMovimiento::Ingreso => nuevo_valor - *monto_vigente,
        TipoMovimiento::Gasto => *monto_vigente - nuevo_valor,
    };

    let hoy_str = Utc::now().date_naive().format("%Y-%m-%d").to_string();
    let nota = format!(
        "Ajuste de corrección (monto anterior: {:.2}, monto correcto: {:.2})",
        monto_vigente, nuevo_valor
    );

    let datos = DatosMovimientoCorregido {
        ajuste_id: uuid::Uuid::new_v4().to_string(),
        movimiento_original_id: movimiento_id.to_string(),
        cuenta_id: cuenta_id.to_string(),
        monto_correcto,
        diferencia,
        fecha: hoy_str,
        nota,
    };

    let evento = EventoLibro {
        id: uuid::Uuid::new_v4().to_string(),
        tipo: "MovimientoCorregido".to_string(),
        momento_registro: Utc::now(),
        dispositivo_origen: dispositivo_id.to_string(),
        datos_json: serde_json::to_string(&datos).unwrap(),
    };

    repo.guardar_evento(&evento)
        .map_err(|_| ErrorDominio::nuevo("general", "No se pudo registrar la corrección."))?;

    ejecutar_obtener_detalle_cuenta(repo, cuenta_id)
}

pub fn ejecutar_obtener_detalle_cuenta<R: RepositorioEventos>(
    repo: &R,
    cuenta_id: &str,
) -> Result<DetalleCuenta, ErrorDominio> {
    let eventos = repo
        .obtener_eventos()
        .map_err(|_| ErrorDominio::nuevo("general", "No se pudo leer el libro local."))?;
    let mut proyeccion = proyectar_libro(&eventos);

    let cta = proyeccion
        .cuentas
        .remove(cuenta_id)
        .ok_or_else(|| ErrorDominio::nuevo("cuenta", "La cuenta no existe."))?;

    let saldo_actual = cta.saldo_actual();
    let mut movimientos = cta.movimientos;
    // Del más reciente al más antiguo (Sección 2.3)
    movimientos.reverse();

    Ok(DetalleCuenta {
        saldo_actual: format!("{:.2}", saldo_actual),
        composicion: ComposicionSaldo {
            inicial: format!("{:.2}", cta.inicial),
            ingresos: format!("{:.2}", cta.ingresos),
            gastos: format!("{:.2}", cta.gastos),
            ajustes: format!("{:.2}", cta.ajustes),
        },
        movimientos,
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
    let proyeccion = proyectar_libro(&eventos);

    let mut cuentas = Vec::new();
    let mut mapa_subtotales: BTreeMap<String, Decimal> = BTreeMap::new();

    for id in &proyeccion.cuentas_orden {
        if let Some(c) = proyeccion.cuentas.get(id) {
            let saldo_actual = c.saldo_actual();
            *mapa_subtotales.entry(c.moneda.clone()).or_insert(Decimal::ZERO) += saldo_actual;

            cuentas.push(Cuenta {
                id: c.id.clone(),
                nombre: c.nombre.clone(),
                tipo: c.tipo.clone(),
                moneda: c.moneda.clone(),
                saldo: Monto {
                    cantidad: saldo_actual.round_dp(2),
                    moneda: c.moneda.clone(),
                },
            });
        }
    }

    let mut subtotales = Vec::new();
    for (codigo, total_dec) in mapa_subtotales {
        let info_moneda = buscar_moneda(&codigo)?;
        subtotales.push(SubtotalMoneda {
            codigo,
            simbolo: info_moneda.simbolo,
            subtotal: format!("{:.2}", total_dec.round_dp(info_moneda.decimales)),
        });
    }

    Ok(ResumenCuentas { cuentas, subtotales })
}

// ==========================================
// ADAPTADOR EN MEMORIA Y PRUEBAS (Sección 3)
// ==========================================
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ejemplo_seccion_2_2_correccion_e_inv_04_inv_05() {
        let mut repo = RepositorioEnMemoria::default();
        let hoy = Utc::now().date_naive().format("%Y-%m-%d").to_string();

        // Paso 1: Cuenta "Billetera dólares" en USD con saldo inicial 1.000,00
        let cta = ejecutar_crear_cuenta(&mut repo, "Billetera dólares", "billetera digital", "USD", "1000.00", "dev-1").unwrap();
        assert_eq!(cta.saldo().cantidad().to_string(), "1000.00");

        // Paso 2: Ingreso Salario +800,00 -> Saldo 1.800,00
        let det = ejecutar_registrar_movimiento(&mut repo, cta.id(), "ingreso", "800.00", &hoy, "Salario", None, "dev-1").unwrap();
        assert_eq!(det.saldo_actual, "1800.00");

        // Paso 3: Gasto Alimentación (era 50, se anotó 500) -> -500,00 -> Saldo 1.300,00
        let det = ejecutar_registrar_movimiento(&mut repo, cta.id(), "gasto", "500.00", &hoy, "Alimentación", Some("Compra mercado".into()), "dev-1").unwrap();
        assert_eq!(det.saldo_actual, "1300.00");
        let id_gasto_erroneo = det.movimientos[0].id.clone();

        // Paso 4: Ajuste del paso 3 (monto correcto: 50,00) -> +450,00 -> Saldo 1.750,00
        let det = ejecutar_corregir_movimiento(&mut repo, cta.id(), &id_gasto_erroneo, "50.00", "dev-1").unwrap();
        assert_eq!(det.saldo_actual, "1750.00");
        assert_eq!(det.composicion.inicial, "1000.00");
        assert_eq!(det.composicion.ingresos, "800.00");
        assert_eq!(det.composicion.gastos, "500.00");
        assert_eq!(det.composicion.ajustes, "450.00");

        // INV-04: El movimiento original sigue en el historial marcado como corregido y existe el ajuste que lo referencia
        assert_eq!(det.movimientos.len(), 3);
        let mov_ajuste = &det.movimientos[0];
        let mov_original = det.movimientos.iter().find(|m| m.id == id_gasto_erroneo).unwrap();
        assert!(mov_original.corregido);
        assert!(mov_ajuste.es_ajuste);
        assert_eq!(mov_ajuste.referencia_id.as_deref(), Some(id_gasto_erroneo.as_str()));
        assert_eq!(mov_ajuste.monto, "450.00");
    }

    #[test]
    fn test_validaciones_registrar_movimiento() {
        let mut repo = RepositorioEnMemoria::default();
        let hoy = Utc::now().date_naive().format("%Y-%m-%d").to_string();
        let cta = ejecutar_crear_cuenta(&mut repo, "Banco USD", "banco", "USD", "100.00", "dev-1").unwrap();

        // 1. Monto cero o negativo -> falla campo "monto"
        let err_cero = ejecutar_registrar_movimiento(&mut repo, cta.id(), "gasto", "0", &hoy, "Alimentación", None, "dev-1").unwrap_err();
        assert_eq!(err_cero.campo, "monto");

        let err_neg = ejecutar_registrar_movimiento(&mut repo, cta.id(), "ingreso", "-10.00", &hoy, "Salario", None, "dev-1").unwrap_err();
        assert_eq!(err_neg.campo, "monto");

        // 2. Más decimales que la moneda (10.555 USD) -> falla campo "monto"
        let err_dec = ejecutar_registrar_movimiento(&mut repo, cta.id(), "gasto", "10.555", &hoy, "Alimentación", None, "dev-1").unwrap_err();
        assert_eq!(err_dec.campo, "monto");

        // 3. Categoría vacía, inexistente o de otro tipo -> falla campo "categoria"
        let err_cat_vacia = ejecutar_registrar_movimiento(&mut repo, cta.id(), "gasto", "10.00", &hoy, "  ", None, "dev-1").unwrap_err();
        assert_eq!(err_cat_vacia.campo, "categoria");

        let err_cat_inex = ejecutar_registrar_movimiento(&mut repo, cta.id(), "gasto", "10.00", &hoy, "Inexistente", None, "dev-1").unwrap_err();
        assert_eq!(err_cat_inex.campo, "categoria");

        let err_cat_cruzada = ejecutar_registrar_movimiento(&mut repo, cta.id(), "gasto", "10.00", &hoy, "Salario", None, "dev-1").unwrap_err();
        assert_eq!(err_cat_cruzada.campo, "categoria");

        // 4. Fecha futura -> falla campo "fecha"
        let err_futura = ejecutar_registrar_movimiento(&mut repo, cta.id(), "gasto", "10.00", "2099-12-31", "Alimentación", None, "dev-1").unwrap_err();
        assert_eq!(err_futura.campo, "fecha");
    }

    #[test]
    fn test_validaciones_corregir_movimiento_y_compensacion_total() {
        let mut repo = RepositorioEnMemoria::default();
        let hoy = Utc::now().date_naive().format("%Y-%m-%d").to_string();
        let cta = ejecutar_crear_cuenta(&mut repo, "Efectivo VES", "efectivo", "VES", "500.00", "dev-1").unwrap();

        let det = ejecutar_registrar_movimiento(&mut repo, cta.id(), "gasto", "200.00", &hoy, "Transporte", None, "dev-1").unwrap();
        assert_eq!(det.saldo_actual, "300.00");
        let id_gasto = det.movimientos[0].id.clone();

        // Monto negativo al corregir es rechazado
        let err_neg = ejecutar_corregir_movimiento(&mut repo, cta.id(), &id_gasto, "-10.00", "dev-1").unwrap_err();
        assert_eq!(err_neg.campo, "monto");

        // Compensar por completo con monto correcto = 0
        let det_cero = ejecutar_corregir_movimiento(&mut repo, cta.id(), &id_gasto, "0.00", "dev-1").unwrap();
        assert_eq!(det_cero.saldo_actual, "500.00");
        let id_ajuste = det_cero.movimientos[0].id.clone();

        // Intentar corregir un ajuste directamente es rechazado
        let err_ajuste = ejecutar_corregir_movimiento(&mut repo, cta.id(), &id_ajuste, "50.00", "dev-1").unwrap_err();
        assert_eq!(err_ajuste.campo, "movimiento");

        // Corregir el movimiento original por segunda vez (de 0.00 vigente a 80.00)
        let det_segunda = ejecutar_corregir_movimiento(&mut repo, cta.id(), &id_gasto, "80.00", "dev-1").unwrap();
        assert_eq!(det_segunda.saldo_actual, "420.00");
    }
}