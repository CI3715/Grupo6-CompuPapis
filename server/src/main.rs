//! main.rs: Servidor HTTP de Cuentas Claras, rutas principales y modelos de eventos.
//!
//! Dirigido al equipo backend para gestionar el arranque y contratos de sincronización.

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
