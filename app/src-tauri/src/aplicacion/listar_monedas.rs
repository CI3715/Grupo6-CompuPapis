//! Caso de uso para consultar el catalogo de monedas disponibles

// Importa la entidad Moneda Desde la capa de dominio
use crate::dominio::moneda::Moneda;

// Retorna la lista de monedas del catalogo base del sistema
pub fn ejecutar_listar_monedas() -> Vec<Moneda> {
    // Delega al dominio la obtencion de las monedas de fabrica
    Moneda::catalogo_base()
}

#[cfg(test)]
mod tests {
    // Importa los elementos del modulo padre para las pruebas
    use super::*;

    #[test]
    // Verifica que el caso de uso retorne las cuatro monedas base
    fn listar_monedas_retorna_catalogo_base() {
        let monedas = ejecutar_listar_monedas();
        assert_eq!(monedas.len(), 4);
    }
}
