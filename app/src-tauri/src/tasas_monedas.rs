use crate::domain::{
    ejecutar_listar_monedas, validar_tasa, ErrorDominio, EventoLibro, Moneda, RepositorioEventos, TasaCambio, 
    ejecutar_listar_tasas, DatosTasaEliminada, DatosMonedaBaseCambiada,
};

pub use crate::domain::validar_moneda;

use chrono:: {Local, Utc};

pub trait Reloj {
    fn ahora(&self) -> chrono::DateTime<Utc>;
}

pub trait GeneradorId {
    fn nuevo_id(&self) -> String;
}

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

    let lista = ejecutar_listar_monedas(repo)?;
    let nueva = validar_moneda(codigo, nombre, simbolo, decimales, &lista)?;

    let datos = match serde_json::to_string(&nueva) {
        Ok(d) => d,
        Err(_) => return Err(ErrorDominio::nuevo("general", "Hubo un problema al guardar la moneda.")),
    };
    // Creamos el evento para asi guardar la moneda
    let evento = EventoLibro {
        id: ids.nuevo_id(),
        tipo: "MonedaCreada".to_string(),
        momento_registro: reloj.ahora(),
        dispositivo_origen: dispositivo_id.to_string(),
        datos_json: datos,
    };

    if repo.guardar_evento(&evento).is_err() {
        return Err(ErrorDominio::nuevo("general", "Hubo un problema al guardar la moneda."));
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
    let nueva = validar_tasa(
        &id, origen, destino, valor, fecha, fuente, &lista, hoy
    )?;

    let datos = match serde_json::to_string(&nueva) {
        Ok(dato) => dato,
        Err(_) => {
            return Err(ErrorDominio::nuevo("general", "Hubo un problema al guardar la tasa."));
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
        return Err(ErrorDominio::nuevo("general", "Hubo un problema al guardar la tasa."));
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
            return Err(ErrorDominio::nuevo("general", "No se pudieron cargar las tasas."));
        }
    };

    let mut resp = Vec::new();

    // Chequeamos las tasas que tenemos guardadas
    for ev in lista {
        if ev.tipo == "TasaRegistrada" {

            let tasa: TasaCambio = match serde_json::from_str(&ev.datos_json) {
                Ok(t) => t,
                Err(_) => {
                    return Err(ErrorDominio::nuevo("general", "Hubo un problema al leer una tasa."));
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
        return Err(ErrorDominio::nuevo("tasa_id", "La tasa no existe o ya se elimino."));
    }

    let dato = DatosTasaEliminada {
        tasa_id: tasa_id.to_string(),
    };

    let datos = match serde_json::to_string(&dato) {
        Ok(d) => d,
        Err(_) => {
            return Err(ErrorDominio::nuevo("general", "Hubo un problema al eliminar la tasa."));
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
        return Err(ErrorDominio::nuevo("general", "Hubo un problema al eliminar la tasa."));
    }

    Ok(())
}

pub fn ejecutar_cambiar_moneda_base<R: RepositorioEventos>(
    repo: &mut R,
    codigo: &str,
    dispositivo_id: &str,
    reloj: &impl Reloj,
    ids: &impl GeneradorId,
) -> Result<String, ErrorDominio> {

    let lista = ejecutar_listar_monedas(repo)?;
    let mut existe = false;

    // Vemos si tenemos esa moneda
    for m in lista {
        if m.codigo == codigo {
            existe = true;
            break;
        }
    }

    if existe == false {
        return Err(ErrorDominio::nuevo("codigo", "Esa moneda no existe."));
    }

    let dato = DatosMonedaBaseCambiada {
        codigo: codigo.to_string(),
    };

    let datos;

    match serde_json::to_string(&dato) {
        Ok(d) => {
            datos = d;
        }
        Err(_) => {
            return Err(ErrorDominio::nuevo("general", "No se pudo guardar el cambio."));
        }
    }

    // Guardamos el cambio de moneda
    let evento = EventoLibro {
        id: ids.nuevo_id(),
        tipo: "MonedaBaseCambiada".to_string(),
        momento_registro: reloj.ahora(),
        dispositivo_origen: dispositivo_id.to_string(),
        datos_json: datos,
    };

    let resp = repo.guardar_evento(&evento);

    if resp.is_err() {
        return Err(ErrorDominio::nuevo("general", "No se pudo guardar el cambio."));
    }

    Ok(codigo.to_string())
}