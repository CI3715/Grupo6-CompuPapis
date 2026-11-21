//! main.rs: Servidor HTTP de Cuentas Claras, rutas principales y modelos de eventos
//!
//! Dirigido al equipo backend para gestionar el arranque y contratos de sincronización

pub mod amistades_db;
mod auth;
mod db;
pub mod division;
pub mod gastos_db;
mod grupos;
mod social;
pub mod sincronizacion_db;
pub mod usuarios_db;

use axum::{Json, Router, routing::get};
use serde::Serialize;
use chrono::NaiveDate;
use uuid::Uuid;

use crate::sincronizacion_db::EventoUsuario;

use std::collections::HashSet;

use axum::http::StatusCode;

use crate::gastos_db::ErrorGasto;

use sqlx::{Postgres, Transaction};

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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
pub async fn emitir_eventos_gasto(
    tx: &mut Transaction<'_, Postgres>,
    solicitante_id: Uuid,
    miembros_activos: &HashSet<Uuid>,
    participantes_gasto: &[Uuid],
    gasto_id: Uuid,
) -> Result<Vec<Uuid>, ErrorGasto> {
    // Verifica membresía y excluye no participantes antes de escribir en base de datos
    let destinatarios = filtrar_destinatarios_evento(
        solicitante_id,
        miembros_activos,
        participantes_gasto,
    )
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL").map_err(|_| "Falta DATABASE_URL")?;
    let secret = std::env::var("JWT_SECRET").map_err(|_| "Falta JWT_SECRET")?;
    if secret.len() < 32 {
        return Err("JWT_SECRET debe tener al menos 32 bytes".into());
    }

    let pool = db::conectar_y_migrar(&database_url).await?;

    let modo_demostracion = std::env::var("TOKEN_DEMO_1_MINUTO").as_deref() == Ok("1");
    let estado = auth::State::new(pool, secret).con_modo_demostracion(modo_demostracion);
    let app = Router::new()
        .route("/ping", get(ping))
        .merge(grupos::routes(estado.clone()))
        .merge(social::routes(estado.clone()))
        .merge(auth::routes(estado));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000").await?;
    println!("Servidor activo en http://127.0.0.1:8000");
    axum::serve(listener, app).await?;
    Ok(())
}
