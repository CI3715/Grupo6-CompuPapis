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
#[allow(clippy::too_many_arguments)] // Preserva el contrato e inyección de reloj e identificadores.
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

#[allow(clippy::too_many_arguments)] // Preserva el contrato e inyección de reloj e identificadores.
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
        id,
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
    let monedas = ejecutar_listar_monedas(repo)?;
    let origen = crate::domain::buscar_moneda_en(origen, &monedas)
        .map_err(|_| ErrorDominio::nuevo("origen", "Esa moneda no existe."))?;
    let destino = crate::domain::buscar_moneda_en(destino, &monedas)
        .map_err(|_| ErrorDominio::nuevo("destino", "Esa moneda no existe."))?;
    if origen.codigo == destino.codigo {
        return Err(ErrorDominio::nuevo(
            "destino",
            "No se puede usar la misma moneda.",
        ));
    }
    // Historial de tasas activas; el libro conserva también las anuladas.
    Ok(ejecutar_listar_tasas(repo)?
        .into_iter()
        .filter(|t| t.origen == origen.codigo && t.destino == destino.codigo)
        .collect())
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
    if !existe {
        return Err(ErrorDominio::nuevo("codigo", "Esa moneda no existe."));
    }

    let dato = DatosMonedaBaseCambiada {
        codigo: codigo.to_string(),
    };

    let datos = serde_json::to_string(&dato)
        .map_err(|_| ErrorDominio::nuevo("general", "No se pudo guardar el cambio."))?;

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

/// Entrada de aplicación compartida por IPC y pruebas sin runtime Tauri.
#[derive(Default, serde::Deserialize)]
pub struct EntradaTasa {
    pub origen: Option<String>,
    pub destino: Option<String>,
    pub valor: Option<String>,
    pub fecha: Option<String>,
    pub fuente: Option<String>,
}

pub fn registrar_tasa_desde_entrada<R: RepositorioEventos>(
    repo: &mut R,
    entrada: EntradaTasa,
    dispositivo_id: &str,
    reloj: &impl Reloj,
    ids: &impl GeneradorId,
) -> Result<TasaCambio, ErrorDominio> {
    let obligatorio = |campo: &str, valor: Option<String>| {
        valor
            .ok_or_else(|| ErrorDominio::nuevo(campo, &format!("El campo {campo} es obligatorio.")))
    };
    ejecutar_registrar_tasa(
        repo,
        &obligatorio("origen", entrada.origen)?,
        &obligatorio("destino", entrada.destino)?,
        &obligatorio("valor", entrada.valor)?,
        &obligatorio("fecha", entrada.fecha)?,
        &obligatorio("fuente", entrada.fuente)?,
        dispositivo_id,
        reloj,
        ids,
    )
}
