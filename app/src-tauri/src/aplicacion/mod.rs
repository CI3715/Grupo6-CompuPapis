//! Capa de aplicacion que contiene los puertos y casos de uso

// Declara el modulo donde se encuentran las interfaces de salida
pub mod puertos;

// Exporta los traits para uso externo
pub use puertos::{GeneradorId, Reloj, RepositorioEventos};
