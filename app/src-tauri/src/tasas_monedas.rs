// Importa tipos y consultas del dominio necesarios para gestionar monedas
use crate::domain::{
    DatosMonedaBaseCambiada, DatosTasaEliminada, ErrorDominio, EventoLibro, Moneda,
    RepositorioEventos, TasaCambio, ejecutar_listar_monedas, ejecutar_listar_tasas, validar_tasa,
};

// Reexporta la validación de monedas para exponer una API accesible
pub use crate::domain::validar_moneda;

use chrono::{Local, Utc};

// Puerto que abstrae la obtención de la fecha y hora actual
pub trait Reloj {
    fn ahora(&self) -> chrono::DateTime<Utc>;
}

// Puerto que abstrae la generación de identificadores únicos de eventos
pub trait GeneradorId {
    fn nuevo_id(&self) -> String;
}

// Valida una moneda propia
pub fn ejecutar_crear_moneda<R: RepositorioEventos>(
    repo: &mut R,
    codigo: &str,
    nombre: &str,
    simbolo: &str,
    decimales: u32,
    dispositivo_id: &str,
    reloj: &impl Reloj,
    ids: &impl GeneradorId,
) -> Result<Moneda, ErrorDominio> {
    // Obtiene el catálogo actual y valida las reglas de la nueva moneda
    let lista = ejecutar_listar_monedas(repo)?;
    let nueva = validar_moneda(codigo, nombre, simbolo, decimales, &lista)?;

    // Serializa los datos de la moneda a formato JSON para el evento
    let datos = match serde_json::to_string(&nueva) {
        Ok(d) => d,
        Err(_) => {
            return Err(ErrorDominio::nuevo(
                "general",
                "Hubo un problema al guardar la moneda.",
            ));
        }
    };

    // Construye el evento inmutable usando los puertos de ID y reloj
    let evento = EventoLibro {
        id: ids.nuevo_id(),
        tipo: "MonedaCreada".to_string(),
        momento_registro: reloj.ahora(),
        dispositivo_origen: dispositivo_id.to_string(),
        datos_json: datos,
    };

    // Verifica el evento en el repositorio local y maneja posibles errores
    if repo.guardar_evento(&evento).is_err() {
        return Err(ErrorDominio::nuevo(
            "general",
            "Hubo un problema al guardar la moneda.",
        ));
    }

    Ok(nueva)
}

pub fn ejecutar_registrar_tasa<R: RepositorioEventos>(
    repo: &mut R,
    origen: &str,
    destino: &str,
    valor: &str,
    fecha: &str,
    fuente: &str,
    dispositivo_id: &str,
    reloj: &impl Reloj,
    ids: &impl GeneradorId,
) -> Result<TasaCambio, ErrorDominio> {
    let lista = ejecutar_listar_monedas(repo)?;
    let id = ids.nuevo_id();
    let hoy = reloj.ahora().with_timezone(&Local).date_naive();

    // Revisamos los datos de la tasa
    let nueva = validar_tasa(&id, origen, destino, valor, fecha, fuente, &lista, hoy)?;

    let datos = match serde_json::to_string(&nueva) {
        Ok(dato) => dato,
        Err(_) => {
            return Err(ErrorDominio::nuevo(
                "general",
                "Hubo un problema al guardar la tasa.",
            ));
        }
    };

    // Guardamos la tasa
    let evento = EventoLibro {
        id: id,
        tipo: "TasaRegistrada".to_string(),
        momento_registro: reloj.ahora(),
        dispositivo_origen: dispositivo_id.to_string(),
        datos_json: datos,
    };

    let resp = repo.guardar_evento(&evento);

    if resp.is_err() {
        return Err(ErrorDominio::nuevo(
            "general",
            "Hubo un problema al guardar la tasa.",
        ));
    }

    Ok(nueva)
}

pub fn ejecutar_historial_tasas<R: RepositorioEventos>(
    repo: &R,
    origen: &str,
    destino: &str,
) -> Result<Vec<TasaCambio>, ErrorDominio> {
    let eventos = repo.obtener_eventos();

    let lista = match eventos {
        Ok(e) => e,
        Err(_) => {
            return Err(ErrorDominio::nuevo(
                "general",
                "No se pudieron cargar las tasas.",
            ));
        }
    };

    let mut resp = Vec::new();

    // Chequeamos las tasas que tenemos guardadas
    for ev in lista {
        if ev.tipo == "TasaRegistrada" {
            let tasa: TasaCambio = match serde_json::from_str(&ev.datos_json) {
                Ok(t) => t,
                Err(_) => {
                    return Err(ErrorDominio::nuevo(
                        "general",
                        "Hubo un problema al leer una tasa.",
                    ));
                }
            };

            if tasa.id != ev.id {
                return Err(ErrorDominio::nuevo("general", "Hay un error en una tasa."));
            }

            if tasa.origen == origen && tasa.destino == destino {
                resp.push(tasa);
            }
        }
    }

    Ok(resp)
}

pub fn ejecutar_eliminar_tasa<R: RepositorioEventos>(
    repo: &mut R,
    tasa_id: &str,
    dispositivo_id: &str,
    reloj: &impl Reloj,
    ids: &impl GeneradorId,
) -> Result<(), ErrorDominio> {
    let tasas = ejecutar_listar_tasas(repo)?;
    let mut existe = false;

    // Buscamos la tasa que queremos eliminar
    for t in tasas {
        if t.id == tasa_id {
            existe = true;
            break;
        }
    }

    if !existe {
        return Err(ErrorDominio::nuevo(
            "tasa_id",
            "La tasa no existe o ya se elimino.",
        ));
    }

    let dato = DatosTasaEliminada {
        tasa_id: tasa_id.to_string(),
    };

    let datos = match serde_json::to_string(&dato) {
        Ok(d) => d,
        Err(_) => {
            return Err(ErrorDominio::nuevo(
                "general",
                "Hubo un problema al eliminar la tasa.",
            ));
        }
    };

    // Guardamos el evento de eliminación
    let evento = EventoLibro {
        id: ids.nuevo_id(),
        tipo: "TasaEliminada".to_string(),
        momento_registro: reloj.ahora(),
        dispositivo_origen: dispositivo_id.to_string(),
        datos_json: datos,
    };

    let resp = repo.guardar_evento(&evento);

    if resp.is_err() {
        return Err(ErrorDominio::nuevo(
            "general",
            "Hubo un problema al eliminar la tasa.",
        ));
    }

    Ok(())
}

// Verifica que la moneda exista y aplica el evento MonedaBaseCambiada
pub fn ejecutar_cambiar_moneda_base<R: RepositorioEventos>(
    repo: &mut R,
    codigo: &str,
    dispositivo_id: &str,
    reloj: &impl Reloj,
    ids: &impl GeneradorId,
) -> Result<String, ErrorDominio> {
    let lista = ejecutar_listar_monedas(repo)?;
    let mut existe = false;

    // Comprueba si el código solicitado pertenece al catálogo dinámico actual
    for m in lista {
        if m.codigo == codigo {
            existe = true;
            break;
        }
    }

    // Rechaza el cambio indicando el campo exacto si la moneda no existe
    if existe == false {
        return Err(ErrorDominio::nuevo("codigo", "Esa moneda no existe."));
    }

    let dato = DatosMonedaBaseCambiada {
        codigo: codigo.to_string(),
    };

    let datos;

    // Serializa el código de la nueva moneda base para el evento
    match serde_json::to_string(&dato) {
        Ok(d) => {
            datos = d;
        }
        Err(_) => {
            return Err(ErrorDominio::nuevo(
                "general",
                "No se pudo guardar el cambio.",
            ));
        }
    }

    // Registra el evento MonedaBaseCambiada con su tiempo e ID
    let evento = EventoLibro {
        id: ids.nuevo_id(),
        tipo: "MonedaBaseCambiada".to_string(),
        momento_registro: reloj.ahora(),
        dispositivo_origen: dispositivo_id.to_string(),
        datos_json: datos,
    };

    let resp = repo.guardar_evento(&evento);

    // Retorna error general si falla la escritura en almacenamiento
    if resp.is_err() {
        return Err(ErrorDominio::nuevo(
            "general",
            "No se pudo guardar el cambio.",
        ));
    }

    Ok(codigo.to_string())
}
