//! Capa de aplicacion que contiene los puertos y casos de uso

// Declara el modulo donde se encuentran las interfaces de salida
pub mod puertos;

// Declara el modulo del caso de uso para listar monedas
pub mod listar_monedas;

// Exporta los traits para uso externo
pub use puertos::{GeneradorId, Reloj, RepositorioEventos};

// Exporta la funcion de caso de uso para uso externo
pub use listar_monedas::ejecutar_listar_monedas;
