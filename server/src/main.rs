//! main.rs: Servidor HTTP de Cuentas Claras, rutas principales y modelos de eventos
//!
//! Dirigido al equipo backend para gestionar el arranque y contratos de sincronización

pub mod amistades_db;
mod auth;
mod db;
pub mod division;
pub mod gastos_db;
mod grupos;
pub mod sincronizacion_db;
mod social;
pub mod usuarios_db;

use axum::{
    Json, Router,
    extract::{
        Path, Query, State as AxumState,
        rejection::{JsonRejection, QueryRejection},
    },
    http::{HeaderMap, StatusCode},
    routing::get,
};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Transaction};
use std::collections::HashSet;
use uuid::Uuid;

use crate::division::Regla;
use crate::gastos_db::{ErrorGasto, GastoNuevo, ParticipanteNuevo, ResultadoRegistroGasto};
use crate::sincronizacion_db::{EventoUsuario, PaginaCambiosDb};

#[derive(Serialize)]
struct PingResponse {
    estado: &'static str,
    mensaje: &'static str,
    version: &'static str,
    timestamp: String,
}

// Identificador textual fijo para eventos de gastos compartidos de grupo
#[allow(dead_code)]
pub const TIPO_EVENTO_GASTO_GRUPO: &str = "gasto_grupo";

// Carga útil inmutable con la parte de un gasto compartido por participante
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DatosParteGasto {
    pub grupo_id: Uuid,
    pub gasto_id: Uuid,
    pub concepto: String,
    pub fecha: NaiveDate,
    pub moneda: String,
    pub monto_parte: String,
    pub recibio_residuo: bool,
}

// Evento inmutable del registro de cambios por usuario, con id y cursor creciente
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventoCambio {
    id: Uuid,
    /// Posición creciente por usuario usada como cursor de sincronización
    posicion: i64,
    /// Categoría textual del evento para soportar tipos futuros
    tipo: String,
    /// Carga útil con los datos del gasto compartido
    datos: DatosParteGasto,
}

#[allow(dead_code)]
impl EventoCambio {
    // Crea un evento inmutable de gasto grupal y falla si la posición no es positiva
    pub fn nuevo_gasto_grupo(
        id: Uuid,
        posicion: i64,
        datos: DatosParteGasto,
    ) -> Result<Self, &'static str> {
        // Replica la restricción CHECK (posicion > 0) de la tabla eventos_usuario.
        if posicion <= 0 {
            return Err("La posición del evento debe ser mayor que cero");
        }

        Ok(Self {
            id,
            posicion,
            tipo: String::from(TIPO_EVENTO_GASTO_GRUPO),
            datos,
        })
    }

    // Convierte un registro de base de datos en un evento inmutable validado
    pub fn desde_evento_db(evento: EventoUsuario) -> Result<Self, &'static str> {
        if evento.posicion <= 0 {
            return Err("La posición del evento debe ser mayor que cero");
        }

        // Agrupa los campos planos de SQLx en el payload del gasto
        let datos = DatosParteGasto {
            grupo_id: evento.grupo_id,
            gasto_id: evento.gasto_id,
            concepto: evento.concepto,
            fecha: evento.fecha,
            moneda: evento.moneda,
            monto_parte: evento.monto_parte,
            recibio_residuo: evento.recibio_residuo,
        };

        Ok(Self {
            id: evento.id,
            posicion: evento.posicion,
            tipo: evento.tipo,
            datos,
        })
    }

    // Devuelve el identificador del evento
    pub fn id(&self) -> Uuid {
        self.id
    }

    // Devuelve la posición usada como cursor
    pub fn posicion(&self) -> i64 {
        self.posicion
    }

    // Devuelve el tipo textual del evento
    pub fn tipo(&self) -> &str {
        &self.tipo
    }

    // Devuelve una referencia de solo lectura a los datos del gasto
    pub fn datos(&self) -> &DatosParteGasto {
        &self.datos
    }
}

// Errores de validación de membresía y destinatarios al emitir eventos de cambio
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorDestinatarios {
    // El grupo no existe o el solicitante no es miembro activo
    GrupoNoAccesible,
    // El gasto no contiene participantes para emitir eventos
    SinParticipantes,
    // Un participante aparece repetido en la división del gasto
    ParticipanteDuplicado(Uuid),
    // Un participante del gasto no pertenece activamente al grupo
    ParticipanteNoMiembro(Uuid),
}

#[allow(dead_code)]
impl ErrorDestinatarios {
    // Devuelve el código HTTP correspondiente, ocultando inexistencia o falta de membresía con 404
    #[allow(dead_code)]
    pub fn codigo_http(&self) -> StatusCode {
        match self {
            Self::GrupoNoAccesible => StatusCode::NOT_FOUND,
            Self::SinParticipantes
            | Self::ParticipanteDuplicado(_)
            | Self::ParticipanteNoMiembro(_) => StatusCode::BAD_REQUEST,
        }
    }

    // Convierte el error de validación al tipo compartido por la capa de gastos
    pub fn a_error_gasto(self) -> ErrorGasto {
        match self {
            Self::GrupoNoAccesible => ErrorGasto::SinAcceso,
            Self::SinParticipantes | Self::ParticipanteDuplicado(_) => {
                ErrorGasto::Invalido("Selecciona participantes únicos.".into())
            }
            Self::ParticipanteNoMiembro(_) => {
                ErrorGasto::Invalido("Cada participante debe ser miembro del grupo.".into())
            }
        }
    }
}

// Valida membresía del solicitante y devuelve solo los participantes activos para emitir eventos
pub fn filtrar_destinatarios_evento(
    solicitante_id: Uuid,
    miembros_activos: &HashSet<Uuid>,
    participantes_gasto: &[Uuid],
) -> Result<Vec<Uuid>, ErrorDestinatarios> {
    // Oculta si el grupo no existe o si el solicitante no pertenece
    if miembros_activos.is_empty() || !miembros_activos.contains(&solicitante_id) {
        return Err(ErrorDestinatarios::GrupoNoAccesible);
    }

    if participantes_gasto.is_empty() {
        return Err(ErrorDestinatarios::SinParticipantes);
    }

    // Evita emitir duplicados y rechaza usuarios ajenos al grupo
    let mut vistos = HashSet::with_capacity(participantes_gasto.len());
    let mut destinatarios = Vec::with_capacity(participantes_gasto.len());

    for &participante_id in participantes_gasto {
        if !miembros_activos.contains(&participante_id) {
            return Err(ErrorDestinatarios::ParticipanteNoMiembro(participante_id));
        }
        if !vistos.insert(participante_id) {
            return Err(ErrorDestinatarios::ParticipanteDuplicado(participante_id));
        }
        destinatarios.push(participante_id);
    }

    // Ordena los ids igual que sincronizacion_db para evitar bloqueos cruzados en PostgreSQL
    destinatarios.sort();
    Ok(destinatarios)
}

// Persiste eventos inmutables para participantes válidos en una transacción y devuelve sus destinatarios
pub async fn emitir_eventos_gasto(
    tx: &mut Transaction<'_, Postgres>,
    solicitante_id: Uuid,
    miembros_activos: &HashSet<Uuid>,
    participantes_gasto: &[Uuid],
    gasto_id: Uuid,
) -> Result<Vec<Uuid>, ErrorGasto> {
    // Verifica membresía y excluye no participantes antes de escribir en base de datos
    let destinatarios =
        filtrar_destinatarios_evento(solicitante_id, miembros_activos, participantes_gasto)
            .map_err(ErrorDestinatarios::a_error_gasto)?;
    // Delega el INSERT con UUID y cursor creciente, propagando errores SQLx
    sincronizacion_db::guardar_eventos_gasto(tx, gasto_id).await?;

    Ok(destinatarios)
}

async fn ping() -> Json<PingResponse> {
    Json(PingResponse {
        estado: "ok",
        mensaje: "Servidor Cuentas Claras activo",
        version: "0.1.0",
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

// Nombre estándar del encabezado HTTP para claves de idempotencia
pub const ENCABEZADO_IDEMPOTENCY_KEY: &str = "idempotency-key";

// Límite máximo de bytes permitidos para una clave de idempotencia
pub const LONGITUD_MAXIMA_CLAVE_IDEMPOTENCIA: usize = 128;

// Mensaje uniforme para ocultar si un grupo existe o falta membresía
const MENSAJE_GRUPO_NO_ENCONTRADO: &str = "Grupo no encontrado.";

// Clave de idempotencia HTTP para evitar gastos duplicados
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaveIdempotencia(String);

#[allow(dead_code)]
impl ClaveIdempotencia {
    // Construye una clave válida y rechaza textos vacíos, extensos o inválidos
    pub fn nueva(valor: &str) -> Result<Self, ErrorPrecondicionGasto> {
        if valor.is_empty() {
            return Err(ErrorPrecondicionGasto::ClaveInvalida(
                "Idempotency-Key no puede estar vacío.",
            ));
        }

        if valor.len() > LONGITUD_MAXIMA_CLAVE_IDEMPOTENCIA {
            return Err(ErrorPrecondicionGasto::ClaveInvalida(
                "Idempotency-Key excede el límite permitido.",
            ));
        }

        // Rechaza espacios y caracteres de control para evitar claves ambiguas
        if !valor.bytes().all(|b| b.is_ascii_graphic()) {
            return Err(ErrorPrecondicionGasto::ClaveInvalida(
                "Idempotency-Key contiene caracteres inválidos.",
            ));
        }

        Ok(Self(valor.to_owned()))
    }

    // Devuelve la clave validada como referencia de texto
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// Resultado validado de sesion, acceso al grupo y clave opcional
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecondicionesGasto {
    pub usuario_id: Uuid,
    pub clave_idempotencia: Option<ClaveIdempotencia>,
}

// Errores de precondiciones HTTP al registrrar un gasto de grupo
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorPrecondicionGasto {
    // Sesión ausente, inválida o expirada
    NoAutorizado,
    // Grupo inexistente o usuario sin membresía aceptada
    GrupoNoAccesible,
    // Encabezado Idempotency-Key con formato o cantidad inválidos
    ClaveInvalida(&'static str),
    // Fallo inesperado al consultar la base de datos
    ErrorInterno,
}

impl ErrorPrecondicionGasto {
    // Traduce el error a código HTTP y se representa en JSON
    pub fn a_respuesta_http(&self) -> (StatusCode, Json<Value>) {
        match self {
            Self::NoAutorizado => (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "mensaje": "Sesión inválida o vencida." })),
            ),
            Self::GrupoNoAccesible => (
                StatusCode::NOT_FOUND,
                Json(json!({ "mensaje": MENSAJE_GRUPO_NO_ENCONTRADO })),
            ),
            Self::ClaveInvalida(mensaje) => {
                (StatusCode::BAD_REQUEST, Json(json!({ "mensaje": mensaje })))
            }
            Self::ErrorInterno => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "mensaje": "No se pudo completar la operación." })),
            ),
        }
    }
}

// Extrae y valida el encabezado Idempotency-Key opcional desde la petición
pub fn extraer_clave_idempotencia(
    headers: &HeaderMap,
) -> Result<Option<ClaveIdempotencia>, ErrorPrecondicionGasto> {
    let mut valores = headers.get_all(ENCABEZADO_IDEMPOTENCY_KEY).iter();
    let Some(primero) = valores.next() else {
        return Ok(None);
    };

    // Rechaza múltiples encabezados para evitar ambigüedad en la deduplicación
    if valores.next().is_some() {
        return Err(ErrorPrecondicionGasto::ClaveInvalida(
            "Envía un solo encabezado Idempotency-Key.",
        ));
    }

    let texto = primero.to_str().map_err(|_| {
        ErrorPrecondicionGasto::ClaveInvalida("Idempotency-Key debe contener texto ASCII válido.")
    })?;

    ClaveIdempotencia::nueva(texto).map(Some)
}

// Verifica membresía aceptada y devuelve 404 uniforme si no existe acceso
pub fn validar_acceso_grupo(es_miembro_aceptado: bool) -> Result<(), ErrorPrecondicionGasto> {
    if es_miembro_aceptado {
        Ok(())
    } else {
        Err(ErrorPrecondicionGasto::GrupoNoAccesible)
    }
}

// Comprueba sesión, membresía uniforme del grupo y encabezado de idempotencia
pub async fn validar_precondiciones_gasto(
    estado: &auth::State,
    headers: &HeaderMap,
    grupo_id: Uuid,
) -> Result<PrecondicionesGasto, ErrorPrecondicionGasto> {
    let usuario_id =
        auth::autenticar(headers, estado).map_err(|_| ErrorPrecondicionGasto::NoAutorizado)?;

    // Valida acceso al grupo antes del encabezado para no filtrar información.
    let es_miembro: Option<bool> = sqlx::query_scalar(
        "SELECT es_admin FROM grupo_miembros WHERE grupo_id = $1 AND usuario_id = $2 AND estado_invitacion = 'aceptada'",
    )
    .bind(grupo_id)
    .bind(usuario_id)
    .fetch_optional(&estado.pool)
    .await
    .map_err(|_| ErrorPrecondicionGasto::ErrorInterno)?;

    validar_acceso_grupo(es_miembro.is_some())?;

    let clave_idempotencia = extraer_clave_idempotencia(headers)?;
    Ok(PrecondicionesGasto {
        usuario_id,
        clave_idempotencia,
    })
}

// Ejecuta el registro atómico del gasto aplicando la clave validada
pub async fn registrar_gasto_idempotente(
    pool: &PgPool,
    precondiciones: &PrecondicionesGasto,
    datos: GastoNuevo,
) -> Result<ResultadoRegistroGasto, ErrorGasto> {
    let clave = precondiciones
        .clave_idempotencia
        .as_ref()
        .map(ClaveIdempotencia::as_str);

    // Delega la reserva de clave, inserción y eventos en una sola transacción
    gastos_db::registrar_con_idempotencia(pool, precondiciones.usuario_id, datos, clave).await
}

// Mensjae uniforme cuando se reutiliza una clave con datos distintos
const MENSAJE_CONFLICTO_IDEMPOTENCIA: &str = "Clave Idempotency-Key ya usada con datos distintos.";

// Participante recibido en el cuerpo JSON para registrar un gasto
#[derive(Deserialize)]
pub struct SolicitudParticipanteGasto {
    pub usuario_id: Uuid,
    pub valor: Option<String>,
}

// Solicitud HTTP para crear un gasto compartido en un grupo
#[derive(Deserialize)]
pub struct SolicitudRegistrarGasto {
    pub concepto: String,
    pub monto: String,
    pub fecha: NaiveDate,
    pub pagado_por: Uuid,
    pub regla: Regla,
    pub participantes: Vec<SolicitudParticipanteGasto>,
}

impl SolicitudRegistrarGasto {
    // Convierte la solicitud HTTP en un gasto listo para persistir
    fn a_gasto_nuevo(self, grupo_id: Uuid) -> GastoNuevo {
        GastoNuevo {
            grupo_id,
            pagado_por: self.pagado_por,
            concepto: self.concepto,
            monto: self.monto,
            fecha: self.fecha,
            regla: self.regla,
            participantes: self
                .participantes
                .into_iter()
                .map(|p| ParticipanteNuevo {
                    usuario_id: p.usuario_id,
                    valor: p.valor,
                })
                .collect(),
        }
    }
}

// Traduce el resultado de persistencia a 201, 200 o 409
pub fn mapear_resultado_gasto_http(resultado: ResultadoRegistroGasto) -> (StatusCode, Json<Value>) {
    match resultado {
        ResultadoRegistroGasto::Creado(gasto) => (StatusCode::CREATED, Json(json!(gasto))),
        ResultadoRegistroGasto::Existente(gasto) => (StatusCode::OK, Json(json!(gasto))),
        ResultadoRegistroGasto::ConflictoIdempotencia => (
            StatusCode::CONFLICT,
            Json(json!({ "mensaje": MENSAJE_CONFLICTO_IDEMPOTENCIA })),
        ),
    }
}

// Traduce errores de dominio o base de datos a respuestas HTTP
pub fn mapear_error_gasto_http(error: ErrorGasto) -> (StatusCode, Json<Value>) {
    match error {
        ErrorGasto::SinAcceso => (
            StatusCode::NOT_FOUND,
            Json(json!({ "mensaje": MENSAJE_GRUPO_NO_ENCONTRADO })),
        ),
        ErrorGasto::Invalido(mensaje) => {
            (StatusCode::BAD_REQUEST, Json(json!({ "mensaje": mensaje })))
        }
        ErrorGasto::Base(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "mensaje": "No se pudo completar la operación." })),
        ),
    }
}

// Procesa POST /grupos/{id}/gastos aplicando idempotencia y códigos HTTP requeridos
pub async fn registrar_gasto_http(
    AxumState(estado): AxumState<auth::State>,
    headers: HeaderMap,
    Path(grupo_id): Path<Uuid>,
    cuerpo: Result<Json<SolicitudRegistrarGasto>, JsonRejection>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    // Valida sesión (401), membresía uniforme (404) y formato de clave (400).
    let precondiciones = validar_precondiciones_gasto(&estado, &headers, grupo_id)
        .await
        .map_err(|error| error.a_respuesta_http())?;

    let Json(solicitud) = cuerpo.map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "mensaje": "Datos de gasto inválidos." })),
        )
    })?;

    let resultado = registrar_gasto_idempotente(
        &estado.pool,
        &precondiciones,
        solicitud.a_gasto_nuevo(grupo_id),
    )
    .await
    .map_err(mapear_error_gasto_http)?;

    Ok(mapear_resultado_gasto_http(resultado))
}

// Límite máximo de eventos devueltos en cada lote de GET /cambios
#[allow(dead_code)]
pub const LIMITE_EVENTOS_CAMBIOS: usize = 100;

// Parámetros de consulta HTTP recibidos en GET /cambios
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ParametrosCambios {
    pub desde: Option<String>,
}

// Errores de validación al interpretar el cursor de consulta desde
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCursorCambios {
    // Cursor vacío, negativo, no numérico o fuera de rango
    CursorInvalido(&'static str),
}

#[allow(dead_code)]
impl ErrorCursorCambios {
    // Traduce el error del cursor a una respuesta HTTP 400 en JSON
    pub fn a_respuesta_http(&self) -> (StatusCode, Json<Value>) {
        match self {
            Self::CursorInvalido(mensaje) => (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "codigo": "cursor_invalido",
                    "mensaje": mensaje
                })),
            ),
        }
    }
}

// Valida el parámetro desde y devuelve un cursor no negativo o error
#[allow(dead_code)]
pub fn validar_cursor_desde(desde: Option<&str>) -> Result<i64, ErrorCursorCambios> {
    // Si se omite el parámetro, la sincronización inicia desde cero
    let Some(texto) = desde else {
        return Ok(0);
    };

    // Exige dígitos ASCII para rechazar negativos, signos, espacios o texto
    if texto.is_empty() || !texto.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ErrorCursorCambios::CursorInvalido(
            "El cursor desde debe ser un entero mayor o igual a cero.",
        ));
    }

    texto.parse::<i64>().map_err(|_| {
        ErrorCursorCambios::CursorInvalido("El cursor desde excede el rango permitido.")
    })
}

// Evento compartido en JSON con la parte del gasto sin datos personales
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventoCambioRespuesta {
    pub id: Uuid,
    pub cursor: i64,
    pub tipo: String,
    pub grupo_id: Uuid,
    pub grupo_nombre: String,
    pub gasto_id: Uuid,
    pub concepto: String,
    pub fecha: NaiveDate,
    pub moneda: String,
    pub monto_parte: String,
    pub incluye_centimos_redondeo: bool,
}

#[allow(dead_code)]
impl EventoCambioRespuesta {
    // Convierte una fila de base de datos en un evento HTTP validado
    pub fn desde_evento_db(
        evento: EventoUsuario,
        grupo_nombre: String,
    ) -> Result<Self, &'static str> {
        // Conserva la regla de posición positiva definida en eventos_usuario
        if evento.posicion <= 0 {
            return Err("El cursor del evento debe ser mayor que cero");
        }

        Ok(Self {
            id: evento.id,
            cursor: evento.posicion,
            tipo: evento.tipo,
            grupo_id: evento.grupo_id,
            grupo_nombre,
            gasto_id: evento.gasto_id,
            concepto: evento.concepto,
            fecha: evento.fecha,
            moneda: evento.moneda,
            monto_parte: evento.monto_parte,
            incluye_centimos_redondeo: evento.recibio_residuo,
        })
    }
}

// Respuesta JSON paginada de GET /cambios con eventos, nuevo cursor y continuidad
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RespuestaCambios {
    pub eventos: Vec<EventoCambioRespuesta>,
    pub nuevo_cursor: i64,
    pub hay_mas: bool,
}

#[allow(dead_code)]
impl RespuestaCambios {
    // Construye una respuesta paginada validando cursor no negativo y límite del lote
    pub fn nueva(
        eventos: Vec<EventoCambioRespuesta>,
        nuevo_cursor: i64,
        hay_mas: bool,
    ) -> Result<Self, &'static str> {
        if nuevo_cursor < 0 {
            return Err("El nuevo cursor no puede ser negativo");
        }
        if eventos.len() > LIMITE_EVENTOS_CAMBIOS {
            return Err("El lote supera el máximo de 100 eventos");
        }

        Ok(Self {
            eventos,
            nuevo_cursor,
            hay_mas,
        })
    }
}

// Convierte una página de base de datos en la respuesta HTTP validada
fn construir_respuesta_cambios(pagina: PaginaCambiosDb) -> Result<RespuestaCambios, &'static str> {
    let eventos = pagina
        .eventos
        .into_iter()
        .map(|item| EventoCambioRespuesta::desde_evento_db(item.evento, item.grupo_nombre))
        .collect::<Result<Vec<_>, _>>()?;

    RespuestaCambios::nueva(eventos, pagina.nuevo_cursor, pagina.hay_mas)
}

// Responde errores internos de sincronización con estado HTTP 500
fn error_interno_cambios() -> (StatusCode, Json<Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({
            "codigo": "error_interno",
            "mensaje": "No se pudo completar la operación."
        })),
    )
}

// Procesa GET /cambios validando sesión, cursor y devolviendo hasta cien eventos
pub async fn obtener_cambios_http(
    AxumState(estado): AxumState<auth::State>,
    headers: HeaderMap,
    parametros: Result<Query<ParametrosCambios>, QueryRejection>,
) -> Result<(StatusCode, Json<RespuestaCambios>), (StatusCode, Json<Value>)> {
    // Valida el token JWT antes del cursor para proteger el recurso
    let usuario_id = auth::autenticar(&headers, &estado).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "codigo": "no_autorizado",
                "mensaje": "Sesión inválida o vencida."
            })),
        )
    })?;

    let Query(consulta) = parametros.map_err(|_| {
        ErrorCursorCambios::CursorInvalido("El parámetro de consulta desde es inválido.")
            .a_respuesta_http()
    })?;

    let desde =
        validar_cursor_desde(consulta.desde.as_deref()).map_err(|e| e.a_respuesta_http())?;

    let pagina = sincronizacion_db::consultar_cambios_paginados(&estado.pool, usuario_id, desde)
        .await
        .map_err(|_| error_interno_cambios())?;

    let respuesta = construir_respuesta_cambios(pagina).map_err(|_| error_interno_cambios())?;
    Ok((StatusCode::OK, Json(respuesta)))
}

// Construye el enrutador HTTP principal con todas las rutas del servidor
pub fn construir_app(estado: auth::State) -> Router {
    Router::new()
        .route("/ping", get(ping))
        .route("/cambios", get(obtener_cambios_http))
        .with_state(estado.clone())
        .merge(grupos::routes(estado.clone()))
        .merge(social::routes(estado.clone()))
        .merge(auth::routes(estado))
}

// Carga variables desde .env (CWD o directorio de server/) sin sobrescribir las ya exportadas
fn cargar_entorno() {
    if dotenvy::dotenv().is_err() {
        let ruta_manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".env");
        let _ = dotenvy::from_path(ruta_manifest);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    cargar_entorno();

    let database_url = std::env::var("DATABASE_URL").map_err(|_| "Falta DATABASE_URL")?;
    let secret = std::env::var("JWT_SECRET").map_err(|_| "Falta JWT_SECRET")?;
    if secret.len() < 32 {
        return Err("JWT_SECRET debe tener al menos 32 bytes".into());
    }

    let pool = db::conectar_y_migrar(&database_url).await?;

    let modo_demostracion = std::env::var("TOKEN_DEMO_1_MINUTO").as_deref() == Ok("1");
    let estado = auth::State::new(pool, secret).con_modo_demostracion(modo_demostracion);
    let app = construir_app(estado);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000").await?;
    println!("Servidor activo en http://127.0.0.1:8000");
    axum::serve(listener, app).await?;
    Ok(())
}

// Pruebas unitarias de emisión de eventos e invariante INV-17
#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderName, HeaderValue};
    use std::collections::HashMap;

    // Crea un UUID determinista para identificar entidades en pruebas
    fn id_fijo(valor: u128) -> Uuid {
        Uuid::from_u128(valor)
    }

    // Cuota individual de un participante dentro de un gasto grupal.
    struct PartePrueba {
        usuario_id: Uuid,
        monto_parte: &'static str,
        recibio_residuo: bool,
    }

    // Metadatos compartidos de un gasto grupal para casos de prueba
    struct GastoPrueba<'a> {
        grupo_id: Uuid,
        gasto_id: Uuid,
        concepto: &'a str,
        fecha: NaiveDate,
        moneda: &'a str,
    }

    // Registro en memoria por usuario para validar generación de eventos.
    struct RegistroCambiosPrueba {
        siguiente_id_evento: u128,
        eventos_por_usuario: HashMap<Uuid, Vec<EventoCambio>>,
    }

    impl RegistroCambiosPrueba {
        // Inicializa un registro vacío con secuenta determinista de identificación
        fn nuevo() -> Self {
            Self {
                siguiente_id_evento: 1000,
                eventos_por_usuario: HashMap::new(),
            }
        }

        // Valida destinatarios con INV-17 y agrega eventos inmutables por participantes
        fn registrar_gasto(
            &mut self,
            solicitante_id: Uuid,
            miembros_activos: &HashSet<Uuid>,
            gasto: &GastoPrueba<'_>,
            partes: &[PartePrueba],
        ) -> Result<(), ErrorDestinatarios> {
            let participantes: Vec<Uuid> = partes.iter().map(|p| p.usuario_id).collect();
            // Bloquea la emisión completa si falla la membresía o algún participante.
            let destinatarios =
                filtrar_destinatarios_evento(solicitante_id, miembros_activos, &participantes)?;

            let mapa_partes: HashMap<Uuid, &PartePrueba> =
                partes.iter().map(|p| (p.usuario_id, p)).collect();

            for usuario_id in destinatarios {
                let parte = mapa_partes[&usuario_id];
                let lista = self.eventos_por_usuario.entry(usuario_id).or_default();
                // Incrementa la posición local del usuario para conservar cursor creciente.
                let posicion =
                    i64::try_from(lista.len() + 1).expect("La posición debe caber en i64");
                let evento_id = id_fijo(self.siguiente_id_evento);
                self.siguiente_id_evento += 1;

                let datos = DatosParteGasto {
                    grupo_id: gasto.grupo_id,
                    gasto_id: gasto.gasto_id,
                    concepto: String::from(gasto.concepto),
                    fecha: gasto.fecha,
                    moneda: String::from(gasto.moneda),
                    monto_parte: String::from(parte.monto_parte),
                    recibio_residuo: parte.recibio_residuo,
                };

                let evento = EventoCambio::nuevo_gasto_grupo(evento_id, posicion, datos)
                    .expect("La posición secuencial debe ser positiva");
                lista.push(evento);
            }

            Ok(())
        }

        // Devuelve los eventos registrados para un usuario en orden de llegada
        fn eventos_de(&self, usuario_id: Uuid) -> &[EventoCambio] {
            self.eventos_por_usuario
                .get(&usuario_id)
                .map_or(&[], Vec::as_slice)
        }

        // Cuenta el total de eventos almacenados entre todos los usuarios.
        fn total_eventos(&self) -> usize {
            self.eventos_por_usuario.values().map(Vec::len).sum()
        }
    }

    // Comprueba que cada participante reciba exactamente un evento con datos correctos
    #[test]
    fn cada_participante_recibe_un_evento_con_datos_y_redondeo_correctos() {
        let ana = id_fijo(1);
        let beto = id_fijo(2);
        let carla = id_fijo(3);
        let miembros = HashSet::from([ana, beto, carla]);
        let fecha = NaiveDate::from_ymd_opt(2026, 11, 20).expect("Fecha válida");

        let gasto = GastoPrueba {
            grupo_id: id_fijo(10),
            gasto_id: id_fijo(100),
            concepto: "Hotel",
            fecha,
            moneda: "USD",
        };
        // Simula el reparto de 100.00 USD del hotel con céntimo residual.
        let partes = [
            PartePrueba {
                usuario_id: ana,
                monto_parte: "33.34",
                recibio_residuo: true,
            },
            PartePrueba {
                usuario_id: beto,
                monto_parte: "33.33",
                recibio_residuo: false,
            },
            PartePrueba {
                usuario_id: carla,
                monto_parte: "33.33",
                recibio_residuo: false,
            },
        ];

        let mut registro = RegistroCambiosPrueba::nuevo();
        let resultado = registro.registrar_gasto(ana, &miembros, &gasto, &partes);
        assert!(resultado.is_ok());
        assert_eq!(registro.total_eventos(), 3);

        for (usuario_id, monto_esperado, residuo_esperado) in [
            (ana, "33.34", true),
            (beto, "33.33", false),
            (carla, "33.33", false),
        ] {
            let eventos = registro.eventos_de(usuario_id);
            assert_eq!(eventos.len(), 1);
            let evento = &eventos[0];
            assert_eq!(evento.tipo(), TIPO_EVENTO_GASTO_GRUPO);
            assert_eq!(evento.posicion(), 1);
            assert_eq!(evento.datos().grupo_id, gasto.grupo_id);
            assert_eq!(evento.datos().gasto_id, gasto.gasto_id);
            assert_eq!(evento.datos().concepto, "Hotel");
            assert_eq!(evento.datos().fecha, fecha);
            assert_eq!(evento.datos().moneda, "USD");
            assert_eq!(evento.datos().monto_parte, monto_esperado);
            assert_eq!(evento.datos().recibio_residuo, residuo_esperado);
        }
    }

    // Verifica que un miembro excluido del gasto no reciba ningún evento
    #[test]
    fn miembro_excluido_de_la_division_no_recibe_eventos() {
        let ana = id_fijo(1);
        let beto = id_fijo(2);
        let carla = id_fijo(3);
        let miembros = HashSet::from([ana, beto, carla]);
        let fecha = NaiveDate::from_ymd_opt(2026, 11, 20).expect("Fecha válida");

        let gasto = GastoPrueba {
            grupo_id: id_fijo(10),
            gasto_id: id_fijo(101),
            concepto: "Taxi",
            fecha,
            moneda: "USD",
        };
        let partes = [
            PartePrueba {
                usuario_id: ana,
                monto_parte: "10.00",
                recibio_residuo: false,
            },
            PartePrueba {
                usuario_id: beto,
                monto_parte: "10.00",
                recibio_residuo: false,
            },
        ];

        let mut registro = RegistroCambiosPrueba::nuevo();
        let resultado = registro.registrar_gasto(ana, &miembros, &gasto, &partes);
        assert!(resultado.is_ok());
        assert_eq!(registro.eventos_de(ana).len(), 1);
        assert_eq!(registro.eventos_de(beto).len(), 1);
        assert!(registro.eventos_de(carla).is_empty());
        assert_eq!(registro.total_eventos(), 2);
    }

    // Comprueba INV-17 rechazando ajenos y devolviendo 404 sin generar eventos
    #[test]
    fn inv_17_rechaza_no_miembros_y_mantiene_404_sin_generar_eventos() {
        let ana = id_fijo(1);
        let beto = id_fijo(2);
        let externo = id_fijo(99);
        let miembros = HashSet::from([ana, beto]);
        let grupo_vacio = HashSet::new();
        let fecha = NaiveDate::from_ymd_opt(2026, 11, 20).expect("Fecha válida");

        let gasto = GastoPrueba {
            grupo_id: id_fijo(10),
            gasto_id: id_fijo(102),
            concepto: "Cena",
            fecha,
            moneda: "USD",
        };
        let mut registro = RegistroCambiosPrueba::nuevo();

        // Caso participante externo falla y evita emisiones parciales
        let partes_con_externo = [
            PartePrueba {
                usuario_id: ana,
                monto_parte: "15.00",
                recibio_residuo: false,
            },
            PartePrueba {
                usuario_id: externo,
                monto_parte: "15.00",
                recibio_residuo: false,
            },
        ];
        let error_externo = registro
            .registrar_gasto(ana, &miembros, &gasto, &partes_con_externo)
            .expect_err("Un participante ajeno debe rechazarse");
        assert_eq!(
            error_externo,
            ErrorDestinatarios::ParticipanteNoMiembro(externo)
        );
        assert_eq!(error_externo.codigo_http(), StatusCode::BAD_REQUEST);
        assert_eq!(registro.total_eventos(), 0);

        // Caso solicitante sin membresía recibe 404 ocultando acceso
        let partes_validas = [PartePrueba {
            usuario_id: ana,
            monto_parte: "30.00",
            recibio_residuo: false,
        }];
        let error_sin_membresia = registro
            .registrar_gasto(externo, &miembros, &gasto, &partes_validas)
            .expect_err("Un solicitante externo debe recibir 404");
        assert_eq!(error_sin_membresia, ErrorDestinatarios::GrupoNoAccesible);
        assert_eq!(error_sin_membresia.codigo_http(), StatusCode::NOT_FOUND);
        assert!(matches!(
            error_sin_membresia.a_error_gasto(),
            ErrorGasto::SinAcceso
        ));
        assert_eq!(registro.total_eventos(), 0);

        // Caso grupo inexistente responde con el mismo 404
        let error_inexistente = registro
            .registrar_gasto(ana, &grupo_vacio, &gasto, &partes_validas)
            .expect_err("Un grupo inexistente debe recibir 404");
        assert_eq!(error_inexistente, ErrorDestinatarios::GrupoNoAccesible);
        assert_eq!(error_inexistente.codigo_http(), StatusCode::NOT_FOUND);
        assert!(matches!(
            error_inexistente.a_error_gasto(),
            ErrorGasto::SinAcceso
        ));
        assert_eq!(registro.total_eventos(), 0);
    }

    // Valida identificadores únicos, orden creciente por usuario y rechazo de posición inválida
    #[test]
    fn eventos_conservan_id_unico_y_orden_creciente_por_usuario() {
        let ana = id_fijo(1);
        let beto = id_fijo(2);
        let carla = id_fijo(3);
        let miembros = HashSet::from([ana, beto, carla]);
        let grupo_id = id_fijo(10);
        let fecha = NaiveDate::from_ymd_opt(2026, 11, 21).expect("Fecha válida");
        let mut registro = RegistroCambiosPrueba::nuevo();

        let hotel = GastoPrueba {
            grupo_id,
            gasto_id: id_fijo(201),
            concepto: "Hotel",
            fecha,
            moneda: "USD",
        };
        let taxi = GastoPrueba {
            grupo_id,
            gasto_id: id_fijo(202),
            concepto: "Taxi",
            fecha,
            moneda: "USD",
        };
        let cena = GastoPrueba {
            grupo_id,
            gasto_id: id_fijo(203),
            concepto: "Cena",
            fecha,
            moneda: "USD",
        };

        registro
            .registrar_gasto(
                ana,
                &miembros,
                &hotel,
                &[
                    PartePrueba {
                        usuario_id: ana,
                        monto_parte: "33.34",
                        recibio_residuo: true,
                    },
                    PartePrueba {
                        usuario_id: beto,
                        monto_parte: "33.33",
                        recibio_residuo: false,
                    },
                    PartePrueba {
                        usuario_id: carla,
                        monto_parte: "33.33",
                        recibio_residuo: false,
                    },
                ],
            )
            .expect("El primer gasto debe registrarse");

        registro
            .registrar_gasto(
                beto,
                &miembros,
                &taxi,
                &[
                    PartePrueba {
                        usuario_id: ana,
                        monto_parte: "10.00",
                        recibio_residuo: false,
                    },
                    PartePrueba {
                        usuario_id: beto,
                        monto_parte: "10.00",
                        recibio_residuo: false,
                    },
                ],
            )
            .expect("El segundo gasto debe registrarse");

        registro
            .registrar_gasto(
                carla,
                &miembros,
                &cena,
                &[
                    PartePrueba {
                        usuario_id: ana,
                        monto_parte: "30.00",
                        recibio_residuo: false,
                    },
                    PartePrueba {
                        usuario_id: beto,
                        monto_parte: "15.00",
                        recibio_residuo: false,
                    },
                    PartePrueba {
                        usuario_id: carla,
                        monto_parte: "15.00",
                        recibio_residuo: false,
                    },
                ],
            )
            .expect("El tercer gasto debe registrarse");

        // Comprueba crecimiento estricto del cursor y unicidad global de identificadores.
        let mut ids_unicos = HashSet::new();
        for (usuario_id, posiciones_esperadas) in [
            (ana, &[1, 2, 3][..]),
            (beto, &[1, 2, 3][..]),
            (carla, &[1, 2][..]),
        ] {
            let eventos = registro.eventos_de(usuario_id);
            let posiciones: Vec<i64> = eventos.iter().map(EventoCambio::posicion).collect();
            assert_eq!(posiciones, posiciones_esperadas);

            for par in eventos.windows(2) {
                assert!(par[0].posicion() < par[1].posicion());
            }
            for evento in eventos {
                assert!(ids_unicos.insert(evento.id()));
            }
        }
        assert_eq!(ids_unicos.len(), 8);

        // Verifica la invariante de posición estrictamente positiva en el constructor.
        let datos_invalidos = DatosParteGasto {
            grupo_id,
            gasto_id: id_fijo(204),
            concepto: String::from("Prueba"),
            fecha,
            moneda: String::from("USD"),
            monto_parte: String::from("1.00"),
            recibio_residuo: false,
        };
        assert!(EventoCambio::nuevo_gasto_grupo(id_fijo(9000), 0, datos_invalidos).is_err());
    }

    // Verifica que omitir Idempotency-Key sea válido y devuelve None
    #[test]
    fn idempotency_key_ausente_devuelve_none() {
        let headers = HeaderMap::new();
        let resultado = extraer_clave_idempotencia(&headers);
        assert_eq!(resultado, Ok(None));
    }

    // Comrpueba que una clave ASCII válida se extraiga sin alteraciones
    #[test]
    fn idempotency_key_valida_se_extrae_correctamente() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static(ENCABEZADO_IDEMPOTENCY_KEY),
            HeaderValue::from_static("gasto-offline-550e8400-e29b-41d4"),
        );

        let clave = extraer_clave_idempotencia(&headers)
            .expect("La clave debe aceptarse")
            .expect("Debe existir una clave");
        assert_eq!(clave.as_str(), "gasto-offline-550e8400-e29b-41d4");
    }

    // Rechaza claves vacías, extensas, con espacios, inválidas o repetidas con código 400
    #[test]
    fn idempotency_key_invalida_responde_400() {
        let clave_larga = "a".repeat(LONGITUD_MAXIMA_CLAVE_IDEMPOTENCIA + 1);
        let casos_invalidos = ["", "   ", "clave con espacio", clave_larga.as_str()];

        for valor in casos_invalidos {
            let mut headers = HeaderMap::new();
            headers.insert(
                HeaderName::from_static(ENCABEZADO_IDEMPOTENCY_KEY),
                HeaderValue::from_str(valor).expect("Valor de prueba construible"),
            );

            let error = extraer_clave_idempotencia(&headers)
                .expect_err("El encabezado inválido debe rechazarse");
            let (codigo, _) = error.a_respuesta_http();
            assert_eq!(codigo, StatusCode::BAD_REQUEST);
        }

        // Verifica rechazo cuando el encazbezado contiene bytes no ASCII
        let mut headers_no_ascii = HeaderMap::new();
        headers_no_ascii.insert(
            HeaderName::from_static(ENCABEZADO_IDEMPOTENCY_KEY),
            HeaderValue::from_bytes(&[0xFF]).expect("Bytes válidos para prueba"),
        );
        let error_ascii = extraer_clave_idempotencia(&headers_no_ascii)
            .expect_err("Bytes no ASCII deben rechazarse");
        assert_eq!(error_ascii.a_respuesta_http().0, StatusCode::BAD_REQUEST);

        // Verifica rechazo cuando se envían múltiples encabezados Idempotency-Key
        let mut headers_duplicados = HeaderMap::new();
        headers_duplicados.append(
            HeaderName::from_static(ENCABEZADO_IDEMPOTENCY_KEY),
            HeaderValue::from_static("clave-1"),
        );
        headers_duplicados.append(
            HeaderName::from_static(ENCABEZADO_IDEMPOTENCY_KEY),
            HeaderValue::from_static("clave-2"),
        );
        let error_duplicado = extraer_clave_idempotencia(&headers_duplicados)
            .expect_err("Encabezados repetidos deben rechazarse");
        assert_eq!(
            error_duplicado.a_respuesta_http().0,
            StatusCode::BAD_REQUEST
        );
    }

    // Verifica respuestas 401 y 404 para sesión y acceso al grupo
    #[test]
    fn precondiciones_mantienen_401_y_404_uniformes() {
        let (codigo_401, cuerpo_401) = ErrorPrecondicionGasto::NoAutorizado.a_respuesta_http();
        assert_eq!(codigo_401, StatusCode::UNAUTHORIZED);
        assert_eq!(
            cuerpo_401.0,
            json!({ "mensaje": "Sesión inválida o vencida." })
        );

        let error_acceso = validar_acceso_grupo(false)
            .expect_err("La falta de membresía o grupo debe fallar con 404");
        let (codigo_404, cuerpo_404) = error_acceso.a_respuesta_http();
        assert_eq!(codigo_404, StatusCode::NOT_FOUND);
        assert_eq!(cuerpo_404.0, json!({ "mensaje": "Grupo no encontrado." }));
        assert_eq!(validar_acceso_grupo(true), Ok(()));
    }

    use crate::gastos_db::{GastoGuardado, ParteGuardada};

    // Comprueba el mapeo HTTP para creación 201, repetición 200 y conflicto 409
    #[test]
    fn mapeo_resultado_gasto_responde_201_200_y_409() {
        let gasto_id = id_fijo(500);
        let ana = id_fijo(1);
        let gasto = GastoGuardado {
            id: gasto_id,
            pagado_por: ana,
            concepto: String::from("Cena"),
            monto: String::from("60.00"),
            fecha: String::from("2026-11-22"),
            regla: Some(String::from("iguales")),
            partes: vec![ParteGuardada {
                usuario_id: ana,
                nombre_usuario: String::from("ana"),
                monto: String::from("60.00"),
                recibio_residuo: Some(false),
            }],
        };

        let (codigo_201, cuerpo_201) =
            mapear_resultado_gasto_http(ResultadoRegistroGasto::Creado(gasto));
        assert_eq!(codigo_201, StatusCode::CREATED);
        assert_eq!(cuerpo_201.0["id"], gasto_id.to_string());
        assert_eq!(cuerpo_201.0["monto"], "60.00");

        let gasto_repetido = GastoGuardado {
            id: gasto_id,
            pagado_por: ana,
            concepto: String::from("Cena"),
            monto: String::from("60.00"),
            fecha: String::from("2026-11-22"),
            regla: Some(String::from("iguales")),
            partes: vec![],
        };
        let (codigo_200, cuerpo_200) =
            mapear_resultado_gasto_http(ResultadoRegistroGasto::Existente(gasto_repetido));
        assert_eq!(codigo_200, StatusCode::OK);
        assert_eq!(cuerpo_200.0["id"], gasto_id.to_string());

        let (codigo_409, cuerpo_409) =
            mapear_resultado_gasto_http(ResultadoRegistroGasto::ConflictoIdempotencia);
        assert_eq!(codigo_409, StatusCode::CONFLICT);
        assert_eq!(
            cuerpo_409.0,
            json!({ "mensaje": MENSAJE_CONFLICTO_IDEMPOTENCIA })
        );
    }

    // Comprueba el mapeo de errores 404 uniforme y 400 de validación
    #[test]
    fn mapeo_error_gasto_mantiene_404_uniforme_y_400_entrega_6() {
        let (codigo_404, cuerpo_404) = mapear_error_gasto_http(ErrorGasto::SinAcceso);
        assert_eq!(codigo_404, StatusCode::NOT_FOUND);
        assert_eq!(cuerpo_404.0, json!({ "mensaje": "Grupo no encontrado." }));

        let (codigo_400, cuerpo_400) =
            mapear_error_gasto_http(ErrorGasto::Invalido(String::from("faltan 5.00")));
        assert_eq!(codigo_400, StatusCode::BAD_REQUEST);
        assert_eq!(cuerpo_400.0, json!({ "mensaje": "faltan 5.00" }));
    }

    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use chrono::{Duration, Utc};
    use jsonwebtoken::{EncodingKey, Header, encode};
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;

    // Carga útil JWT usada para firmar tokens en pruebas HTTP
    #[derive(Serialize)]
    struct ClaimsPrueba {
        sub: Uuid,
        exp: usize,
    }

    // Firma un token JWT con vigencia relativa en minutos para pruebas
    fn firmar_token_prueba(usuario_id: Uuid, secreto: &str, minutos: i64) -> String {
        let exp = usize::try_from((Utc::now() + Duration::minutes(minutos)).timestamp())
            .expect("La marca de tiempo debe ser positiva");
        encode(
            &Header::default(),
            &ClaimsPrueba {
                sub: usuario_id,
                exp,
            },
            &EncodingKey::from_secret(secreto.as_bytes()),
        )
        .expect("El token de prueba debe firmarse")
    }

    // Construye una petición POST /grupos/{id}/gastos con token y clave opcionales
    fn peticion_gasto_http(
        grupo_id: Uuid,
        token: Option<&str>,
        clave_idempotencia: Option<&str>,
        cuerpo: &str,
    ) -> Request<Body> {
        let mut builder = Request::builder()
            .method("POST")
            .uri(format!("/grupos/{grupo_id}/gastos"))
            .header("content-type", "application/json");

        if let Some(t) = token {
            builder = builder.header("authorization", format!("Bearer {t}"));
        }
        if let Some(clave) = clave_idempotencia {
            builder = builder.header(ENCABEZADO_IDEMPOTENCY_KEY, clave);
        }

        builder
            .body(Body::from(cuerpo.to_owned()))
            .expect("La petición HTTP de prueba debe construirse")
    }

    // Comprueba que POST /grupos/{id}/gastos rechace credenciales inválidas con 401
    #[tokio::test]
    async fn post_gastos_rechaza_credenciales_ausentes_invalidas_y_vencidas_con_401() {
        let secreto = "secreto de prueba con al menos 32 bytes";
        // Usa conexión diferida porque el rechazo 401 ocurre antes de consultar PostgreSQL
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://test:test@127.0.0.1/test")
            .expect("El pool diferido debe configurarse");
        let app = construir_app(auth::State::new(pool, secreto.into()));
        let grupo_id = id_fijo(10);
        let ana = id_fijo(1);
        let token_vencido = firmar_token_prueba(ana, secreto, -5);
        let token_otro_secreto =
            firmar_token_prueba(ana, "otro secreto distinto con mas de 32 bytes", 5);

        let casos = [
            None,
            Some("token-malformado"),
            Some(token_otro_secreto.as_str()),
            Some(token_vencido.as_str()),
        ];

        for token in casos {
            let respuesta = app
                .clone()
                .oneshot(peticion_gasto_http(
                    grupo_id,
                    token,
                    Some("clave-prueba-401"),
                    "{}",
                ))
                .await
                .expect("La petición de prueba debe ejecutarse");

            assert_eq!(respuesta.status(), StatusCode::UNAUTHORIZED);
            let bytes = to_bytes(respuesta.into_body(), 65536)
                .await
                .expect("El cuerpo de respuesta debe leerse");
            let json_resp: Value =
                serde_json::from_slice(&bytes).expect("La respuesta debe ser JSON válido");
            assert_eq!(
                json_resp,
                json!({ "mensaje": "Sesión inválida o vencida." })
            );
        }
    }

    // Valida 201, 200, 409, 404 uniforme, 400 y 409 en POST /grupos/{id}/gastos
    #[tokio::test]
    #[ignore = "requiere PostgreSQL de pruebas"]
    async fn post_gastos_idempotencia_http_201_200_409_404_400_y_401() {
        let url = std::env::var("E5_TEST_DATABASE_URL").expect("Configura E5_TEST_DATABASE_URL");
        let pool = db::conectar_y_migrar(&url)
            .await
            .expect("La base de pruebas debe conectarse");
        let secreto = "secreto de prueba con al menos 32 bytes";
        let app = construir_app(auth::State::new(pool.clone(), secreto.into()));

        let ana = Uuid::new_v4();
        let beto = Uuid::new_v4();
        let ajeno = Uuid::new_v4();
        let grupo = Uuid::new_v4();
        let grupo_inexistente = Uuid::new_v4();
        let sufijo = Uuid::new_v4().simple().to_string();

        for (id, nombre) in [(ana, "ana"), (beto, "beto"), (ajeno, "ajeno")] {
            let usuario = format!("{nombre}_{}", &sufijo[..8]);
            sqlx::query("INSERT INTO usuarios(id,nombre_usuario,nombre_visible,correo,password_hash) VALUES($1,$2,$3,$4,'hash')")
                .bind(id)
                .bind(&usuario)
                .bind(nombre)
                .bind(format!("{usuario}@example.com"))
                .execute(&pool)
                .await
                .expect("El usuario de prueba debe insertarse");
        }
        sqlx::query("INSERT INTO grupos(id,nombre,moneda,creador_id) VALUES($1,'Viaje','USD',$2)")
            .bind(grupo)
            .bind(ana)
            .execute(&pool)
            .await
            .expect("El grupo de prueba debe insertarse");
        for id in [ana, beto] {
            sqlx::query("INSERT INTO grupo_miembros(grupo_id,usuario_id,estado_invitacion) VALUES($1,$2,'aceptada')")
                .bind(grupo)
                .bind(id)
                .execute(&pool)
                .await
                .expect("La membresía de prueba debe insertarse");
        }

        let token_ana = firmar_token_prueba(ana, secreto, 10);
        let token_ajeno = firmar_token_prueba(ajeno, secreto, 10);
        let token_vencido = firmar_token_prueba(ana, secreto, -5);

        let cuerpo_hotel = json!({
            "concepto": "Hotel",
            "monto": "100.00",
            "fecha": "2026-11-22",
            "pagado_por": ana,
            "regla": "iguales",
            "participantes": [
                { "usuario_id": ana },
                { "usuario_id": beto }
            ]
        })
        .to_string();

        // Verifica rechazo 401 sin token y con token vencido
        for token in [None, Some(token_vencido.as_str())] {
            let resp = app
                .clone()
                .oneshot(peticion_gasto_http(
                    grupo,
                    token,
                    Some("clave-401"),
                    &cuerpo_hotel,
                ))
                .await
                .expect("La petición 401 debe ejecutarse");
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        }

        // Verifica 404 idéntico para grupo inexistente y usuario no miembro
        let mut cuerpos_404 = Vec::new();
        for (grupo_obj, token) in [
            (grupo, token_ajeno.as_str()),
            (grupo_inexistente, token_ana.as_str()),
        ] {
            let resp = app
                .clone()
                .oneshot(peticion_gasto_http(
                    grupo_obj,
                    Some(token),
                    Some("clave-404"),
                    &cuerpo_hotel,
                ))
                .await
                .expect("La petición 404 debe ejecutarse");
            assert_eq!(resp.status(), StatusCode::NOT_FOUND);
            let bytes = to_bytes(resp.into_body(), 65536).await.unwrap();
            cuerpos_404.push(serde_json::from_slice::<Value>(&bytes).unwrap());
        }
        assert_eq!(cuerpos_404[0], json!({ "mensaje": "Grupo no encontrado." }));
        assert_eq!(cuerpos_404[0], cuerpos_404[1]);

        let clave_hotel = format!("clave-hotel-{}", Uuid::new_v4());

        // Verifica rechazo 400 ante encabezado Idempotency-Key mal formado
        let resp_clave_mala = app
            .clone()
            .oneshot(peticion_gasto_http(
                grupo,
                Some(&token_ana),
                Some("clave con espacio"),
                &cuerpo_hotel,
            ))
            .await
            .unwrap();
        assert_eq!(resp_clave_mala.status(), StatusCode::BAD_REQUEST);

        // Verifica rechazo 400 ante porcentajes inválidos sin bloquear la clave
        let cuerpo_invalido = json!({
            "concepto": "Hotel",
            "monto": "100.00",
            "fecha": "2026-11-22",
            "pagado_por": ana,
            "regla": "porcentajes",
            "participantes": [
                { "usuario_id": ana, "valor": "50" },
                { "usuario_id": beto, "valor": "45" }
            ]
        })
        .to_string();
        let resp_400 = app
            .clone()
            .oneshot(peticion_gasto_http(
                grupo,
                Some(&token_ana),
                Some(&clave_hotel),
                &cuerpo_invalido,
            ))
            .await
            .unwrap();
        assert_eq!(resp_400.status(), StatusCode::BAD_REQUEST);
        let datos_400: Value =
            serde_json::from_slice(&to_bytes(resp_400.into_body(), 65536).await.unwrap()).unwrap();
        assert!(
            datos_400["mensaje"]
                .as_str()
                .unwrap()
                .contains("faltan 5.00")
        );

        // Verifica creación inicial con 201 Created usando la misma clave
        let resp_201 = app
            .clone()
            .oneshot(peticion_gasto_http(
                grupo,
                Some(&token_ana),
                Some(&clave_hotel),
                &cuerpo_hotel,
            ))
            .await
            .unwrap();
        assert_eq!(resp_201.status(), StatusCode::CREATED);
        let gasto_creado: Value =
            serde_json::from_slice(&to_bytes(resp_201.into_body(), 65536).await.unwrap()).unwrap();
        assert_eq!(gasto_creado["monto"], "100.00");

        // Verifica 200 OK en reintento equivalente devolviendo el mismo gasto
        let cuerpo_equivalente = json!({
            "participantes": [
                { "usuario_id": beto },
                { "usuario_id": ana }
            ],
            "regla": "iguales",
            "pagado_por": ana,
            "fecha": "2026-11-22",
            "monto": "100",
            "concepto": "  Hotel  "
        })
        .to_string();
        let resp_200 = app
            .clone()
            .oneshot(peticion_gasto_http(
                grupo,
                Some(&token_ana),
                Some(&clave_hotel),
                &cuerpo_equivalente,
            ))
            .await
            .unwrap();
        assert_eq!(resp_200.status(), StatusCode::OK);
        let gasto_repetido: Value =
            serde_json::from_slice(&to_bytes(resp_200.into_body(), 65536).await.unwrap()).unwrap();
        assert_eq!(gasto_repetido["id"], gasto_creado["id"]);
        assert_eq!(gasto_repetido["monto"], "100.00");

        // Verifica 409 Conflict al reutilizar la clave con datos distintos
        let cuerpo_distinto = json!({
            "concepto": "Cena",
            "monto": "60.00",
            "fecha": "2026-11-22",
            "pagado_por": ana,
            "regla": "iguales",
            "participantes": [
                { "usuario_id": ana },
                { "usuario_id": beto }
            ]
        })
        .to_string();
        let resp_409 = app
            .clone()
            .oneshot(peticion_gasto_http(
                grupo,
                Some(&token_ana),
                Some(&clave_hotel),
                &cuerpo_distinto,
            ))
            .await
            .unwrap();
        assert_eq!(resp_409.status(), StatusCode::CONFLICT);
        let datos_409: Value =
            serde_json::from_slice(&to_bytes(resp_409.into_body(), 65536).await.unwrap()).unwrap();
        assert_eq!(
            datos_409,
            json!({ "mensaje": MENSAJE_CONFLICTO_IDEMPOTENCIA })
        );

        // Confirma que no se duplicaron gastos ni eventos tras los reintentos
        let total_gastos: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM gastos WHERE grupo_id = $1")
                .bind(grupo)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(total_gastos, 1);

        let total_eventos: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM eventos_usuario WHERE grupo_id = $1")
                .bind(grupo)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(total_eventos, 2);

        // Limpia en orden las tablas dependientes para dejar la base intacta
        sqlx::query("DELETE FROM claves_gastos WHERE usuario_id = $1")
            .bind(ana)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM eventos_usuario WHERE grupo_id = $1")
            .bind(grupo)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM grupos WHERE id = $1")
            .bind(grupo)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM usuarios WHERE id = ANY($1)")
            .bind(&[ana, beto, ajeno][..])
            .execute(&pool)
            .await
            .unwrap();
    }

    // Verifica que un cursor omitido, cero o positivo sea aceptado
    #[test]
    fn validar_cursor_desde_acepta_omitido_cero_y_positivos() {
        assert_eq!(validar_cursor_desde(None), Ok(0));
        assert_eq!(validar_cursor_desde(Some("0")), Ok(0));
        assert_eq!(validar_cursor_desde(Some("42")), Ok(42));
        assert_eq!(
            validar_cursor_desde(Some(&i64::MAX.to_string())),
            Ok(i64::MAX)
        );
    }

    // Comprueba rechazo 400 ante cursores negativos, vacíos, textuales o fuera de rango
    #[test]
    fn validar_cursor_desde_rechaza_valores_invalidos_con_400() {
        let casos_invalidos = [
            "",
            " ",
            "-1",
            "+1",
            "abc",
            "12.5",
            "1 2",
            "99999999999999999999999",
        ];

        for valor in casos_invalidos {
            let error = validar_cursor_desde(Some(valor))
                .expect_err("Un cursor mal formado debe rechazarse");
            let (codigo, cuerpo) = error.a_respuesta_http();
            assert_eq!(codigo, StatusCode::BAD_REQUEST);
            assert_eq!(cuerpo.0["codigo"], "cursor_invalido");
            assert!(cuerpo.0["mensaje"].is_string());
        }
    }

    // Verifica el contrato JSON de RespuestaCambios y la ausencia de datos personales
    #[test]
    fn respuesta_cambios_serializa_campos_openapi_sin_libro_personal() {
        let evento_db = EventoUsuario {
            id: id_fijo(700),
            posicion: 1,
            tipo: String::from(TIPO_EVENTO_GASTO_GRUPO),
            gasto_id: id_fijo(701),
            grupo_id: id_fijo(702),
            concepto: String::from("Hotel"),
            fecha: NaiveDate::from_ymd_opt(2026, 11, 22).expect("Fecha válida"),
            moneda: String::from("USD"),
            monto_parte: String::from("33.33"),
            recibio_residuo: false,
        };

        let evento_http =
            EventoCambioRespuesta::desde_evento_db(evento_db, String::from("Viaje a Mérida"))
                .expect("El evento válido debe convertirse");
        let respuesta = RespuestaCambios::nueva(vec![evento_http], 1, false)
            .expect("La respuesta paginada debe construirse");

        let json_resp =
            serde_json::to_value(&respuesta).expect("La respuesta debe serializarse a JSON");
        assert_eq!(json_resp["nuevo_cursor"], 1);
        assert_eq!(json_resp["hay_mas"], false);

        let objeto_evento = json_resp["eventos"][0]
            .as_object()
            .expect("Cada evento debe ser un objeto JSON");
        assert_eq!(objeto_evento.len(), 11);
        assert_eq!(objeto_evento["cursor"], 1);
        assert_eq!(objeto_evento["grupo_nombre"], "Viaje a Mérida");
        assert_eq!(objeto_evento["monto_parte"], "33.33");
        assert_eq!(objeto_evento["incluye_centimos_redondeo"], false);

        // Confirma INV-16 verificando que no existan campos del libro personal
        for campo_privado in ["cuenta_id", "cuenta", "saldo", "movimientos", "usuario_id"] {
            assert!(!objeto_evento.contains_key(campo_privado));
        }
    }

    // Rechaza eventos con posición no positiva y lotes mayores a cien elementos
    #[test]
    fn respuesta_cambios_rechaza_cursor_invalido_o_lote_excedido() {
        let evento_invalido = EventoUsuario {
            id: id_fijo(710),
            posicion: 0,
            tipo: String::from(TIPO_EVENTO_GASTO_GRUPO),
            gasto_id: id_fijo(711),
            grupo_id: id_fijo(712),
            concepto: String::from("Cena"),
            fecha: NaiveDate::from_ymd_opt(2026, 11, 22).expect("Fecha válida"),
            moneda: String::from("USD"),
            monto_parte: String::from("15.00"),
            recibio_residuo: false,
        };
        assert!(
            EventoCambioRespuesta::desde_evento_db(evento_invalido, String::from("Viaje")).is_err()
        );

        assert!(RespuestaCambios::nueva(vec![], -1, false).is_err());

        let evento_base = EventoCambioRespuesta {
            id: id_fijo(720),
            cursor: 1,
            tipo: String::from(TIPO_EVENTO_GASTO_GRUPO),
            grupo_id: id_fijo(721),
            grupo_nombre: String::from("Viaje"),
            gasto_id: id_fijo(722),
            concepto: String::from("Café"),
            fecha: NaiveDate::from_ymd_opt(2026, 11, 22).expect("Fecha válida"),
            moneda: String::from("USD"),
            monto_parte: String::from("2.00"),
            incluye_centimos_redondeo: false,
        };
        let lote_excedido = vec![evento_base; LIMITE_EVENTOS_CAMBIOS + 1];
        assert!(RespuestaCambios::nueva(lote_excedido, 101, true).is_err());
    }

    // Construye una petición GET a /cambios con token Bearer opcional
    fn peticion_cambios_http(ruta: &str, token: Option<&str>) -> Request<Body> {
        let mut builder = Request::builder().method("GET").uri(ruta);
        if let Some(t) = token {
            builder = builder.header("authorization", format!("Bearer {t}"));
        }
        builder
            .body(Body::empty())
            .expect("La petición GET de prueba debe construirse")
    }

    // Comprueba que GET /cambios rechace tokens ausentes, inválidos o vencidos con 401
    #[tokio::test]
    async fn get_cambios_rechaza_credenciales_ausentes_invalidas_y_vencidas_con_401() {
        let secreto = "secreto de prueba con al menos 32 bytes";
        // Usa conexión diferida porque el rechazo 401 ocurre antes de consultar PostgreSQL
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://test:test@127.0.0.1/test")
            .expect("El pool diferido debe configurarse");
        let app = construir_app(auth::State::new(pool, secreto.into()));
        let ana = id_fijo(1);
        let token_vencido = firmar_token_prueba(ana, secreto, -5);
        let token_otro_secreto =
            firmar_token_prueba(ana, "otro secreto distinto con mas de 32 bytes", 5);

        let casos = [
            None,
            Some("token-malformado"),
            Some(token_otro_secreto.as_str()),
            Some(token_vencido.as_str()),
        ];

        for token in casos {
            let respuesta = app
                .clone()
                .oneshot(peticion_cambios_http("/cambios?desde=0", token))
                .await
                .expect("La petición de prueba debe ejecutarse");

            assert_eq!(respuesta.status(), StatusCode::UNAUTHORIZED);
            let bytes = to_bytes(respuesta.into_body(), 65536)
                .await
                .expect("El cuerpo de respuesta debe leerse");
            let json_resp: Value =
                serde_json::from_slice(&bytes).expect("La respuesta debe ser JSON válido");
            assert_eq!(
                json_resp,
                json!({
                    "codigo": "no_autorizado",
                    "mensaje": "Sesión inválida o vencida."
                })
            );
        }
    }

    // Comprueba que GET /cambios rechace cursores inválidos con código 400
    #[tokio::test]
    async fn get_cambios_rechaza_cursor_invalido_con_400() {
        let secreto = "secreto de prueba con al menos 32 bytes";
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://test:test@127.0.0.1/test")
            .expect("El pool diferido debe configurarse");
        let app = construir_app(auth::State::new(pool, secreto.into()));
        let token_ana = firmar_token_prueba(id_fijo(1), secreto, 10);

        let rutas_invalidas = [
            "/cambios?desde=",
            "/cambios?desde=-1",
            "/cambios?desde=%2B1",
            "/cambios?desde=abc",
            "/cambios?desde=12.5",
            "/cambios?desde=99999999999999999999999",
        ];

        for ruta in rutas_invalidas {
            let respuesta = app
                .clone()
                .oneshot(peticion_cambios_http(ruta, Some(&token_ana)))
                .await
                .expect("La petición de prueba debe ejecutarse");

            assert_eq!(respuesta.status(), StatusCode::BAD_REQUEST);
            let bytes = to_bytes(respuesta.into_body(), 65536)
                .await
                .expect("El cuerpo de respuesta debe leerse");
            let json_resp: Value =
                serde_json::from_slice(&bytes).expect("La respuesta debe ser JSON válido");
            assert_eq!(json_resp["codigo"], "cursor_invalido");
            assert!(json_resp["mensaje"].is_string());
        }
    }

    // Verifica paginación en lotes de cien, consulta vacía y reinicio del cursor
    #[test]
    fn construir_respuesta_cambios_pagina_en_lotes_y_maneja_vacio_o_reinicio() {
        let fecha = NaiveDate::from_ymd_opt(2026, 11, 22).expect("Fecha válida");
        let grupo_id = id_fijo(800);
        let todos: Vec<sincronizacion_db::EventoCambioDb> = (1..=105_i64)
            .map(|posicion| sincronizacion_db::EventoCambioDb {
                evento: EventoUsuario {
                    id: id_fijo(8000 + u128::try_from(posicion).expect("Posición positiva")),
                    posicion,
                    tipo: String::from(TIPO_EVENTO_GASTO_GRUPO),
                    gasto_id: id_fijo(9000 + u128::try_from(posicion).expect("Posición positiva")),
                    grupo_id,
                    concepto: format!("Gasto {posicion}"),
                    fecha,
                    moneda: String::from("USD"),
                    monto_parte: String::from("5.00"),
                    recibio_residuo: false,
                },
                grupo_nombre: String::from("Viaje a Mérida"),
            })
            .collect();

        // Simula el primer lote cuando existen más de cien eventos disponibles
        let lote_1: Vec<_> = todos.iter().take(101).cloned().collect();
        let resp_1 =
            construir_respuesta_cambios(sincronizacion_db::armar_pagina_cambios(lote_1, 0))
                .expect("La primera página debe construirse");
        assert_eq!(resp_1.eventos.len(), LIMITE_EVENTOS_CAMBIOS);
        assert_eq!(resp_1.eventos.first().unwrap().cursor, 1);
        assert_eq!(resp_1.eventos.last().unwrap().cursor, 100);
        assert_eq!(resp_1.nuevo_cursor, 100);
        assert!(resp_1.hay_mas);

        // Simula la segunda página con los cinco eventos restantes
        let lote_2: Vec<_> = todos
            .iter()
            .filter(|e| e.evento.posicion > resp_1.nuevo_cursor)
            .take(101)
            .cloned()
            .collect();
        let resp_2 = construir_respuesta_cambios(sincronizacion_db::armar_pagina_cambios(
            lote_2,
            resp_1.nuevo_cursor,
        ))
        .expect("La segunda página debe construirse");
        assert_eq!(resp_2.eventos.len(), 5);
        assert_eq!(resp_2.eventos.first().unwrap().cursor, 101);
        assert_eq!(resp_2.eventos.last().unwrap().cursor, 105);
        assert_eq!(resp_2.nuevo_cursor, 105);
        assert!(!resp_2.hay_mas);

        // Simula consulta sin eventos nuevos y reinicio explícito desde cero
        let resp_vacia = construir_respuesta_cambios(sincronizacion_db::armar_pagina_cambios(
            Vec::new(),
            resp_2.nuevo_cursor,
        ))
        .expect("La página vacía debe construirse");
        assert!(resp_vacia.eventos.is_empty());
        assert_eq!(resp_vacia.nuevo_cursor, 105);
        assert!(!resp_vacia.hay_mas);

        let lote_reinicio: Vec<_> = todos.iter().take(101).cloned().collect();
        let resp_reinicio =
            construir_respuesta_cambios(sincronizacion_db::armar_pagina_cambios(lote_reinicio, 0))
                .expect("El reinicio desde cero debe construirse");
        assert_eq!(resp_reinicio, resp_1);
    }

    // Valida aislamiento INV-17, paginación y reinicio de cursor en GET /cambios
    #[tokio::test]
    #[ignore = "requiere PostgreSQL de pruebas"]
    async fn get_cambios_aislamiento_paginacion_y_reinicio_con_postgres() {
        let url = std::env::var("E5_TEST_DATABASE_URL").expect("Configura E5_TEST_DATABASE_URL");
        let pool = db::conectar_y_migrar(&url)
            .await
            .expect("La base de pruebas debe conectarse");
        let secreto = "secreto de prueba con al menos 32 bytes";
        let app = construir_app(auth::State::new(pool.clone(), secreto.into()));

        let ana = Uuid::new_v4();
        let beto = Uuid::new_v4();
        let elena = Uuid::new_v4();
        let carla = Uuid::new_v4();
        let diego = Uuid::new_v4();
        let grupo_compartido = Uuid::new_v4();
        let grupo_ajeno = Uuid::new_v4();
        let sufijo = Uuid::new_v4().simple().to_string();

        // Inserta usuarios de prueba para dos grupos aislados
        for (id, nombre) in [
            (ana, "ana"),
            (beto, "beto"),
            (elena, "elena"),
            (carla, "carla"),
            (diego, "diego"),
        ] {
            let usuario = format!("{nombre}_{}", &sufijo[..8]);
            sqlx::query("INSERT INTO usuarios(id,nombre_usuario,nombre_visible,correo,password_hash) VALUES($1,$2,$3,$4,'hash')")
                .bind(id)
                .bind(&usuario)
                .bind(nombre)
                .bind(format!("{usuario}@example.com"))
                .execute(&pool)
                .await
                .expect("El usuario de prueba debe insertarse");
        }

        sqlx::query(
            "INSERT INTO grupos(id,nombre,moneda,creador_id) VALUES($1,'Viaje a Mérida','USD',$2)",
        )
        .bind(grupo_compartido)
        .bind(ana)
        .execute(&pool)
        .await
        .expect("El grupo compartido debe insertarse");
        for id in [ana, beto, elena] {
            sqlx::query("INSERT INTO grupo_miembros(grupo_id,usuario_id,estado_invitacion) VALUES($1,$2,'aceptada')")
                .bind(grupo_compartido)
                .bind(id)
                .execute(&pool)
                .await
                .expect("La membresía del grupo compartido debe insertarse");
        }

        sqlx::query(
            "INSERT INTO grupos(id,nombre,moneda,creador_id) VALUES($1,'Grupo Ajeno','USD',$2)",
        )
        .bind(grupo_ajeno)
        .bind(carla)
        .execute(&pool)
        .await
        .expect("El grupo ajeno debe insertarse");
        for id in [carla, diego] {
            sqlx::query("INSERT INTO grupo_miembros(grupo_id,usuario_id,estado_invitacion) VALUES($1,$2,'aceptada')")
                .bind(grupo_ajeno)
                .bind(id)
                .execute(&pool)
                .await
                .expect("La membresía del grupo ajeno debe insertarse");
        }

        let token_ana = firmar_token_prueba(ana, secreto, 10);
        let token_elena = firmar_token_prueba(elena, secreto, 10);
        let token_carla = firmar_token_prueba(carla, secreto, 10);
        let token_vencido = firmar_token_prueba(ana, secreto, -5);

        // Registra 105 gastos en el grupo compartido excluyendo a Elena
        for numero in 1..=105 {
            let cuerpo = json!({
                "concepto": format!("Gasto {numero}"),
                "monto": "10.00",
                "fecha": "2026-11-22",
                "pagado_por": ana,
                "regla": "iguales",
                "participantes": [
                    { "usuario_id": ana },
                    { "usuario_id": beto }
                ]
            })
            .to_string();
            let resp = app
                .clone()
                .oneshot(peticion_gasto_http(
                    grupo_compartido,
                    Some(&token_ana),
                    None,
                    &cuerpo,
                ))
                .await
                .expect("El registro de gasto compartido debe ejecutarse");
            assert_eq!(resp.status(), StatusCode::CREATED);
        }

        // Registra un gasto en el grupo ajeno para verificar aislamiento
        let cuerpo_ajeno = json!({
            "concepto": "Cena ajena",
            "monto": "20.00",
            "fecha": "2026-11-22",
            "pagado_por": carla,
            "regla": "iguales",
            "participantes": [
                { "usuario_id": carla },
                { "usuario_id": diego }
            ]
        })
        .to_string();
        let resp_ajeno = app
            .clone()
            .oneshot(peticion_gasto_http(
                grupo_ajeno,
                Some(&token_carla),
                None,
                &cuerpo_ajeno,
            ))
            .await
            .expect("El registro de gasto ajeno debe ejecutarse");
        assert_eq!(resp_ajeno.status(), StatusCode::CREATED);

        // Verifica rechazo 401 sin token, con token inválido y vencido
        for token in [None, Some("invalido"), Some(token_vencido.as_str())] {
            let resp = app
                .clone()
                .oneshot(peticion_cambios_http("/cambios?desde=0", token))
                .await
                .expect("La petición 401 debe ejecutarse");
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        }

        // Verifica rechazo 400 cuando el cursor desde es inválido
        for ruta in ["/cambios?desde=-1", "/cambios?desde=abc"] {
            let resp = app
                .clone()
                .oneshot(peticion_cambios_http(ruta, Some(&token_ana)))
                .await
                .expect("La petición 400 debe ejecutarse");
            assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        }

        // Verifica aislamiento INV-17 Elena excluida no recibe eventos
        let resp_elena = app
            .clone()
            .oneshot(peticion_cambios_http(
                "/cambios?desde=0",
                Some(&token_elena),
            ))
            .await
            .expect("La consulta de Elena debe ejecutarse");
        assert_eq!(resp_elena.status(), StatusCode::OK);
        let datos_elena: Value =
            serde_json::from_slice(&to_bytes(resp_elena.into_body(), 65536).await.unwrap())
                .unwrap();
        assert_eq!(datos_elena["eventos"].as_array().unwrap().len(), 0);
        assert_eq!(datos_elena["nuevo_cursor"], 0);
        assert_eq!(datos_elena["hay_mas"], false);

        // Verifica aislamiento INV-17 Carla solo recibe su grupo ajeno
        let resp_carla = app
            .clone()
            .oneshot(peticion_cambios_http(
                "/cambios?desde=0",
                Some(&token_carla),
            ))
            .await
            .expect("La consulta de Carla debe ejecutarse");
        assert_eq!(resp_carla.status(), StatusCode::OK);
        let datos_carla: Value =
            serde_json::from_slice(&to_bytes(resp_carla.into_body(), 65536).await.unwrap())
                .unwrap();
        let eventos_carla = datos_carla["eventos"].as_array().unwrap();
        assert_eq!(eventos_carla.len(), 1);
        assert_eq!(eventos_carla[0]["grupo_id"], grupo_ajeno.to_string());
        assert_eq!(eventos_carla[0]["grupo_nombre"], "Grupo Ajeno");
        assert_eq!(datos_carla["nuevo_cursor"], 1);
        assert_eq!(datos_carla["hay_mas"], false);

        // Comprueba primera página ordenada de cien eventos sin datos ajenos
        let resp_pag_1 = app
            .clone()
            .oneshot(peticion_cambios_http("/cambios?desde=0", Some(&token_ana)))
            .await
            .expect("La primera página de Ana debe ejecutarse");
        assert_eq!(resp_pag_1.status(), StatusCode::OK);
        let pagina_1: Value =
            serde_json::from_slice(&to_bytes(resp_pag_1.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        let eventos_1 = pagina_1["eventos"].as_array().unwrap();
        assert_eq!(eventos_1.len(), LIMITE_EVENTOS_CAMBIOS);
        assert_eq!(pagina_1["nuevo_cursor"], 100);
        assert_eq!(pagina_1["hay_mas"], true);
        for (indice, evento) in eventos_1.iter().enumerate() {
            let esperado = i64::try_from(indice + 1).expect("El cursor debe caber en i64");
            assert_eq!(evento["cursor"], esperado);
            assert_eq!(evento["grupo_id"], grupo_compartido.to_string());
            assert_eq!(evento["grupo_nombre"], "Viaje a Mérida");
        }

        // Comprueba segunda página con los cinco eventos restantes
        let resp_pag_2 = app
            .clone()
            .oneshot(peticion_cambios_http(
                "/cambios?desde=100",
                Some(&token_ana),
            ))
            .await
            .expect("La segunda página de Ana debe ejecutarse");
        assert_eq!(resp_pag_2.status(), StatusCode::OK);
        let pagina_2: Value =
            serde_json::from_slice(&to_bytes(resp_pag_2.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        let eventos_2 = pagina_2["eventos"].as_array().unwrap();
        assert_eq!(eventos_2.len(), 5);
        assert_eq!(pagina_2["nuevo_cursor"], 105);
        assert_eq!(pagina_2["hay_mas"], false);
        for (indice, evento) in eventos_2.iter().enumerate() {
            let esperado = i64::try_from(101 + indice).expect("El cursor debe caber en i64");
            assert_eq!(evento["cursor"], esperado);
            assert_eq!(evento["grupo_id"], grupo_compartido.to_string());
        }

        // Comprueba consulta sin eventos nuevos conservando el cursor
        let resp_vacia = app
            .clone()
            .oneshot(peticion_cambios_http(
                "/cambios?desde=105",
                Some(&token_ana),
            ))
            .await
            .expect("La consulta sin novedades debe ejecutarse");
        assert_eq!(resp_vacia.status(), StatusCode::OK);
        let pagina_vacia: Value =
            serde_json::from_slice(&to_bytes(resp_vacia.into_body(), 65536).await.unwrap())
                .unwrap();
        assert_eq!(pagina_vacia["eventos"].as_array().unwrap().len(), 0);
        assert_eq!(pagina_vacia["nuevo_cursor"], 105);
        assert_eq!(pagina_vacia["hay_mas"], false);

        // Comprueba reinicio del cursor a cero y consulta sin parámetro desde
        let resp_reinicio = app
            .clone()
            .oneshot(peticion_cambios_http("/cambios", Some(&token_ana)))
            .await
            .expect("El reinicio de cursor debe ejecutarse");
        assert_eq!(resp_reinicio.status(), StatusCode::OK);
        let pagina_reinicio: Value = serde_json::from_slice(
            &to_bytes(resp_reinicio.into_body(), 1024 * 1024)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(pagina_reinicio["nuevo_cursor"], 100);
        assert_eq!(pagina_reinicio["hay_mas"], true);
        assert_eq!(
            pagina_reinicio["eventos"].as_array().unwrap()[0]["id"],
            eventos_1[0]["id"]
        );

        // Limpia los datos de prueba respetando el orden de claves foránea
        sqlx::query("DELETE FROM eventos_usuario WHERE grupo_id = ANY($1)")
            .bind(&[grupo_compartido, grupo_ajeno][..])
            .execute(&pool)
            .await
            .expect("Los eventos de prueba deben limpiarse");
        sqlx::query("DELETE FROM grupos WHERE id = ANY($1)")
            .bind(&[grupo_compartido, grupo_ajeno][..])
            .execute(&pool)
            .await
            .expect("Los grupos de prueba deben limpiarse");
        sqlx::query("DELETE FROM usuarios WHERE id = ANY($1)")
            .bind(&[ana, beto, elena, carla, diego][..])
            .execute(&pool)
            .await
            .expect("Los usuarios de prueba deben limpiarse");
    }
}
