//! Parte central del dominio financiero de la aplicacion, red e interfaz

// Submodulo con el catalgo base de monedas soportadas por la aplicacion
pub mod moneda;
// Submodulo con el objeto de valor Monto y sus validaciones
pub mod monto;
// Submodulo con la entidad Cuenta y tipos de cuentas soportados
pub mod cuenta;
// Submodulo con los eventos del libro personal
pub mod evento;

// Exporta los tipos de moneda para simplificar el uso en otros modulos
pub use moneda::{CodigoMoneda, ErrorMoneda, Moneda};
// Exporta monto y su error para uso directo en el dominio
pub use monto::{ErrorMonto, Monto};
// Exporta los tipos de cuenta para su uso externo
pub use cuenta::{Cuenta, ErrorCuenta, TipoCuenta};
// Exporta el evento CuentaCreada y su tipo de error asociado
pub use evento::{CuentaCreada, ErrorEvento, TIPO_EVENTO_CUENTA_CREADA};
