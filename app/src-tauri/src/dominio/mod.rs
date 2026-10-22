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
// Exporta eventos y el contenedor del libro personal
pub use evento::{CuentaCreada, ErrorEvento, LibroPersonal, TIPO_EVENTO_CUENTA_CREADA};

#[cfg(test)]
mod tests {
    // Pruebas de arquitectura sobre el codigo del dominio

    // Devuelve los archivos fuente del dominio sin sus bloques de prueba
    fn codigo_produccion_dominio() -> [(&'static str, &'static str); 5] {
        [
            ("mod.rs", include_str!("mod.rs")),
            ("moneda.rs", include_str!("moneda.rs")),
            ("monto.rs", include_str!("monto.rs")),
            ("cuenta.rs", include_str!("cuenta.rs")),
            ("evento.rs", include_str!("evento.rs")),
        ]
    }

    #[test]
    fn dominio_no_importa_dependencias_externas_prohibidas() {
        // Verifica que el dominio no importe rusqlite, tauri ni reqwest
        let prohibidas = ["rusqlite", "tauri", "reqwest"];

        for (archivo, contenido) in codigo_produccion_dominio() {
            let codigo_produccion = contenido.split("[cfg(test)]").next().unwrap_or("");
            for dependencia in prohibidas {
                assert!(
                    !codigo_produccion.contains(dependencia),
                    "El archivo {archivo} viola la regla de dependencia importando {dependencia}"
                );
            }
        }
    }

    #[test]
    fn dominio_no_usa_tipos_de_coma_flotante() {
        // Verifica que el dominio use f32 ni f64
        let flotantes_prohibidos = ["f32", "f64"];

        for (archivo, contenido) in codigo_produccion_dominio() {
            let codigo_produccion = contenido.split("[cfg(test)]").next().unwrap_or("");
            for tipo_flotante in flotantes_prohibidos {
                assert!(
                    !codigo_produccion.contains(tipo_flotante),
                    "El archivo {archivo} viola la regla de flotantes prohibidos importando {tipo_flotante}"
                );
            }
        }
    }
}
