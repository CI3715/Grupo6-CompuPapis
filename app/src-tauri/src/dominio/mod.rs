//! Parte central del dominio financiero de la aplicacion, red e interfaz

// Submodulo con el catalgo base de monedas soportadas por la aplicacion
pub mod moneda;
// Submodulo con el objeto de valor Monto y sus validaciones
pub mod monto;

// Exporta los tipos de moneda para simplificar el uso en otros modulos
pub use moneda::{CodigoMoneda, ErrorMoneda, Moneda};
// Exporta monto y su error para uso directo en el dominio
pub use monto::{ErrorMonto, Monto};

// Pendiente por crear el modulo cuenta
