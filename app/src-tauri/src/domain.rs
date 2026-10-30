use chrono::{DateTime, Local, NaiveDate, Utc};
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
    cantidad: Decimal,
    moneda: String,
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

/// La moneda es privada y no hay una operación para cambiarla (INV-18).
/// ```compile_fail
/// use app_lib::domain::Cuenta;
/// fn cambiar_moneda(cuenta: &mut Cuenta) {
///     cuenta.moneda = "EUR".to_string();
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Cuenta {
    id: String,
    nombre: String,
    tipo: TipoCuenta,
    moneda: String,
    saldo: Monto,
}

impl Cuenta {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn nombre(&self) -> &str {
        &self.nombre
    }
    pub fn tipo(&self) -> &TipoCuenta {
        &self.tipo
    }
    pub fn moneda(&self) -> &str {
        &self.moneda
    }
    pub fn saldo(&self) -> &Monto {
        &self.saldo
    }
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
            _ => Err(ErrorDominio::nuevo(
                "tipo",
                "El tipo de movimiento debe ser ingreso o gasto.",
            )),
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
        "Alimentación",
        "Transporte",
        "Servicios",
        "Salud",
        "Educación",
        "Entretenimiento",
        "Otros gastos",
    ];
    let ingresos = ["Salario", "Ventas", "Regalos", "Otros ingresos"];

    let mut lista = Vec::new();
    for g in gastos {
        lista.push(Categoria {
            nombre: g.to_string(),
            tipo: TipoMovimiento::Gasto,
        });
    }
    for i in ingresos {
        lista.push(Categoria {
            nombre: i.to_string(),
            tipo: TipoMovimiento::Ingreso,
        });
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    fecha_creacion: String,
    ingresos: Decimal,
    gastos: Decimal,
    ajustes: Decimal,
    movimientos: Vec<MovimientoVista>,
    // Mapa movimiento_original_id -> (TipoMovimiento, monto_vigente_positivo, categoria, fecha)
    originales: HashMap<String, (TipoMovimiento, Decimal, String, String)>,
}

impl EstadoCuentaInterno {
    // INV-05: Saldo actual = saldo inicial + ingresos - gastos ± ajustes
    fn saldo_actual(&self) -> Result<Decimal, ErrorDominio> {
        sumar_exacto(
            sumar_exacto(sumar_exacto(self.inicial, self.ingresos)?, -self.gastos)?,
            self.ajustes,
        )
    }

    // Calcula el saldo sumando movimientos y ajustes hasta la fecha indicada
    fn saldo_en_fecha(&self, fecha_consulta: NaiveDate) -> Result<Decimal, ErrorDominio> {
        let mut saldo = self.inicial;

        // Filtra cada movimiento registrado cuya fecha no supere la fecha consultada
        for mov in &self.movimientos {
            let dia = match NaiveDate::parse_from_str(&mov.fecha, "%Y-%m-%d") {
                Ok(d) => d,
                Err(_) => return Err(error_libro()),
            };

            if dia <= fecha_consulta {
                let monto = match Decimal::from_str(&mov.monto) {
                    Ok(n) => n,
                    Err(_) => return Err(error_libro()),
                };
                saldo = sumar_exacto(saldo, monto)?;
            }
        }

        Ok(saldo)
    }
}

struct ProyeccionLibro {
    cuentas_orden: Vec<String>,
    cuentas: HashMap<String, EstadoCuentaInterno>,
    categorias: Vec<Categoria>,
    // Almacena el catálogo dinámico de monedas y la moneda base activa
    monedas: Vec<Moneda>,
    tasas: Vec<TasaCambio>,
    moneda_base: String,
}

fn error_libro() -> ErrorDominio {
    ErrorDominio::nuevo(
        "almacenamiento",
        "El libro contiene un evento incompatible o inconsistente. Conserva el archivo SQLite para revisar sus datos; no se ha modificado.",
    )
}

fn sumar_exacto(a: Decimal, b: Decimal) -> Result<Decimal, ErrorDominio> {
    a.checked_add(b)
        .ok_or_else(|| ErrorDominio::nuevo("monto", "El monto excede el rango decimal permitido."))
}

#[derive(Deserialize)]
struct MovimientoHistorico {
    movimiento_id: String,
    cuenta_id: String,
    tipo_movimiento: String,
    monto: Decimal,
    categoria: String,
    nota: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum MovimientoAlmacenado {
    Actual(DatosMovimientoRegistrado),
    Historico(MovimientoHistorico),
}

#[derive(Deserialize)]
struct CorreccionHistorica {
    ajuste_id: String,
    movimiento_original_id: String,
    cuenta_id: String,
    monto_ajuste: Decimal,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum CorreccionAlmacenada {
    Actual(DatosMovimientoCorregido),
    Historica(CorreccionHistorica),
}

fn proyectar_libro(eventos: &[EventoLibro]) -> Result<ProyeccionLibro, ErrorDominio> {
    let mut eventos_aplicados = HashMap::new();
    let mut cuentas_orden = Vec::new();
    let mut cuentas: HashMap<String, EstadoCuentaInterno> = HashMap::new();
    let mut categorias = categorias_de_fabrica();
    // Inicializa el catálogo con las monedas de fábrica y USD como base
    let mut monedas = catalogo_monedas();
    let mut tasas = Vec::new();
    let mut moneda_base = "USD".to_string();
    let mut ids_movimientos = HashSet::new();

    for ev in eventos {
        if let Some(anterior) = eventos_aplicados.insert(ev.id.clone(), ev) {
            if anterior.tipo != ev.tipo
                || anterior.datos_json != ev.datos_json
                || anterior.momento_registro != ev.momento_registro
                || anterior.dispositivo_origen != ev.dispositivo_origen
            {
                return Err(error_libro());
            }
            continue;
        }
        let fecha_registro = ev
            .momento_registro
            .with_timezone(&Local)
            .date_naive()
            .to_string();
        match ev.tipo.as_str() {
            "MonedaCreada" => {
                let m: Moneda = match serde_json::from_str(&ev.datos_json) {
                    Ok(dato) => dato,
                    Err(_) => return Err(error_libro()),
                };

                // Valida e incorpora una moneda propia al catálogo en memoria
                match validar_moneda(&m.codigo, &m.nombre, &m.simbolo, m.decimales, &monedas) {
                    Ok(nueva) => {
                        if nueva == m {
                            monedas.push(nueva);
                        } else {
                            return Err(error_libro());
                        }
                    }
                    Err(_) => return Err(error_libro()),
                }
            }

            "TasaRegistrada" => {
                let t: TasaCambio = match serde_json::from_str(&ev.datos_json) {
                    Ok(dato) => dato,
                    Err(_) => return Err(error_libro()),
                };

                if t.id != ev.id {
                    return Err(error_libro());
                }

                // Revisamos la tasa antes de añadirla
                match validar_tasa(
                    &t.id,
                    &t.origen,
                    &t.destino,
                    &t.valor.to_string(),
                    &t.fecha,
                    &t.fuente,
                    &monedas,
                    ev.momento_registro.with_timezone(&Local).date_naive(),
                ) {
                    Ok(nueva) => {
                        if nueva == t {
                            tasas.push(nueva);
                        } else {
                            return Err(error_libro());
                        }
                    }
                    Err(_) => return Err(error_libro()),
                }
            }

            "TasaEliminada" => {
                let dato: DatosTasaEliminada = match serde_json::from_str(&ev.datos_json) {
                    Ok(d) => d,
                    Err(_) => return Err(error_libro()),
                };

                // Buscamos la tasa que se eliminó
                let pos = tasas.iter().position(|t| t.id == dato.tasa_id);

                match pos {
                    Some(i) => {
                        tasas.remove(i);
                    }
                    None => return Err(error_libro()),
                }
            }

            "MonedaBaseCambiada" => {
                let dato: DatosMonedaBaseCambiada = match serde_json::from_str(&ev.datos_json) {
                    Ok(d) => d,
                    Err(_) => return Err(error_libro()),
                };

                // Actualiza la moneda base si su código existe en el catálogo
                if buscar_moneda_en(&dato.codigo, &monedas).is_err() {
                    return Err(error_libro());
                }

                moneda_base = dato.codigo;
            }

            "CuentaCreada" => {
                let d: DatosCuentaCreada =
                    serde_json::from_str(&ev.datos_json).map_err(|_| error_libro())?;
                // Valida la moneda y el saldo inicial usando el catálogo dinámico
                let moneda = buscar_moneda_en(d.saldo_inicial.moneda(), &monedas)
                    .map_err(|_| error_libro())?;

                let inicial = Monto::desde_str_con_moneda(
                    &d.saldo_inicial.cantidad().to_string(),
                    &moneda,
                    "saldo_inicial",
                    true,
                )
                .map_err(|_| error_libro())?;

                if d.cuenta_id.is_empty()
                    || d.nombre.trim().is_empty()
                    || cuentas.contains_key(&d.cuenta_id)
                    || cuentas
                        .values()
                        .any(|c| c.nombre.to_lowercase() == d.nombre.trim().to_lowercase())
                {
                    return Err(error_libro());
                }
                cuentas_orden.push(d.cuenta_id.clone());
                cuentas.insert(
                    d.cuenta_id.clone(),
                    EstadoCuentaInterno {
                        id: d.cuenta_id,
                        nombre: d.nombre,
                        tipo: d.tipo,
                        moneda: inicial.moneda().to_string(),
                        inicial: inicial.cantidad(),
                        fecha_creacion: fecha_registro,
                        ingresos: Decimal::ZERO,
                        gastos: Decimal::ZERO,
                        ajustes: Decimal::ZERO,
                        movimientos: Vec::new(),
                        originales: HashMap::new(),
                    },
                );
            }
            "CategoriaCreada" => {
                let d: DatosCategoriaCreada =
                    serde_json::from_str(&ev.datos_json).map_err(|_| error_libro())?;
                if d.nombre.trim().is_empty() {
                    return Err(error_libro());
                }
                if !categorias
                    .iter()
                    .any(|c| c.tipo == d.tipo && c.nombre.to_lowercase() == d.nombre.to_lowercase())
                {
                    categorias.push(Categoria {
                        nombre: d.nombre,
                        tipo: d.tipo,
                    });
                }
            }
            "MovimientoRegistrado" => {
                let almacenado: MovimientoAlmacenado =
                    serde_json::from_str(&ev.datos_json).map_err(|_| error_libro())?;
                let d = match almacenado {
                    MovimientoAlmacenado::Actual(d) => d,
                    MovimientoAlmacenado::Historico(d) => {
                        let cta = cuentas.get(&d.cuenta_id).ok_or_else(error_libro)?;
                        // obtiene la moneda de la cuenta desde el catálogo dinámico
                        let moneda =
                            buscar_moneda_en(&cta.moneda, &monedas).map_err(|_| error_libro())?;
                        DatosMovimientoRegistrado {
                            movimiento_id: d.movimiento_id,
                            cuenta_id: d.cuenta_id,
                            tipo: TipoMovimiento::desde_str(&d.tipo_movimiento)
                                .map_err(|_| error_libro())?,
                            monto: Monto::desde_str_con_moneda(
                                &d.monto.to_string(),
                                &moneda,
                                "monto",
                                false,
                            )
                            .map_err(|_| error_libro())?,
                            fecha: fecha_registro,
                            categoria: d.categoria,
                            nota: d.nota,
                        }
                    }
                };

                let cta = cuentas.get_mut(&d.cuenta_id).ok_or_else(error_libro)?;
                // Valida el monto y formatea con los decimales propios de la moneda
                let moneda = buscar_moneda_en(&cta.moneda, &monedas).map_err(|_| error_libro())?;
                if d.movimiento_id.is_empty()
                    || !ids_movimientos.insert(d.movimiento_id.clone())
                    || d.monto.moneda() != cta.moneda
                    || d.categoria.trim().is_empty()
                    || NaiveDate::parse_from_str(&d.fecha, "%Y-%m-%d").is_err()
                {
                    return Err(error_libro());
                }
                let cant = Monto::desde_str_con_moneda(
                    &d.monto.cantidad().to_string(),
                    &moneda,
                    "monto",
                    false,
                )
                .map_err(|_| error_libro())?
                .cantidad();
                let (tipo, firmado) = match d.tipo {
                    TipoMovimiento::Ingreso => {
                        cta.ingresos = sumar_exacto(cta.ingresos, cant)?;
                        ("ingreso", cant)
                    }
                    TipoMovimiento::Gasto => {
                        cta.gastos = sumar_exacto(cta.gastos, cant)?;
                        ("gasto", -cant)
                    }
                };
                cta.originales.insert(
                    d.movimiento_id.clone(),
                    (d.tipo, cant, d.categoria.clone(), d.fecha.clone()),
                );
                let dec = moneda.decimales as usize;
                cta.movimientos.push(MovimientoVista {
                    id: d.movimiento_id,
                    fecha: d.fecha,
                    tipo: tipo.into(),
                    categoria: d.categoria,
                    nota: d.nota,
                    monto: format!("{:.*}", dec, firmado),
                    monto_vigente: format!("{:.*}", dec, cant),
                    corregido: false,
                    es_ajuste: false,
                    referencia_id: None,
                });
                cta.saldo_actual()?;
            }
            "MovimientoCorregido" => {
                let almacenada: CorreccionAlmacenada =
                    serde_json::from_str(&ev.datos_json).map_err(|_| error_libro())?;
                let d = match almacenada {
                    CorreccionAlmacenada::Actual(d) => d,
                    CorreccionAlmacenada::Historica(d) => {
                        let cta = cuentas.get(&d.cuenta_id).ok_or_else(error_libro)?;
                        // Busca la moneda en el catálogo dinámico para reconstruir el ajuste
                        let moneda =
                            buscar_moneda_en(&cta.moneda, &monedas).map_err(|_| error_libro())?;
                        let (tipo, vigente, _, _) = cta
                            .originales
                            .get(&d.movimiento_original_id)
                            .ok_or_else(error_libro)?;
                        let nuevo = sumar_exacto(
                            *vigente,
                            match tipo {
                                TipoMovimiento::Ingreso => d.monto_ajuste,
                                TipoMovimiento::Gasto => -d.monto_ajuste,
                            },
                        )?;
                        DatosMovimientoCorregido {
                            ajuste_id: d.ajuste_id,
                            movimiento_original_id: d.movimiento_original_id,
                            cuenta_id: d.cuenta_id,
                            monto_correcto: Monto::desde_str_con_moneda(
                                &nuevo.to_string(),
                                &moneda,
                                "monto",
                                true,
                            )
                            .map_err(|_| error_libro())?,
                            diferencia: d.monto_ajuste,
                            fecha: fecha_registro,
                            nota: "Ajuste histórico".into(),
                        }
                    }
                };

                let cta = cuentas.get_mut(&d.cuenta_id).ok_or_else(error_libro)?;
                // Revalida el monto corregido y formatea el ajuste según sus decimales
                let moneda = buscar_moneda_en(&cta.moneda, &monedas).map_err(|_| error_libro())?;
                let (tipo, vigente, categoria, _) = cta
                    .originales
                    .get_mut(&d.movimiento_original_id)
                    .ok_or_else(error_libro)?;
                let nuevo = Monto::desde_str_con_moneda(
                    &d.monto_correcto.cantidad().to_string(),
                    &moneda,
                    "monto",
                    true,
                )
                .map_err(|_| error_libro())?
                .cantidad();
                let esperado = match tipo {
                    TipoMovimiento::Ingreso => sumar_exacto(nuevo, -*vigente)?,
                    TipoMovimiento::Gasto => sumar_exacto(*vigente, -nuevo)?,
                };
                if d.monto_correcto.moneda() != cta.moneda
                    || d.diferencia != esperado
                    || d.ajuste_id.is_empty()
                    || !ids_movimientos.insert(d.ajuste_id.clone())
                    || NaiveDate::parse_from_str(&d.fecha, "%Y-%m-%d").is_err()
                {
                    return Err(error_libro());
                }
                let dec = moneda.decimales as usize;
                *vigente = nuevo;
                let categoria = categoria.clone();
                cta.ajustes = sumar_exacto(cta.ajustes, d.diferencia)?;
                for mov in &mut cta.movimientos {
                    if mov.id == d.movimiento_original_id {
                        mov.corregido = true;
                        mov.monto_vigente = format!("{:.*}", dec, nuevo);
                    }
                }
                cta.movimientos.push(MovimientoVista {
                    id: d.ajuste_id,
                    fecha: d.fecha,
                    tipo: "ajuste".into(),
                    categoria,
                    nota: Some(d.nota),
                    monto: format!("{:.*}", dec, d.diferencia),
                    monto_vigente: format!("{:.*}", dec, nuevo),
                    corregido: false,
                    es_ajuste: true,
                    referencia_id: Some(d.movimiento_original_id),
                });
                cta.saldo_actual()?;
            }
            _ => return Err(error_libro()),
        }
    }
    Ok(ProyeccionLibro {
        cuentas_orden,
        cuentas,
        categorias,
        monedas,
        tasas,
        moneda_base,
    })
}

fn validar_efecto_subtotal(
    libro: &ProyeccionLibro,
    moneda: &str,
    efecto: Decimal,
) -> Result<(), ErrorDominio> {
    let subtotal = libro
        .cuentas_orden
        .iter()
        .try_fold(Decimal::ZERO, |total, id| {
            let cuenta = &libro.cuentas[id];
            if cuenta.moneda == moneda {
                sumar_exacto(total, cuenta.saldo_actual()?)
            } else {
                Ok(total)
            }
        })?;
    sumar_exacto(subtotal, efecto)?;
    Ok(())
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
        return Err(ErrorDominio::nuevo(
            "nombre",
            "El nombre de la cuenta no puede estar vacío.",
        ));
    }

    let tipo = TipoCuenta::desde_str(tipo_str)?;

    let eventos = repo
        .obtener_eventos()
        .map_err(|_| ErrorDominio::nuevo("general", "No se pudo leer el almacenamiento local."))?;

    let proyeccion = proyectar_libro(&eventos)?;

    // Busca en el catálogo proyectado para permitir cuentas con monedas propias
    let moneda = buscar_moneda_en(moneda_str, &proyeccion.monedas)?;

    let monto_inicial =
        Monto::desde_str_con_moneda(saldo_inicial_str, &moneda, "saldo_inicial", true)?;

    if proyeccion
        .cuentas
        .values()
        .any(|c| c.nombre.to_lowercase() == nombre_limpio.to_lowercase())
    {
        return Err(ErrorDominio::nuevo(
            "nombre",
            "Ya existe una cuenta con ese nombre.",
        ));
    }

    validar_efecto_subtotal(
        &proyeccion,
        monto_inicial.moneda(),
        monto_inicial.cantidad(),
    )?;
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

    repo.guardar_evento(&evento).map_err(|_| {
        ErrorDominio::nuevo("general", "No se pudo guardar la cuenta en el dispositivo.")
    })?;

    Ok(Cuenta {
        id: cuenta_id,
        nombre: nombre_limpio.to_string(),
        tipo,
        moneda: monto_inicial.moneda().to_string(),
        saldo: monto_inicial,
    })
}

// Devuelve el catálogo completo de monedas de fábrica y propias
pub fn ejecutar_listar_monedas<R: RepositorioEventos>(
    repo: &R,
) -> Result<Vec<Moneda>, ErrorDominio> {
    // Convierte fallos de lectura del repositorio en un error de dominio
    let eventos = match repo.obtener_eventos() {
        Ok(e) => e,
        Err(_) => {
            return Err(ErrorDominio::nuevo(
                "general",
                "No se pudieron cargar las monedas.",
            ));
        }
    };

    // Reconstruye el estado del libro aplicando la secuencia histórica de eventos
    let libro = proyectar_libro(&eventos)?;

    Ok(libro.monedas)
}

// Devuelve el código de la moneda base activa del libro
pub fn ejecutar_obtener_moneda_base<R: RepositorioEventos>(
    repo: &R,
) -> Result<String, ErrorDominio> {
    let lista = repo
        .obtener_eventos()
        .map_err(|_| ErrorDominio::nuevo("general", "No se pudieron cargar los datos."))?;

    // Obtiene la moneda base tras aplicar los eventos registrados
    let resp = proyectar_libro(&lista)?;

    Ok(resp.moneda_base)
}

pub fn ejecutar_listar_tasas<R: RepositorioEventos>(
    repo: &R,
) -> Result<Vec<TasaCambio>, ErrorDominio> {
    let eventos = match repo.obtener_eventos() {
        Ok(e) => e,
        Err(_) => {
            return Err(ErrorDominio::nuevo(
                "general",
                "No se pudieron cargar las tasas.",
            ));
        }
    };

    let libro = proyectar_libro(&eventos)?;

    let mut tasas = libro.tasas;
    tasas.reverse();
    tasas.sort_by(|a, b| b.fecha.cmp(&a.fecha));
    Ok(tasas)
}

pub fn ejecutar_listar_categorias<R: RepositorioEventos>(
    repo: &R,
) -> Result<Vec<Categoria>, ErrorDominio> {
    let eventos = repo
        .obtener_eventos()
        .map_err(|_| ErrorDominio::nuevo("general", "No se pudieron cargar las categorías."))?;
    Ok(proyectar_libro(&eventos)?.categorias)
}

pub fn ejecutar_crear_categoria<R: RepositorioEventos>(
    repo: &mut R,
    nombre: &str,
    tipo_str: &str,
    dispositivo_id: &str,
) -> Result<Categoria, ErrorDominio> {
    let nombre_limpio = nombre.trim();
    if nombre_limpio.is_empty() {
        return Err(ErrorDominio::nuevo(
            "categoria",
            "El nombre de la categoría no puede estar vacío.",
        ));
    }
    let tipo = TipoMovimiento::desde_str(tipo_str)?;

    let eventos = repo
        .obtener_eventos()
        .map_err(|_| ErrorDominio::nuevo("general", "No se pudo leer el almacenamiento local."))?;
    let proyeccion = proyectar_libro(&eventos)?;

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

pub struct RegistrarMovimiento<'a> {
    pub cuenta_id: &'a str,
    pub tipo_str: &'a str,
    pub monto_str: &'a str,
    pub fecha_str: &'a str,
    pub categoria_str: &'a str,
    pub nota_opt: Option<String>,
    pub dispositivo_id: &'a str,
}

pub fn ejecutar_registrar_movimiento<R: RepositorioEventos>(
    repo: &mut R,
    entrada: RegistrarMovimiento<'_>,
) -> Result<DetalleCuenta, ErrorDominio> {
    let RegistrarMovimiento {
        cuenta_id,
        tipo_str,
        monto_str,
        fecha_str,
        categoria_str,
        nota_opt,
        dispositivo_id,
    } = entrada;
    let eventos = repo
        .obtener_eventos()
        .map_err(|_| ErrorDominio::nuevo("general", "No se pudo leer el libro local."))?;
    let proyeccion = proyectar_libro(&eventos)?;

    let cta = proyeccion
        .cuentas
        .get(cuenta_id)
        .ok_or_else(|| ErrorDominio::nuevo("cuenta", "La cuenta seleccionada no existe."))?;

    // Valida el tipo
    let tipo = TipoMovimiento::desde_str(tipo_str)?;

    // Valida los decimales de la moneda de la cuenta
    let moneda = buscar_moneda_en(&cta.moneda, &proyeccion.monedas)?;
    let monto = Monto::desde_str_con_moneda(monto_str, &moneda, "monto", false)?;

    // Valida la fecha (formato YYYY-MM-DD y no futura)
    let fecha_limpia = validar_fecha_movimiento(fecha_str, Local::now().date_naive())?.to_string();

    // Valida la categoría (no vacía, existente y del mismo tipo)
    let cat_limpia = categoria_str.trim();
    if cat_limpia.is_empty() {
        return Err(ErrorDominio::nuevo(
            "categoria",
            "La categoría es obligatoria.",
        ));
    }
    // Buscamos la categoria que tenga el mismo nombre y tipo.
    let cat_encontrada = proyeccion
        .categorias
        .iter()
        .find(|c| c.nombre.to_lowercase() == cat_limpia.to_lowercase() && c.tipo == tipo);

    let cat_nombre_oficial = match cat_encontrada {
        Some(c) => c.nombre.clone(),
        None => {
            if proyeccion
                .categorias
                .iter()
                .any(|c| c.nombre.to_lowercase() == cat_limpia.to_lowercase())
            {
                return Err(ErrorDominio::nuevo(
                    "categoria",
                    "La categoría seleccionada no es valida para este tipo de movimiento.",
                ));
            }

            return Err(ErrorDominio::nuevo(
                "categoria",
                "La categoría seleccionada no existe.",
            ));
        }
    };
    let efecto = match tipo {
        TipoMovimiento::Ingreso => monto.cantidad(),
        TipoMovimiento::Gasto => -monto.cantidad(),
    };
    sumar_exacto(cta.saldo_actual()?, efecto)?;
    validar_efecto_subtotal(&proyeccion, &cta.moneda, efecto)?;
    match tipo {
        TipoMovimiento::Ingreso => sumar_exacto(cta.ingresos, monto.cantidad())?,
        TipoMovimiento::Gasto => sumar_exacto(cta.gastos, monto.cantidad())?,
    };
    let nota_limpia = nota_opt
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty());

    let datos = DatosMovimientoRegistrado {
        movimiento_id: uuid::Uuid::new_v4().to_string(),
        cuenta_id: cuenta_id.to_string(),
        tipo,
        monto,
        fecha: fecha_limpia,
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
    let proyeccion = proyectar_libro(&eventos)?;

    let cta = proyeccion
        .cuentas
        .get(cuenta_id)
        .ok_or_else(|| ErrorDominio::nuevo("cuenta", "La cuenta indicada no existe."))?;

    // Verificar que sea un movimiento original y no un ajuste
    let (tipo_orig, monto_vigente, _cat_orig, _fecha_orig) =
        cta.originales.get(movimiento_id).ok_or_else(|| {
            ErrorDominio::nuevo(
                "movimiento",
                "Solo se pueden corregir movimientos originales, no ajustes.",
            )
        })?;

    // El monto correcto puede ser 0 (para anular un movimiento que no debió existir) o positivo
    let moneda = buscar_moneda_en(&cta.moneda, &proyeccion.monedas)?;
    let monto_correcto = Monto::desde_str_con_moneda(monto_correcto_str, &moneda, "monto", true)?;

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
        TipoMovimiento::Ingreso => sumar_exacto(nuevo_valor, -*monto_vigente)?,
        TipoMovimiento::Gasto => sumar_exacto(*monto_vigente, -nuevo_valor)?,
    };

    sumar_exacto(cta.ajustes, diferencia)?;
    sumar_exacto(cta.saldo_actual()?, diferencia)?;
    validar_efecto_subtotal(&proyeccion, &cta.moneda, diferencia)?;
    let hoy_str = Local::now().date_naive().format("%Y-%m-%d").to_string();

    let dec = moneda.decimales as usize;

    let nota = format!(
        "Ajuste de corrección (monto anterior: {:.*}, monto correcto: {:.*})",
        dec, monto_vigente, dec, nuevo_valor
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
    let mut proyeccion = proyectar_libro(&eventos)?;

    let cta = proyeccion
        .cuentas
        .remove(cuenta_id)
        .ok_or_else(|| ErrorDominio::nuevo("cuenta", "La cuenta no existe."))?;

    let saldo_actual = cta.saldo_actual()?;

    //Formatea el detalle de la cuenta según los decimales de su moneda
    let moneda = buscar_moneda_en(&cta.moneda, &proyeccion.monedas)?;
    let dec = moneda.decimales as usize;
    let mut movimientos = cta.movimientos;
    // Del más reciente al más antiguo
    movimientos.reverse();
    movimientos.sort_by(|a, b| b.fecha.cmp(&a.fecha));

    Ok(DetalleCuenta {
        saldo_actual: format!("{:.*}", dec, saldo_actual),
        composicion: ComposicionSaldo {
            inicial: format!("{:.*}", dec, cta.inicial),
            ingresos: format!("{:.*}", dec, cta.ingresos),
            gastos: format!("{:.*}", dec, cta.gastos),
            ajustes: format!("{:.*}", dec, cta.ajustes),
        },
        movimientos,
    })
}

// Representa el subtotal acumulado en una moneda específica
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

pub fn ejecutar_listar_cuentas<R: RepositorioEventos>(
    repo: &R,
) -> Result<ResumenCuentas, ErrorDominio> {
    let eventos = repo.obtener_eventos().map_err(|_| {
        ErrorDominio::nuevo("general", "No se pudieron cargar las cuentas locales.")
    })?;
    let proyeccion = proyectar_libro(&eventos)?;

    let mut cuentas = Vec::new();
    let mut mapa_subtotales: BTreeMap<String, Decimal> = BTreeMap::new();

    for id in &proyeccion.cuentas_orden {
        if let Some(c) = proyeccion.cuentas.get(id) {
            let saldo_actual = c.saldo_actual()?;
            // Redondea y formatea saldos y subtotales según los decimales de cada moneda
            let moneda = buscar_moneda_en(&c.moneda, &proyeccion.monedas)?;
            let subtotal = mapa_subtotales
                .entry(c.moneda.clone())
                .or_insert(Decimal::ZERO);
            *subtotal = sumar_exacto(*subtotal, saldo_actual)?;

            cuentas.push(Cuenta {
                id: c.id.clone(),
                nombre: c.nombre.clone(),
                tipo: c.tipo.clone(),
                moneda: c.moneda.clone(),
                saldo: Monto {
                    cantidad: saldo_actual.round_dp(moneda.decimales),
                    moneda: c.moneda.clone(),
                },
            });
        }
    }

    let mut subtotales = Vec::new();
    for (codigo, total_dec) in mapa_subtotales {
        let info_moneda = buscar_moneda_en(&codigo, &proyeccion.monedas)?;
        subtotales.push(SubtotalMoneda {
            codigo,
            simbolo: info_moneda.simbolo,
            subtotal: format!(
                "{:.*}",
                info_moneda.decimales as usize,
                total_dec.round_dp(info_moneda.decimales)
            ),
        });
    }

    Ok(ResumenCuentas {
        cuentas,
        subtotales,
    })
}

// Calcula saldos por cuenta y subtotales por moneda hasta una fecha dada
fn calcular_saldos_en_fecha(
    libro: &ProyeccionLibro,
    fecha_consulta: NaiveDate,
) -> Result<ResumenCuentas, ErrorDominio> {
    let mut cuentas = Vec::new();
    let mut mapa_subtotales: BTreeMap<String, Decimal> = BTreeMap::new();

    // Recorre las cuentas respectando el orden en que fueron creadas
    for id in &libro.cuentas_orden {
        if let Some(c) = libro.cuentas.get(id) {
            if c.fecha_creacion > fecha_consulta.to_string() {
                continue;
            }
            let saldo_fecha = c.saldo_en_fecha(fecha_consulta)?;
            let moneda = buscar_moneda_en(&c.moneda, &libro.monedas)?;

            // Acumula el subtotal exacto correspondiente a la moneda de la cuenta
            let subtotal = mapa_subtotales
                .entry(c.moneda.clone())
                .or_insert(Decimal::ZERO);
            *subtotal = sumar_exacto(*subtotal, saldo_fecha)?;

            cuentas.push(Cuenta {
                id: c.id.clone(),
                nombre: c.nombre.clone(),
                tipo: c.tipo.clone(),
                moneda: c.moneda.clone(),
                saldo: Monto {
                    cantidad: saldo_fecha.round_dp(moneda.decimales),
                    moneda: c.moneda.clone(),
                },
            });
        }
    }

    let mut subtotales = Vec::new();

    // Formatea cada subtotal respetando la escala decimal de su moneda
    for (codigo, total_dec) in mapa_subtotales {
        let info_moneda = buscar_moneda_en(&codigo, &libro.monedas)?;
        subtotales.push(SubtotalMoneda {
            codigo,
            simbolo: info_moneda.simbolo,
            subtotal: format!(
                "{:.*}",
                info_moneda.decimales as usize,
                total_dec.round_dp(info_moneda.decimales)
            ),
        });
    }

    Ok(ResumenCuentas {
        cuentas,
        subtotales,
    })
}

// Obtiene las cuentas y subtotales proyectados hasta una fecha de consulta
pub fn ejecutar_listar_cuentas_en_fecha<R: RepositorioEventos>(
    repo: &R,
    fecha_consulta: NaiveDate,
) -> Result<ResumenCuentas, ErrorDominio> {
    let fecha_consulta =
        validar_fecha_movimiento(&fecha_consulta.to_string(), Local::now().date_naive())?;
    // Lee los eventos del repositorio local para reconstruir el libro
    let eventos = match repo.obtener_eventos() {
        Ok(e) => e,
        Err(_) => {
            return Err(ErrorDominio::nuevo(
                "general",
                "No se pudieron cargar las cuentas locales.",
            ));
        }
    };

    let libro = proyectar_libro(&eventos)?;
    calcular_saldos_en_fecha(&libro, fecha_consulta)
}

/// Tasa original seleccionada y sentido de aplicación. Las tasas se reciben
/// en orden de inserción del libro, con las anuladas ya excluidas.
#[derive(Debug, Clone, PartialEq)]
pub struct TasaAplicable {
    pub tasa: Option<TasaCambio>,
    pub inversa: bool,
}

pub fn formatear_aviso_tasa_faltante(origen: &str, base: &str, fecha: NaiveDate) -> String {
    format!(
        "Falta una tasa de {} a {} con fecha igual o anterior al {}",
        origen.trim().to_uppercase(),
        base.trim().to_uppercase(),
        fecha.format("%d/%m/%Y")
    )
}

pub fn buscar_tasa_aplicable(
    origen: &str,
    moneda_base: &str,
    fecha: NaiveDate,
    tasas: &[TasaCambio],
) -> Result<TasaAplicable, ErrorDominio> {
    let origen = origen.trim().to_uppercase();
    let base = moneda_base.trim().to_uppercase();
    if origen == base {
        return Ok(TasaAplicable {
            tasa: None,
            inversa: false,
        });
    }
    // Prioridad a la directa vigente; la inversa solo se usa si no hay directa.
    for inversa in [false, true] {
        let (desde, hasta) = if inversa {
            (&base, &origen)
        } else {
            (&origen, &base)
        };
        let mut mejor: Option<(NaiveDate, &TasaCambio)> = None;
        for tasa in tasas
            .iter()
            .filter(|t| &t.origen == desde && &t.destino == hasta)
        {
            let dia =
                NaiveDate::parse_from_str(&tasa.fecha, "%Y-%m-%d").map_err(|_| error_libro())?;
            if tasa.valor <= Decimal::ZERO || tasa.valor.scale() > 8 {
                return Err(error_libro());
            }
            if dia <= fecha && mejor.as_ref().is_none_or(|(anterior, _)| dia >= *anterior) {
                mejor = Some((dia, tasa));
            }
        }
        if let Some((_, tasa)) = mejor {
            return Ok(TasaAplicable {
                tasa: Some(tasa.clone()),
                inversa,
            });
        }
    }
    Err(ErrorDominio::nuevo(
        "tasa",
        &formatear_aviso_tasa_faltante(&origen, &base, fecha),
    ))
}

/// Convierte sin redondeo de presentación ni multiplicar por una inversa
/// previamente aproximada: en sentido inverso divide directamente por la tasa.
pub fn convertir_monto_a_base(
    cantidad: Decimal,
    origen: &str,
    base: &str,
    fecha: NaiveDate,
    tasas: &[TasaCambio],
) -> Result<(Decimal, Option<TasaCambio>, bool), ErrorDominio> {
    let aplicable = buscar_tasa_aplicable(origen, base, fecha, tasas)?;
    let convertido = match &aplicable.tasa {
        None => Some(cantidad),
        Some(t) if aplicable.inversa => cantidad.checked_div(t.valor),
        Some(t) => cantidad.checked_mul(t.valor),
    }
    .ok_or_else(|| {
        ErrorDominio::nuevo("valor", "La conversión excede el rango decimal permitido.")
    })?;
    Ok((convertido, aplicable.tasa, aplicable.inversa))
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
        if let Some(existente) = self.eventos.iter().find(|e| e.id == evento.id) {
            if existente != evento {
                return Err("El identificador ya pertenece a otro evento.".into());
            }
        } else {
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
    use crate::pruebas_tasas_monedas::{aviso_faltante, consultar_fixture_en_fecha};

    #[test]
    fn test_ejemplo_seccion_2_2_correccion_e_inv_04_inv_05() {
        let mut repo = RepositorioEnMemoria::default();
        let hoy = chrono::Local::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();

        // Paso 1: Cuenta "Billetera dólares" en USD con saldo inicial 1.000,00
        let cta = ejecutar_crear_cuenta(
            &mut repo,
            "Billetera dólares",
            "billetera digital",
            "USD",
            "1000.00",
            "dev-1",
        )
        .unwrap();
        assert_eq!(cta.saldo().cantidad().to_string(), "1000.00");

        // Paso 2: Ingreso Salario +800,00 -> Saldo 1.800,00
        let det = ejecutar_registrar_movimiento(
            &mut repo,
            RegistrarMovimiento {
                cuenta_id: cta.id(),
                tipo_str: "ingreso",
                monto_str: "800.00",
                fecha_str: &hoy,
                categoria_str: "Salario",
                nota_opt: None,
                dispositivo_id: "dev-1",
            },
        )
        .unwrap();
        assert_eq!(det.saldo_actual, "1800.00");

        // Paso 3: Gasto Alimentación (era 50, se anotó 500) -> -500,00 -> Saldo 1.300,00
        let det = ejecutar_registrar_movimiento(
            &mut repo,
            RegistrarMovimiento {
                cuenta_id: cta.id(),
                tipo_str: "gasto",
                monto_str: "500.00",
                fecha_str: &hoy,
                categoria_str: "Alimentación",
                nota_opt: Some("Compra mercado".into()),
                dispositivo_id: "dev-1",
            },
        )
        .unwrap();
        assert_eq!(det.saldo_actual, "1300.00");
        let id_gasto_erroneo = det.movimientos[0].id.clone();

        // Paso 4: Ajuste del paso 3 (monto correcto: 50,00) -> +450,00 -> Saldo 1.750,00
        let det =
            ejecutar_corregir_movimiento(&mut repo, cta.id(), &id_gasto_erroneo, "50.00", "dev-1")
                .unwrap();
        assert_eq!(det.saldo_actual, "1750.00");
        assert_eq!(det.composicion.inicial, "1000.00");
        assert_eq!(det.composicion.ingresos, "800.00");
        assert_eq!(det.composicion.gastos, "500.00");
        assert_eq!(det.composicion.ajustes, "450.00");

        // INV-04: El movimiento original sigue en el historial marcado como corregido y existe el ajuste que lo referencia
        assert_eq!(det.movimientos.len(), 3);
        let mov_ajuste = &det.movimientos[0];
        let mov_original = det
            .movimientos
            .iter()
            .find(|m| m.id == id_gasto_erroneo)
            .unwrap();
        assert!(mov_original.corregido);
        assert!(mov_ajuste.es_ajuste);
        assert_eq!(
            mov_ajuste.referencia_id.as_deref(),
            Some(id_gasto_erroneo.as_str())
        );
        assert_eq!(mov_ajuste.monto, "450.00");
    }

    #[test]
    fn test_validaciones_registrar_movimiento() {
        let mut repo = RepositorioEnMemoria::default();
        let hoy = chrono::Local::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();
        let cta = ejecutar_crear_cuenta(&mut repo, "Banco USD", "banco", "USD", "100.00", "dev-1")
            .unwrap();

        // 1. Monto cero o negativo -> falla campo "monto"
        let err_cero = ejecutar_registrar_movimiento(
            &mut repo,
            RegistrarMovimiento {
                cuenta_id: cta.id(),
                tipo_str: "gasto",
                monto_str: "0",
                fecha_str: &hoy,
                categoria_str: "Alimentación",
                nota_opt: None,
                dispositivo_id: "dev-1",
            },
        )
        .unwrap_err();
        assert_eq!(err_cero.campo, "monto");

        let err_neg = ejecutar_registrar_movimiento(
            &mut repo,
            RegistrarMovimiento {
                cuenta_id: cta.id(),
                tipo_str: "ingreso",
                monto_str: "-10.00",
                fecha_str: &hoy,
                categoria_str: "Salario",
                nota_opt: None,
                dispositivo_id: "dev-1",
            },
        )
        .unwrap_err();
        assert_eq!(err_neg.campo, "monto");

        // 2. Más decimales que la moneda (10.555 USD) -> falla campo "monto"
        let err_dec = ejecutar_registrar_movimiento(
            &mut repo,
            RegistrarMovimiento {
                cuenta_id: cta.id(),
                tipo_str: "gasto",
                monto_str: "10.555",
                fecha_str: &hoy,
                categoria_str: "Alimentación",
                nota_opt: None,
                dispositivo_id: "dev-1",
            },
        )
        .unwrap_err();
        assert_eq!(err_dec.campo, "monto");

        // 3. Categoría vacía, inexistente o de otro tipo -> falla campo "categoria"
        let err_cat_vacia = ejecutar_registrar_movimiento(
            &mut repo,
            RegistrarMovimiento {
                cuenta_id: cta.id(),
                tipo_str: "gasto",
                monto_str: "10.00",
                fecha_str: &hoy,
                categoria_str: "  ",
                nota_opt: None,
                dispositivo_id: "dev-1",
            },
        )
        .unwrap_err();
        assert_eq!(err_cat_vacia.campo, "categoria");

        let err_cat_inex = ejecutar_registrar_movimiento(
            &mut repo,
            RegistrarMovimiento {
                cuenta_id: cta.id(),
                tipo_str: "gasto",
                monto_str: "10.00",
                fecha_str: &hoy,
                categoria_str: "Inexistente",
                nota_opt: None,
                dispositivo_id: "dev-1",
            },
        )
        .unwrap_err();
        assert_eq!(err_cat_inex.campo, "categoria");

        let err_cat_cruzada = ejecutar_registrar_movimiento(
            &mut repo,
            RegistrarMovimiento {
                cuenta_id: cta.id(),
                tipo_str: "gasto",
                monto_str: "10.00",
                fecha_str: &hoy,
                categoria_str: "Salario",
                nota_opt: None,
                dispositivo_id: "dev-1",
            },
        )
        .unwrap_err();
        assert_eq!(err_cat_cruzada.campo, "categoria");

        // 4. Fecha futura -> falla campo "fecha"
        let err_futura = ejecutar_registrar_movimiento(
            &mut repo,
            RegistrarMovimiento {
                cuenta_id: cta.id(),
                tipo_str: "gasto",
                monto_str: "10.00",
                fecha_str: "2099-12-31",
                categoria_str: "Alimentación",
                nota_opt: None,
                dispositivo_id: "dev-1",
            },
        )
        .unwrap_err();
        assert_eq!(err_futura.campo, "fecha");
    }

    #[test]
    fn test_validaciones_corregir_movimiento_y_compensacion_total() {
        let mut repo = RepositorioEnMemoria::default();
        let hoy = chrono::Local::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();
        let cta = ejecutar_crear_cuenta(
            &mut repo,
            "Efectivo VES",
            "efectivo",
            "VES",
            "500.00",
            "dev-1",
        )
        .unwrap();

        let det = ejecutar_registrar_movimiento(
            &mut repo,
            RegistrarMovimiento {
                cuenta_id: cta.id(),
                tipo_str: "gasto",
                monto_str: "200.00",
                fecha_str: &hoy,
                categoria_str: "Transporte",
                nota_opt: None,
                dispositivo_id: "dev-1",
            },
        )
        .unwrap();
        assert_eq!(det.saldo_actual, "300.00");
        let id_gasto = det.movimientos[0].id.clone();

        // Monto negativo al corregir es rechazado
        let err_neg =
            ejecutar_corregir_movimiento(&mut repo, cta.id(), &id_gasto, "-10.00", "dev-1")
                .unwrap_err();
        assert_eq!(err_neg.campo, "monto");

        // Compensar por completo con monto correcto = 0
        let det_cero =
            ejecutar_corregir_movimiento(&mut repo, cta.id(), &id_gasto, "0.00", "dev-1").unwrap();
        assert_eq!(det_cero.saldo_actual, "500.00");
        let id_ajuste = det_cero.movimientos[0].id.clone();

        // Intentar corregir un ajuste directamente es rechazado
        let err_ajuste =
            ejecutar_corregir_movimiento(&mut repo, cta.id(), &id_ajuste, "50.00", "dev-1")
                .unwrap_err();
        assert_eq!(err_ajuste.campo, "movimiento");

        // Corregir el movimiento original por segunda vez (de 0.00 vigente a 80.00)
        let det_segunda =
            ejecutar_corregir_movimiento(&mut repo, cta.id(), &id_gasto, "80.00", "dev-1").unwrap();
        assert_eq!(det_segunda.saldo_actual, "420.00");
    }

    // Verifica la selección de tasa vigente, desempate diario, inversa y par faltante
    #[test]
    fn test_seleccion_tasa_vigente_inversa_y_par_faltante() {
        let tasas = vec![
            TasaCambio {
                id: "t1".into(),
                origen: "USD".into(),
                destino: "VES".into(),
                valor: Decimal::from_str("880.00").unwrap(),
                fecha: "2026-10-19".into(),
                fuente: "BCV".into(),
            },
            TasaCambio {
                id: "t2".into(),
                origen: "USD".into(),
                destino: "VES".into(),
                valor: Decimal::from_str("885.00").unwrap(),
                fecha: "2026-10-20".into(),
                fuente: "BCV mañana".into(),
            },
            TasaCambio {
                id: "t3".into(),
                origen: "USD".into(),
                destino: "VES".into(),
                valor: Decimal::from_str("887.00").unwrap(),
                fecha: "2026-10-20".into(),
                fuente: "BCV tarde".into(),
            },
            TasaCambio {
                id: "t4".into(),
                origen: "USDT".into(),
                destino: "VES".into(),
                valor: Decimal::from_str("1000.00").unwrap(),
                fecha: "2026-10-21".into(),
                fuente: "casa de cambio".into(),
            },
        ];

        let fecha_consulta = NaiveDate::from_ymd_opt(2026, 10, 23).unwrap();

        // Selecciona la última tasa directa registrada del día más reciente
        let directa = buscar_tasa_aplicable("USD", "VES", fecha_consulta, &tasas).unwrap();
        assert!(!directa.inversa);
        assert_eq!(
            directa.tasa.as_ref().unwrap().valor,
            Decimal::from_str("887.00").unwrap()
        );
        assert_eq!(directa.tasa.as_ref().unwrap().fuente, "BCV tarde");

        // Aplica la inversa exacta de VES a USD
        let (en_usd, vista_inv, inversa) = convertir_monto_a_base(
            Decimal::from_str("88700.00").unwrap(),
            "VES",
            "USD",
            fecha_consulta,
            &tasas,
        )
        .unwrap();
        assert!(inversa);
        assert_eq!(vista_inv.unwrap().id, "t3");
        assert_eq!(en_usd.round_dp(2), Decimal::from_str("100.00").unwrap());

        // Rechaza tasas cruzadas (USDT -> VES -> USD) e informa el par faltante
        let err_faltante =
            buscar_tasa_aplicable("USDT", "USD", fecha_consulta, &tasas).unwrap_err();
        assert_eq!(err_faltante.campo, "tasa");
        assert_eq!(
            err_faltante.mensaje,
            "Falta una tasa de USDT a USD con fecha igual o anterior al 23/10/2026"
        );
    }

    // Verifica el ejemplo de la sección 2.3, aviso de tasa faltante y redondeo único
    #[test]
    fn test_consultar_patrimonio_seccion_2_3_y_redondeo_final() {
        let mut repo = RepositorioEnMemoria::default();
        let fecha = NaiveDate::from_ymd_opt(2026, 10, 23).unwrap();
        // Crea las tres cuentas del ejemplo de la especificación
        ejecutar_crear_cuenta(
            &mut repo,
            "Efectivo bolívares",
            "efectivo",
            "VES",
            "88700.00",
            "d1",
        )
        .unwrap();
        ejecutar_crear_cuenta(
            &mut repo,
            "Banco en dólares",
            "banco",
            "USD",
            "200.00",
            "d1",
        )
        .unwrap();
        ejecutar_crear_cuenta(
            &mut repo,
            "Billetera USDT",
            "billetera digital",
            "USDT",
            "50.00",
            "d1",
        )
        .unwrap();

        // Registra tasas de USD->VES y USDT->VES con sus fechas y fuentes
        let t_usd = TasaCambio {
            id: "t-usd-ves".into(),
            origen: "USD".into(),
            destino: "VES".into(),
            valor: Decimal::from_str("887.00").unwrap(),
            fecha: "2026-10-20".into(),
            fuente: "BCV".into(),
        };
        repo.guardar_evento(&EventoLibro {
            id: "t-usd-ves".into(),
            tipo: "TasaRegistrada".into(),
            momento_registro: Utc::now(),
            dispositivo_origen: "d1".into(),
            datos_json: serde_json::to_string(&t_usd).unwrap(),
        })
        .unwrap();

        let t_usdt = TasaCambio {
            id: "t-usdt-ves".into(),
            origen: "USDT".into(),
            destino: "VES".into(),
            valor: Decimal::from_str("1000.00").unwrap(),
            fecha: "2026-10-21".into(),
            fuente: "casa de cambio".into(),
        };
        repo.guardar_evento(&EventoLibro {
            id: "t-usdt-ves".into(),
            tipo: "TasaRegistrada".into(),
            momento_registro: Utc::now(),
            dispositivo_origen: "d1".into(),
            datos_json: serde_json::to_string(&t_usdt).unwrap(),
        })
        .unwrap();

        // Cambia la moneda base a VES y verifica el total de 316100.00 VES
        repo.guardar_evento(&EventoLibro {
            id: "base-ves".into(),
            tipo: "MonedaBaseCambiada".into(),
            momento_registro: Utc::now(),
            dispositivo_origen: "d1".into(),
            datos_json: serde_json::to_string(&DatosMonedaBaseCambiada {
                codigo: "VES".into(),
            })
            .unwrap(),
        })
        .unwrap();

        let pat_ves = consultar_fixture_en_fecha(&repo, fecha).unwrap();
        assert_eq!(pat_ves.total.as_deref(), Some("316100.00"));
        assert!(pat_ves.faltantes.is_empty());

        // Cambia la base a USD; falta USDT->USD, omite total y muestra aviso
        repo.guardar_evento(&EventoLibro {
            id: "base-usd".into(),
            tipo: "MonedaBaseCambiada".into(),
            momento_registro: Utc::now(),
            dispositivo_origen: "d1".into(),
            datos_json: serde_json::to_string(&DatosMonedaBaseCambiada {
                codigo: "USD".into(),
            })
            .unwrap(),
        })
        .unwrap();

        let pat_usd_incompleto = consultar_fixture_en_fecha(&repo, fecha).unwrap();
        assert!(pat_usd_incompleto.total.is_none());
        assert_eq!(
            aviso_faltante(&pat_usd_incompleto).as_deref(),
            Some("Falta una tasa de USDT a USD con fecha igual o anterior al 23/10/2026")
        );
        assert_eq!(pat_usd_incompleto.subtotales.len(), 3);

        // Registra 1 USDT = 1.00 USD y verifica que el total pase a 350.00 USD
        let t_usdt_usd = TasaCambio {
            id: "t-usdt-usd".into(),
            origen: "USDT".into(),
            destino: "USD".into(),
            valor: Decimal::from_str("1.00").unwrap(),
            fecha: "2026-10-23".into(),
            fuente: "casa de cambio".into(),
        };
        repo.guardar_evento(&EventoLibro {
            id: "t-usdt-usd".into(),
            tipo: "TasaRegistrada".into(),
            momento_registro: Utc::now(),
            dispositivo_origen: "d1".into(),
            datos_json: serde_json::to_string(&t_usdt_usd).unwrap(),
        })
        .unwrap();

        let pat_usd_completo = consultar_fixture_en_fecha(&repo, fecha).unwrap();
        assert_eq!(pat_usd_completo.total.as_deref(), Some("350.00"));
        assert!(pat_usd_completo.faltantes.is_empty());
    }
}

#[test]
fn probar_crear_cuentas() {
    let mut repo = RepositorioEnMemoria::default();

    // Creamos una cuenta
    let cuenta =
        ejecutar_crear_cuenta(&mut repo, "Cartera", "efectivo", "VES", "100.25", "prueba").unwrap();

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
        let resp = ejecutar_crear_cuenta(&mut repo, nombre, tipo, moneda, saldo, "prueba");

        assert!(resp.is_err());
        assert_eq!(resp.unwrap_err().campo, campo);
    }

    // Solo se debe guardar la primera cuenta
    assert_eq!(repo.obtener_eventos().unwrap().len(), 1);

    // Creamos las otras cuentas
    ejecutar_crear_cuenta(&mut repo, "Banco", "banco", "USD", "0.10", "prueba").unwrap();

    ejecutar_crear_cuenta(&mut repo, "Ahorros", "banco", "USD", "0.20", "prueba").unwrap();

    ejecutar_crear_cuenta(
        &mut repo,
        "Billetera",
        "billetera digital",
        "USDT",
        "0",
        "prueba",
    )
    .unwrap();

    // Vemos las cuentas y los subtotales
    let lista = ejecutar_listar_cuentas(&repo).unwrap();

    assert_eq!(lista.cuentas.len(), 4);
    assert_eq!(lista.subtotales.len(), 3);

    let usd = lista
        .subtotales
        .iter()
        .find(|dato| dato.codigo == "USD")
        .unwrap();

    assert_eq!(usd.subtotal, "0.30");

    // Vemos si la cuenta sigue usando VES
    let cartera = lista
        .cuentas
        .iter()
        .find(|c| c.nombre() == "Cartera")
        .unwrap();

    assert_eq!(cartera.moneda(), "VES");
    assert_eq!(cartera.saldo().moneda(), "VES");

    // Agregamos el mismo evento otra vez
    let eventos = repo.obtener_eventos().unwrap();
    let mut repetidos = eventos.clone();
    repetidos.push(eventos[0].clone());

    assert_eq!(proyectar_libro(&repetidos).unwrap().cuentas.len(), 4);
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluacionMovimiento {
    pub deja_saldo_negativo: bool,
    pub saldo_resultante: String,
}

pub fn evaluar_movimiento<R: RepositorioEventos>(
    repo: &R,
    cuenta_id: &str,
    tipo: &str,
    monto: &str,
) -> Result<EvaluacionMovimiento, ErrorDominio> {
    let eventos = repo
        .obtener_eventos()
        .map_err(|_| ErrorDominio::nuevo("general", "No se pudo leer el libro local."))?;
    let libro = proyectar_libro(&eventos)?;
    let cuenta = libro
        .cuentas
        .get(cuenta_id)
        .ok_or_else(|| ErrorDominio::nuevo("cuenta", "La cuenta seleccionada no existe."))?;

    let tipo = TipoMovimiento::desde_str(tipo)?;
    // Calcula y formatea el saldo resultante con la escala de la moneda
    let moneda = buscar_moneda_en(&cuenta.moneda, &libro.monedas)?;

    let monto = Monto::desde_str_con_moneda(monto, &moneda, "monto", false)?;
    let efecto = match tipo {
        TipoMovimiento::Ingreso => monto.cantidad(),
        TipoMovimiento::Gasto => -monto.cantidad(),
    };
    let saldo = sumar_exacto(cuenta.saldo_actual()?, efecto)?;
    validar_efecto_subtotal(&libro, &cuenta.moneda, efecto)?;
    Ok(EvaluacionMovimiento {
        deja_saldo_negativo: tipo == TipoMovimiento::Gasto && saldo < Decimal::ZERO,
        saldo_resultante: format!("{:.*}", moneda.decimales as usize, saldo),
    })
}

/// El adaptador obtiene hoy del reloj local del dispositivo; las pruebas inyectan una fecha civil.
pub fn validar_fecha_movimiento(fecha: &str, hoy: NaiveDate) -> Result<NaiveDate, ErrorDominio> {
    let fecha = NaiveDate::parse_from_str(fecha.trim(), "%Y-%m-%d").map_err(|_| {
        ErrorDominio::nuevo("fecha", "La fecha debe tener el formato válido AAAA-MM-DD.")
    })?;
    if fecha > hoy {
        return Err(ErrorDominio::nuevo(
            "fecha",
            "No se permiten movimientos con fecha futura.",
        ));
    }
    Ok(fecha)
}

// Dependencia mínima de aplicación para consultar_patrimonio. Reutiliza el libro,
// catálogo y moneda base existentes; no guarda conversiones ni saldos derivados.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilaPatrimonio {
    pub cuenta_id: String,
    pub nombre: String,
    pub moneda: String,
    pub saldo: String,
    pub tasa: Option<TasaCambio>,
    pub inversa: bool,
    pub en_base: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtotalPatrimonio {
    pub codigo: String,
    pub simbolo: String,
    pub subtotal: String,
    pub tasa: Option<TasaCambio>,
    pub inversa: bool,
    pub en_base: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ParFaltante {
    pub origen: String,
    pub destino: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Patrimonio {
    pub moneda_base: String,
    pub fecha: String,
    pub filas: Vec<FilaPatrimonio>,
    pub subtotales: Vec<SubtotalPatrimonio>,
    pub total: Option<String>,
    pub faltantes: Vec<ParFaltante>,
}

pub fn ejecutar_consultar_patrimonio<R: RepositorioEventos>(
    repo: &R,
    fecha: Option<&str>,
    hoy: NaiveDate,
) -> Result<Patrimonio, ErrorDominio> {
    let dia = match fecha {
        Some(f) => validar_fecha_movimiento(f, hoy)?,
        None => hoy,
    };
    let fecha = dia.to_string();
    let eventos = repo.obtener_eventos().map_err(|_| error_libro())?;
    let libro = proyectar_libro(&eventos)?;
    let base = buscar_moneda_en(&libro.moneda_base, &libro.monedas)?;
    let resumen = calcular_saldos_en_fecha(&libro, dia)?;
    let convertir = |cantidad: Decimal,
                     moneda: &str|
     -> Result<(Option<Decimal>, Option<TasaCambio>, bool), ErrorDominio> {
        match convertir_monto_a_base(cantidad, moneda, &base.codigo, dia, &libro.tasas) {
            Ok((valor, tasa, inversa)) => Ok((Some(valor), tasa, inversa)),
            Err(e) if e.campo == "tasa" => Ok((None, None, false)),
            Err(e) => Err(e),
        }
    };
    let mut filas = Vec::new();
    for cuenta in resumen.cuentas {
        let moneda = buscar_moneda_en(cuenta.moneda(), &libro.monedas)?;
        let (en_base, tasa, inversa) = convertir(cuenta.saldo().cantidad(), cuenta.moneda())?;
        filas.push(FilaPatrimonio {
            cuenta_id: cuenta.id().to_string(),
            nombre: cuenta.nombre().to_string(),
            moneda: cuenta.moneda().to_string(),
            saldo: format!(
                "{:.*}",
                moneda.decimales as usize,
                cuenta.saldo().cantidad()
            ),
            tasa,
            inversa,
            en_base: en_base.map(|v| v.to_string()),
        });
    }
    let mut subtotales = Vec::new();
    let mut faltantes = Vec::new();
    let mut total = Decimal::ZERO;
    for subtotal in resumen.subtotales {
        let cantidad = Decimal::from_str_exact(&subtotal.subtotal).map_err(|_| error_libro())?;
        let (en_base, tasa, inversa) = convertir(cantidad, &subtotal.codigo)?;
        if let Some(valor) = en_base {
            total = sumar_exacto(total, valor)?;
        } else {
            faltantes.push(ParFaltante {
                origen: subtotal.codigo.clone(),
                destino: base.codigo.clone(),
            });
        }
        subtotales.push(SubtotalPatrimonio {
            codigo: subtotal.codigo,
            simbolo: subtotal.simbolo,
            subtotal: subtotal.subtotal,
            tasa,
            inversa,
            en_base: en_base.map(|v| v.to_string()),
        });
    }
    Ok(Patrimonio {
        moneda_base: base.codigo,
        fecha,
        filas,
        subtotales,
        total: if faltantes.is_empty() {
            Some(format!(
                "{:.*}",
                base.decimales as usize,
                total.round_dp(base.decimales)
            ))
        } else {
            None
        },
        faltantes,
    })
}
