//! Declara los puertos de salida para desacoplar la capa de aplicacion

// Importa el tipo de fecha y hora en formato UTC
use chrono::{DateTime, Utc};

// Importa la estructura de evento inmutable desde el modulo dominio
use crate::dominio::CuentaCreada;

// Contrato para guardar y consultar eventos del libro personal
pub trait RepositorioEventos {
    // Guardar un evento dentro del almacenamiento local del libro
    fn guardar_evento(&mut self, evento: &CuentaCreada) -> Result<(), String>;

    // Obtiene la lista completa de eventos registrado en el libro
    fn obtener_eventos(&self) -> Result<Vec<CuentaCreada>, String>;
}

// Contrato para obtener la fecha y hora actual del sistema
pub trait Reloj {
    // Devuelve la fecha y hora actual en formato UTC
    fn ahora(&self) -> DateTime<Utc>;
}

// Contrato para generar identificadores unicos en cuentas y eventos
pub trait GeneradorId {
    // Genera un nuevo identificador unico en formato de texto
    fn nuevo_id(&self) -> String;
}
