//! Parte central del dominio financiero de la aplicacion, red e interfaz

// Submodulo con el catalgo base de monedas soportadas por la aplicacion
pub mod moneda;

// Reexporta los tipos de moneda para simplificar el uso en otros modulos
pub use moneda::{CodigoMoneda, ErrorMoneda, Moneda};

// Pendiente por crear los submodulos monto y cuenta
