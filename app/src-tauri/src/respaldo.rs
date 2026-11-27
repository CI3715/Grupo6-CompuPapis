//! respaldo.rs: Exportación, cifrado autenticado (Argon2id + AES-256-GCM)
//! e importación idempotente del libro de eventos (RF-INT-01, RF-INT-02, RF-INT-05, RF-INT-06, RNF-04).

use crate::domain::{
    ErrorDominio, EventoLibro, RepositorioEventos, ResultadoFusion, ordenar_eventos_determinista,
};
use crate::sqlite_repo::RepositorioSqlite;
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use chrono::{DateTime, Utc};
use ring::aead::{AES_256_GCM, Aad, LessSafeKey, Nonce, UnboundKey};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use zeroize::Zeroizing;

pub const FORMATO_LIBRO: &str = "cuentas-claras-libro";
pub const FORMATO_CIFRADO: &str = "cuentas-claras-respaldo-cifrado";
pub const VERSION_FORMATO_ACTUAL: u32 = 1;
pub const LONGITUD_MINIMA_FRASE: usize = 12;
pub const ALGORITMO_KDF: &str = "argon2id";
pub const ALGORITMO_CIFRADO: &str = "aes-256-gcm";

pub const MENSAJE_FRASE_O_DANADO: &str = "Frase de paso incorrecta o archivo dañado.";

const TAMANO_SAL: usize = 16;
const TAMANO_NONCE: usize = 12;
const TAMANO_CLAVE: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParametrosKdf {
    pub memoria_kib: u32,
    pub iteraciones: u32,
    pub paralelismo: u32,
}

impl ParametrosKdf {
    /// Parámetros recomendados para producción (64 MiB, 3 iteraciones, 1 hilo — ver ADR 014).
    pub const fn estandar() -> Self {
        Self {
            memoria_kib: 65_536,
            iteraciones: 3,
            paralelismo: 1,
        }
    }

    /// Parámetros ligeros únicamente para pruebas unitarias repetitivas (mismo algoritmo Argon2id).
    #[cfg(test)]
    pub const fn para_pruebas() -> Self {
        Self {
            memoria_kib: 4_096,
            iteraciones: 1,
            paralelismo: 1,
        }
    }

    fn validar(&self) -> Result<(), ErrorDominio> {
        if !(1_024..=524_288).contains(&self.memoria_kib)
            || !(1..=20).contains(&self.iteraciones)
            || !(1..=16).contains(&self.paralelismo)
        {
            return Err(ErrorDominio::nuevo("archivo", MENSAJE_FRASE_O_DANADO));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncabezadoLibro {
    pub formato: String,
    pub version: u32,
    pub fecha_exportacion: String,
    pub cantidad_eventos: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EventoSerializado {
    id: String,
    tipo: String,
    momento_registro: String,
    dispositivo_origen: String,
    datos: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InfoKdf {
    algoritmo: String,
    memoria_kib: u32,
    iteraciones: u32,
    paralelismo: u32,
}

/// Envoltorio cifrado: la parte en claro no revela ningún dato del libro (RF-INT-06).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArchivoCifrado {
    formato: String,
    version: u32,
    kdf: InfoKdf,
    cifrado: String,
    sal: String,
    nonce: String,
    datos: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumenExportacion {
    pub ruta: String,
    pub cantidad_eventos: usize,
    pub cifrado: bool,
    pub fecha_exportacion: String,
}

/// Valida que la frase cumpla la longitud mínima (12 caracteres) y coincida con su confirmación.
pub fn validar_frase_doble(frase: &str, confirmacion: &str) -> Result<(), ErrorDominio> {
    if frase.chars().count() < LONGITUD_MINIMA_FRASE {
        return Err(ErrorDominio::nuevo(
            "frase",
            "La frase de paso debe tener al menos 12 caracteres.",
        ));
    }
    if frase != confirmacion {
        return Err(ErrorDominio::nuevo(
            "confirmacion",
            "Las frases de paso no coinciden.",
        ));
    }
    Ok(())
}

/// Valida una frase de paso individual (para configurar sincronización).
pub fn validar_frase_minima(frase: &str) -> Result<(), ErrorDominio> {
    if frase.chars().count() < LONGITUD_MINIMA_FRASE {
        return Err(ErrorDominio::nuevo(
            "frase",
            "La frase de paso debe tener al menos 12 caracteres.",
        ));
    }
    Ok(())
}

/// Serializa un conjunto de eventos en formato NDJSON legible por una persona (Sección 2.2, RNF-04).
/// No modifica el libro ni incluye credenciales, tokens ni la frase de paso (RF-INT-01, RF-INT-05).
pub fn serializar_eventos_ndjson(
    eventos: &[EventoLibro],
    fecha_exportacion: DateTime<Utc>,
) -> Result<String, ErrorDominio> {
    let ordenados = ordenar_eventos_determinista(eventos);
    let encabezado = EncabezadoLibro {
        formato: FORMATO_LIBRO.to_string(),
        version: VERSION_FORMATO_ACTUAL,
        fecha_exportacion: fecha_exportacion.to_rfc3339(),
        cantidad_eventos: ordenados.len(),
    };

    let mut salida = serde_json::to_string(&encabezado)
        .map_err(|_| ErrorDominio::nuevo("exportacion", "No se pudo serializar el encabezado."))?;
    salida.push('\n');

    for ev in ordenados {
        let datos: Value = serde_json::from_str(&ev.datos_json).map_err(|_| {
            ErrorDominio::nuevo(
                "exportacion",
                "El libro contiene un evento con datos JSON inválidos.",
            )
        })?;
        let linea_ev = EventoSerializado {
            id: ev.id.clone(),
            tipo: ev.tipo.clone(),
            momento_registro: ev.momento_registro.to_rfc3339(),
            dispositivo_origen: ev.dispositivo_origen.clone(),
            datos,
        };
        let texto_linea = serde_json::to_string(&linea_ev)
            .map_err(|_| ErrorDominio::nuevo("exportacion", "No se pudo serializar un evento."))?;
        salida.push_str(&texto_linea);
        salida.push('\n');
    }

    Ok(salida)
}

/// Parsea y valida por completo un archivo NDJSON en claro antes de aplicar nada (Sección 2.4).
pub fn parsear_eventos_ndjson(
    contenido: &str,
) -> Result<(EncabezadoLibro, Vec<EventoLibro>), ErrorDominio> {
    let mut lineas = contenido.lines().map(str::trim).filter(|l| !l.is_empty());

    let primera = lineas.next().ok_or_else(|| {
        ErrorDominio::nuevo(
            "archivo",
            "El archivo está vacío o no tiene el formato de copia de seguridad.",
        )
    })?;

    let encabezado: EncabezadoLibro = serde_json::from_str(primera).map_err(|_| {
        ErrorDominio::nuevo(
            "archivo",
            "El archivo no tiene un encabezado válido de copia de seguridad.",
        )
    })?;

    if encabezado.formato != FORMATO_LIBRO {
        return Err(ErrorDominio::nuevo(
            "archivo",
            "El archivo no corresponde a un respaldo de Cuentas Claras.",
        ));
    }

    if encabezado.version != VERSION_FORMATO_ACTUAL {
        return Err(ErrorDominio::nuevo(
            "version",
            &format!(
                "Versión de formato no soportada: el archivo tiene la versión {} y esta aplicación soporta la versión {}.",
                encabezado.version, VERSION_FORMATO_ACTUAL
            ),
        ));
    }

    if DateTime::parse_from_rfc3339(&encabezado.fecha_exportacion).is_err() {
        return Err(ErrorDominio::nuevo(
            "archivo",
            "La fecha de exportación del archivo no es válida.",
        ));
    }

    let mut eventos = Vec::new();
    for (idx, linea) in lineas.enumerate() {
        let num_linea = idx + 2;
        let ev: EventoSerializado = serde_json::from_str(linea).map_err(|_| {
            ErrorDominio::nuevo(
                "archivo",
                &format!("El evento en la línea {num_linea} está mal formado."),
            )
        })?;

        if ev.id.trim().is_empty()
            || ev.tipo.trim().is_empty()
            || ev.dispositivo_origen.trim().is_empty()
        {
            return Err(ErrorDominio::nuevo(
                "archivo",
                &format!("El evento en la línea {num_linea} tiene campos obligatorios vacíos."),
            ));
        }

        let momento_registro = DateTime::parse_from_rfc3339(&ev.momento_registro)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|_| {
                ErrorDominio::nuevo(
                    "archivo",
                    &format!(
                        "El evento en la línea {num_linea} tiene una fecha de registro inválida."
                    ),
                )
            })?;

        let datos_json = serde_json::to_string(&ev.datos).map_err(|_| {
            ErrorDominio::nuevo(
                "archivo",
                &format!("No se pudieron leer los datos del evento en la línea {num_linea}."),
            )
        })?;

        // Si los datos estan mal no importamos nada
        if !crate::domain::datos_evento_validos(&ev.tipo, &datos_json) {
            return Err(ErrorDominio::nuevo(
                "archivo",
                &format!(
                    "Los datos del evento en la linea {} no son validos.",
                    num_linea
                ),
            ));
        }

        eventos.push(EventoLibro {
            id: ev.id,
            tipo: ev.tipo,
            momento_registro,
            dispositivo_origen: ev.dispositivo_origen,
            datos_json,
        });
    }

    if eventos.len() != encabezado.cantidad_eventos {
        return Err(ErrorDominio::nuevo(
            "archivo",
            &format!(
                "El archivo indica {} eventos, pero contiene {}. El archivo podría estar incompleto.",
                encabezado.cantidad_eventos,
                eventos.len()
            ),
        ));
    }

    Ok((encabezado, eventos))
}

fn construir_aad(
    formato: &str,
    version: u32,
    kdf: &InfoKdf,
    cifrado: &str,
    sal_b64: &str,
    nonce_b64: &str,
) -> Vec<u8> {
    format!(
        "{formato}|v{version}|{}|{}|{}|{}|{cifrado}|{sal_b64}|{nonce_b64}",
        kdf.algoritmo, kdf.memoria_kib, kdf.iteraciones, kdf.paralelismo
    )
    .into_bytes()
}

fn derivar_clave(
    frase: &str,
    sal: &[u8],
    params: ParametrosKdf,
) -> Result<Zeroizing<[u8; TAMANO_CLAVE]>, ErrorDominio> {
    params.validar()?;
    let argon_params = Params::new(
        params.memoria_kib,
        params.iteraciones,
        params.paralelismo,
        Some(TAMANO_CLAVE),
    )
    .map_err(|_| ErrorDominio::nuevo("cifrado", "Parámetros de derivación inválidos."))?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon_params);
    let mut clave = Zeroizing::new([0u8; TAMANO_CLAVE]);
    argon2
        .hash_password_into(frase.as_bytes(), sal, clave.as_mut())
        .map_err(|_| ErrorDominio::nuevo("cifrado", "No se pudo derivar la clave de cifrado."))?;
    Ok(clave)
}

/// Cifra bytes arbitrarios usando Argon2id (sal aleatoria nueva) + AES-256-GCM (nonce aleatorio nuevo + AAD).
pub fn cifrar_con_parametros(
    texto_plano: &[u8],
    frase: &str,
    params: ParametrosKdf,
) -> Result<Vec<u8>, ErrorDominio> {
    validar_frase_minima(frase)?;

    let rng = SystemRandom::new();
    let mut sal = [0u8; TAMANO_SAL];
    let mut nonce_bytes = [0u8; TAMANO_NONCE];
    rng.fill(&mut sal)
        .map_err(|_| ErrorDominio::nuevo("cifrado", "No se pudo generar la sal aleatoria."))?;
    rng.fill(&mut nonce_bytes)
        .map_err(|_| ErrorDominio::nuevo("cifrado", "No se pudo generar el nonce aleatorio."))?;

    let clave = derivar_clave(frase, &sal, params)?;

    let kdf_info = InfoKdf {
        algoritmo: ALGORITMO_KDF.to_string(),
        memoria_kib: params.memoria_kib,
        iteraciones: params.iteraciones,
        paralelismo: params.paralelismo,
    };
    let sal_b64 = B64.encode(sal);
    let nonce_b64 = B64.encode(nonce_bytes);
    let aad = construir_aad(
        FORMATO_CIFRADO,
        VERSION_FORMATO_ACTUAL,
        &kdf_info,
        ALGORITMO_CIFRADO,
        &sal_b64,
        &nonce_b64,
    );

    let unbound = UnboundKey::new(&AES_256_GCM, clave.as_ref())
        .map_err(|_| ErrorDominio::nuevo("cifrado", "No se pudo inicializar el cifrador."))?;
    let key = LessSafeKey::new(unbound);
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);

    let mut buffer = texto_plano.to_vec();
    key.seal_in_place_append_tag(nonce, Aad::from(&aad), &mut buffer)
        .map_err(|_| ErrorDominio::nuevo("cifrado", "No se pudo cifrar el contenido."))?;

    let envoltorio = ArchivoCifrado {
        formato: FORMATO_CIFRADO.to_string(),
        version: VERSION_FORMATO_ACTUAL,
        kdf: kdf_info,
        cifrado: ALGORITMO_CIFRADO.to_string(),
        sal: sal_b64,
        nonce: nonce_b64,
        datos: B64.encode(buffer),
    };

    serde_json::to_vec_pretty(&envoltorio)
        .map_err(|_| ErrorDominio::nuevo("cifrado", "No se pudo empaquetar el archivo cifrado."))
}

/// Cifra con los parámetros estándar de producción (Argon2id 64 MiB, 3 iteraciones + AES-256-GCM).
pub fn cifrar(texto_plano: &[u8], frase: &str) -> Result<Vec<u8>, ErrorDominio> {
    cifrar_con_parametros(texto_plano, frase, ParametrosKdf::estandar())
}

/// Descifra y autentica un archivo cifrado. Cualquier frase incorrecta o alteración falla con el mensaje requerido.
pub fn descifrar(contenido: &[u8], frase: &str) -> Result<Vec<u8>, ErrorDominio> {
    let envoltorio: ArchivoCifrado = serde_json::from_slice(contenido)
        .map_err(|_| ErrorDominio::nuevo("archivo", MENSAJE_FRASE_O_DANADO))?;

    if envoltorio.formato != FORMATO_CIFRADO {
        return Err(ErrorDominio::nuevo("archivo", MENSAJE_FRASE_O_DANADO));
    }

    if envoltorio.version != VERSION_FORMATO_ACTUAL {
        return Err(ErrorDominio::nuevo(
            "version",
            &format!(
                "Versión de formato no soportada: el archivo tiene la versión {} y esta aplicación soporta la versión {}.",
                envoltorio.version, VERSION_FORMATO_ACTUAL
            ),
        ));
    }

    if envoltorio.kdf.algoritmo != ALGORITMO_KDF || envoltorio.cifrado != ALGORITMO_CIFRADO {
        return Err(ErrorDominio::nuevo("archivo", MENSAJE_FRASE_O_DANADO));
    }

    let sal = B64
        .decode(&envoltorio.sal)
        .map_err(|_| ErrorDominio::nuevo("archivo", MENSAJE_FRASE_O_DANADO))?;
    let nonce_vec = B64
        .decode(&envoltorio.nonce)
        .map_err(|_| ErrorDominio::nuevo("archivo", MENSAJE_FRASE_O_DANADO))?;
    if sal.len() != TAMANO_SAL || nonce_vec.len() != TAMANO_NONCE {
        return Err(ErrorDominio::nuevo("archivo", MENSAJE_FRASE_O_DANADO));
    }
    let mut nonce_bytes = [0u8; TAMANO_NONCE];
    nonce_bytes.copy_from_slice(&nonce_vec);

    let mut datos_cifrados = B64
        .decode(&envoltorio.datos)
        .map_err(|_| ErrorDominio::nuevo("archivo", MENSAJE_FRASE_O_DANADO))?;

    let params = ParametrosKdf {
        memoria_kib: envoltorio.kdf.memoria_kib,
        iteraciones: envoltorio.kdf.iteraciones,
        paralelismo: envoltorio.kdf.paralelismo,
    };
    let clave = derivar_clave(frase, &sal, params)?;

    let aad = construir_aad(
        &envoltorio.formato,
        envoltorio.version,
        &envoltorio.kdf,
        &envoltorio.cifrado,
        &envoltorio.sal,
        &envoltorio.nonce,
    );

    let unbound = UnboundKey::new(&AES_256_GCM, clave.as_ref())
        .map_err(|_| ErrorDominio::nuevo("archivo", MENSAJE_FRASE_O_DANADO))?;
    let key = LessSafeKey::new(unbound);
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);

    let plano = key
        .open_in_place(nonce, Aad::from(&aad), &mut datos_cifrados)
        .map_err(|_| ErrorDominio::nuevo("frase", MENSAJE_FRASE_O_DANADO))?;

    Ok(plano.to_vec())
}

/// Detecta si el contenido de un archivo de respaldo está cifrado.
pub fn es_archivo_cifrado(contenido: &[u8]) -> bool {
    let Ok(valor) = serde_json::from_slice::<Value>(contenido) else {
        return false;
    };
    valor.get("formato").and_then(Value::as_str) == Some(FORMATO_CIFRADO)
}

/// Genera los bytes de exportación (cifrados o sin cifrar) sin modificar el libro (RF-INT-01).
pub fn exportar_bytes(
    repo: &impl RepositorioEventos,
    cifrado: bool,
    frase: Option<&str>,
    confirmacion: Option<&str>,
    params_kdf: ParametrosKdf,
) -> Result<(Vec<u8>, usize, String), ErrorDominio> {
    let eventos = repo
        .obtener_eventos()
        .map_err(|_| ErrorDominio::nuevo("almacenamiento", "No se pudo leer el libro local."))?;
    let ahora = Utc::now();
    let ndjson = serializar_eventos_ndjson(&eventos, ahora)?;
    let fecha_iso = ahora.to_rfc3339();

    if cifrado {
        let f = frase.unwrap_or("");
        let c = confirmacion.unwrap_or("");
        validar_frase_doble(f, c)?;
        let bytes = cifrar_con_parametros(ndjson.as_bytes(), f, params_kdf)?;
        Ok((bytes, eventos.len(), fecha_iso))
    } else {
        Ok((ndjson.into_bytes(), eventos.len(), fecha_iso))
    }
}

/// Valida, descifra (si está cifrado) e importa un respaldo en una sola transacción (Sección 2.4).
pub fn importar_bytes(
    repo: &mut RepositorioSqlite,
    contenido: &[u8],
    frase: Option<&str>,
) -> Result<ResultadoFusion, ErrorDominio> {
    let texto_claro = if es_archivo_cifrado(contenido) {
        let f = frase.unwrap_or("");
        if f.is_empty() {
            return Err(ErrorDominio::nuevo(
                "frase",
                "Este archivo está cifrado. Ingresa la frase de paso para descifrarlo.",
            ));
        }
        let descifrado = descifrar(contenido, f)?;
        String::from_utf8(descifrado)
            .map_err(|_| ErrorDominio::nuevo("archivo", MENSAJE_FRASE_O_DANADO))?
    } else {
        String::from_utf8(contenido.to_vec()).map_err(|_| {
            ErrorDominio::nuevo(
                "archivo",
                "El archivo no es un texto válido ni un respaldo cifrado reconocido.",
            )
        })?
    };

    let (_encabezado, eventos) = parsear_eventos_ndjson(&texto_claro)?;
    repo.fusionar_eventos(&eventos)
        .map_err(|e| ErrorDominio::nuevo("importacion", &e))
}

/// Escribe un archivo de forma atómica (temporal + rename) para evitar archivos a medio escribir.
pub fn escribir_archivo_atomico(ruta: &Path, contenido: &[u8]) -> Result<(), ErrorDominio> {
    if let Some(padre) = ruta.parent()
        && !padre.as_os_str().is_empty()
    {
        std::fs::create_dir_all(padre).map_err(|_| {
            ErrorDominio::nuevo(
                "ruta",
                "No se pudo crear la carpeta de destino para el archivo.",
            )
        })?;
    }
    let nombre = ruta
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("respaldo");
    let temporal = ruta.with_file_name(format!(".{nombre}.tmp-{}", uuid::Uuid::new_v4()));
    if std::fs::write(&temporal, contenido).is_err() {
        let _ = std::fs::remove_file(&temporal);
        return Err(ErrorDominio::nuevo(
            "ruta",
            "No se pudo escribir el archivo en la ruta indicada.",
        ));
    }
    if std::fs::rename(&temporal, ruta).is_err() {
        let _ = std::fs::remove_file(&temporal);
        return Err(ErrorDominio::nuevo(
            "ruta",
            "No se pudo guardar el archivo en la ruta indicada.",
        ));
    }
    Ok(())
}

/// Exporta el libro completo a un archivo en disco (cifrado o sin cifrar).
pub fn exportar_a_archivo(
    repo: &impl RepositorioEventos,
    ruta: &Path,
    cifrado: bool,
    frase: Option<&str>,
    confirmacion: Option<&str>,
) -> Result<ResumenExportacion, ErrorDominio> {
    let (bytes, cantidad_eventos, fecha_exportacion) = exportar_bytes(
        repo,
        cifrado,
        frase,
        confirmacion,
        ParametrosKdf::estandar(),
    )?;
    escribir_archivo_atomico(ruta, &bytes)?;
    Ok(ResumenExportacion {
        ruta: ruta.to_string_lossy().to_string(),
        cantidad_eventos,
        cifrado,
        fecha_exportacion,
    })
}

/// Importa un respaldo desde un archivo en disco, aplicando todos los eventos válidos en una sola transacción.
pub fn importar_desde_archivo(
    repo: &mut RepositorioSqlite,
    ruta: &Path,
    frase: Option<&str>,
) -> Result<ResultadoFusion, ErrorDominio> {
    let bytes = std::fs::read(ruta).map_err(|_| {
        ErrorDominio::nuevo(
            "ruta",
            "No se pudo abrir el archivo de copia de seguridad indicado.",
        )
    })?;
    importar_bytes(repo, &bytes, frase)
}
