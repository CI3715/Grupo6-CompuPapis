//! sincronizacion_personal.rs: Puerto TransporteSincronizacion (Git, Carpeta, Memoria),
//! almacén de secretos del SO (keyring) y motor de sincronización incremental entre
//! dispositivos propios sin intervención del servidor (RF-INT-03 a RF-INT-07, RNF-10, RNF-14, RNF-15).

use crate::domain::{ErrorDominio, EventoRechazado, RepositorioEventos};
use crate::respaldo::{
    ParametrosKdf, cifrar_con_parametros, descifrar, escribir_archivo_atomico,
    parsear_eventos_ndjson, serializar_eventos_ndjson, validar_frase_minima,
};
use crate::sqlite_repo::RepositorioSqlite;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

const SERVICIO_KEYRING: &str = "cuentas-claras-sync-personal";
const CLAVE_CONFIG_SYNC: &str = "sync_personal_config";
const CLAVE_ULTIMA_SYNC: &str = "sync_personal_ultima";
const CLAVE_SECRETO_FRASE: &str = "frase_paso_sync";
const CLAVE_SECRETO_GIT_TOKEN: &str = "git_credencial_token";
const EXTENSION_LOTE: &str = ".ccsync";
const MAX_REINTENTOS_PUBLICACION: usize = 5;

// =========================================================
// PUERTO DE ALMACÉN DE SECRETOS DEL SISTEMA OPERATIVO
// =========================================================

pub trait AlmacenSecretos {
    fn guardar(&self, clave: &str, valor: &str) -> Result<(), ErrorDominio>;
    fn leer(&self, clave: &str) -> Result<Option<String>, ErrorDominio>;
    fn eliminar(&self, clave: &str) -> Result<(), ErrorDominio>;
}

/// Adaptador real respaldado por el almacén de secretos del sistema operativo (keyring).
pub struct AlmacenSecretosSo {
    servicio: String,
}

impl AlmacenSecretosSo {
    pub fn nuevo(prefijo_instancia: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(prefijo_instancia.as_bytes());
        let sufijo = format!("{:x}", hasher.finalize());
        Self {
            servicio: format!("{SERVICIO_KEYRING}-{}", &sufijo[..12]),
        }
    }
}

impl AlmacenSecretos for AlmacenSecretosSo {
    fn guardar(&self, clave: &str, valor: &str) -> Result<(), ErrorDominio> {
        let entry = keyring::Entry::new(&self.servicio, clave).map_err(|_| {
            ErrorDominio::nuevo(
                "secretos",
                "No se pudo acceder al almacén de secretos del sistema operativo.",
            )
        })?;
        entry.set_password(valor).map_err(|_| {
            ErrorDominio::nuevo(
                "secretos",
                "No se pudo guardar el secreto en el almacén del sistema operativo. Verifica que el llavero del sistema esté desbloqueado.",
            )
        })
    }

    fn leer(&self, clave: &str) -> Result<Option<String>, ErrorDominio> {
        let entry = keyring::Entry::new(&self.servicio, clave).map_err(|_| {
            ErrorDominio::nuevo(
                "secretos",
                "No se pudo acceder al almacén de secretos del sistema operativo.",
            )
        })?;
        match entry.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(ErrorDominio::nuevo(
                "secretos",
                "No se pudo leer el secreto desde el almacén del sistema operativo.",
            )),
        }
    }

    fn eliminar(&self, clave: &str) -> Result<(), ErrorDominio> {
        let entry = keyring::Entry::new(&self.servicio, clave).map_err(|_| {
            ErrorDominio::nuevo(
                "secretos",
                "No se pudo acceder al almacén de secretos del sistema operativo.",
            )
        })?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(ErrorDominio::nuevo(
                "secretos",
                "No se pudo eliminar el secreto del almacén del sistema operativo.",
            )),
        }
    }
}

/// Adaptador en memoria para pruebas unitarias e integración sin depender del llavero del SO.
#[derive(Default, Clone)]
pub struct AlmacenSecretosMemoria {
    datos: Arc<Mutex<HashMap<String, String>>>,
}

impl AlmacenSecretos for AlmacenSecretosMemoria {
    fn guardar(&self, clave: &str, valor: &str) -> Result<(), ErrorDominio> {
        self.datos
            .lock()
            .map_err(|_| ErrorDominio::nuevo("secretos", "Error interno en almacén de secretos."))?
            .insert(clave.to_string(), valor.to_string());
        Ok(())
    }

    fn leer(&self, clave: &str) -> Result<Option<String>, ErrorDominio> {
        Ok(self
            .datos
            .lock()
            .map_err(|_| ErrorDominio::nuevo("secretos", "Error interno en almacén de secretos."))?
            .get(clave)
            .cloned())
    }

    fn eliminar(&self, clave: &str) -> Result<(), ErrorDominio> {
        self.datos
            .lock()
            .map_err(|_| ErrorDominio::nuevo("secretos", "Error interno en almacén de secretos."))?
            .remove(clave);
        Ok(())
    }
}

// =========================================================
// PUERTO TransporteSincronizacion (RNF-14, RNF-15)
// =========================================================

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ReferenciaLote {
    pub dispositivo_id: String,
    pub nombre_archivo: String,
}

impl ReferenciaLote {
    pub fn clave(&self) -> String {
        format!("{}/{}", self.dispositivo_id, self.nombre_archivo)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultadoPublicacion {
    Publicado,
    /// El remoto avanzó concurrentemente (Git): el motor debe volver al paso 1 y reintentar.
    RemotoActualizado,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorTransporte {
    DestinoInaccesible(String),
    CredencialesInvalidas,
    ErrorOperacion(String),
}

impl ErrorTransporte {
    pub fn a_error_dominio(&self) -> ErrorDominio {
        match self {
            Self::DestinoInaccesible(detalle) => ErrorDominio::nuevo("destino", detalle),
            Self::CredencialesInvalidas => ErrorDominio::nuevo(
                "credenciales",
                "Las credenciales de Git fueron rechazadas. Actualízalas en la configuración de sincronización y vuelve a intentarlo.",
            ),
            Self::ErrorOperacion(msg) => ErrorDominio::nuevo("sincronizacion", msg),
        }
    }
}

/// Puerto hexagonal para destinos de sincronización personal (Git, Carpeta, Memoria).
/// Agregar un destino nuevo solo requiere implementar este trait, sin tocar el dominio.
pub trait TransporteSincronizacion {
    fn preparar(&mut self) -> Result<(), ErrorTransporte>;
    fn listar_lotes(&self) -> Result<Vec<ReferenciaLote>, ErrorTransporte>;
    fn leer_lote(&self, lote: &ReferenciaLote) -> Result<Vec<u8>, ErrorTransporte>;
    fn escribir_lote(
        &mut self,
        dispositivo_id: &str,
        nombre_archivo: &str,
        contenido: &[u8],
    ) -> Result<(), ErrorTransporte>;
    fn publicar(&mut self, mensaje: &str) -> Result<ResultadoPublicacion, ErrorTransporte>;
}

fn validar_segmento_seguro(segmento: &str) -> Result<(), ErrorTransporte> {
    if segmento.is_empty()
        || segmento.contains('/')
        || segmento.contains('\\')
        || segmento.contains("..")
        || segmento.starts_with('.')
    {
        return Err(ErrorTransporte::ErrorOperacion(
            "Nombre de dispositivo o lote inválido.".into(),
        ));
    }
    Ok(())
}

// =========================================================
// ADAPTADOR 1: TRANSPORTE EN MEMORIA (PRUEBAS)
// =========================================================

#[derive(Default)]
struct EstadoMemoriaCompartida {
    /// dispositivo_id -> (nombre_archivo -> contenido cifrado)
    lotes: BTreeMap<String, BTreeMap<String, Vec<u8>>>,
    inaccesible: bool,
    credenciales_invalidas: bool,
    simular_remoto_actualizado_veces: usize,
}

#[derive(Clone, Default)]
pub struct TransporteMemoria {
    estado: Arc<Mutex<EstadoMemoriaCompartida>>,
}

impl TransporteMemoria {
    pub fn nuevo() -> Self {
        Self::default()
    }

    pub fn establecer_inaccesible(&self, valor: bool) {
        if let Ok(mut e) = self.estado.lock() {
            e.inaccesible = valor;
        }
    }

    pub fn establecer_credenciales_invalidas(&self, valor: bool) {
        if let Ok(mut e) = self.estado.lock() {
            e.credenciales_invalidas = valor;
        }
    }

    pub fn simular_conflicto_remoto(&self, veces: usize) {
        if let Ok(mut e) = self.estado.lock() {
            e.simular_remoto_actualizado_veces = veces;
        }
    }

    pub fn total_lotes(&self) -> usize {
        self.estado
            .lock()
            .map(|e| e.lotes.values().map(BTreeMap::len).sum())
            .unwrap_or(0)
    }

    pub fn todos_los_bytes(&self) -> Vec<Vec<u8>> {
        self.estado
            .lock()
            .map(|e| e.lotes.values().flat_map(|m| m.values().cloned()).collect())
            .unwrap_or_default()
    }
}

impl TransporteSincronizacion for TransporteMemoria {
    fn preparar(&mut self) -> Result<(), ErrorTransporte> {
        let e = self
            .estado
            .lock()
            .map_err(|_| ErrorTransporte::ErrorOperacion("Error interno de memoria.".into()))?;
        if e.credenciales_invalidas {
            return Err(ErrorTransporte::CredencialesInvalidas);
        }
        if e.inaccesible {
            return Err(ErrorTransporte::DestinoInaccesible(
                "No se pudo llegar al repositorio o a la carpeta; se reintentará.".into(),
            ));
        }
        Ok(())
    }

    fn listar_lotes(&self) -> Result<Vec<ReferenciaLote>, ErrorTransporte> {
        let e = self
            .estado
            .lock()
            .map_err(|_| ErrorTransporte::ErrorOperacion("Error interno de memoria.".into()))?;
        if e.inaccesible {
            return Err(ErrorTransporte::DestinoInaccesible(
                "No se pudo llegar al repositorio o a la carpeta; se reintentará.".into(),
            ));
        }
        let mut salida = Vec::new();
        for (disp, archivos) in &e.lotes {
            for nombre in archivos.keys() {
                salida.push(ReferenciaLote {
                    dispositivo_id: disp.clone(),
                    nombre_archivo: nombre.clone(),
                });
            }
        }
        Ok(salida)
    }

    fn leer_lote(&self, lote: &ReferenciaLote) -> Result<Vec<u8>, ErrorTransporte> {
        let e = self
            .estado
            .lock()
            .map_err(|_| ErrorTransporte::ErrorOperacion("Error interno de memoria.".into()))?;
        e.lotes
            .get(&lote.dispositivo_id)
            .and_then(|m| m.get(&lote.nombre_archivo))
            .cloned()
            .ok_or_else(|| {
                ErrorTransporte::ErrorOperacion("No se encontró el lote solicitado.".into())
            })
    }

    fn escribir_lote(
        &mut self,
        dispositivo_id: &str,
        nombre_archivo: &str,
        contenido: &[u8],
    ) -> Result<(), ErrorTransporte> {
        validar_segmento_seguro(dispositivo_id)?;
        validar_segmento_seguro(nombre_archivo)?;
        let mut e = self
            .estado
            .lock()
            .map_err(|_| ErrorTransporte::ErrorOperacion("Error interno de memoria.".into()))?;
        if e.inaccesible {
            return Err(ErrorTransporte::DestinoInaccesible(
                "No se pudo llegar al repositorio o a la carpeta; se reintentará.".into(),
            ));
        }
        let carpeta = e.lotes.entry(dispositivo_id.to_string()).or_default();
        if carpeta.contains_key(nombre_archivo) {
            return Err(ErrorTransporte::ErrorOperacion(
                "Los archivos de lote son inmutables y no pueden sobrescribirse.".into(),
            ));
        }
        carpeta.insert(nombre_archivo.to_string(), contenido.to_vec());
        Ok(())
    }

    fn publicar(&mut self, _mensaje: &str) -> Result<ResultadoPublicacion, ErrorTransporte> {
        let mut e = self
            .estado
            .lock()
            .map_err(|_| ErrorTransporte::ErrorOperacion("Error interno de memoria.".into()))?;
        if e.credenciales_invalidas {
            return Err(ErrorTransporte::CredencialesInvalidas);
        }
        if e.inaccesible {
            return Err(ErrorTransporte::DestinoInaccesible(
                "No se pudo llegar al repositorio o a la carpeta; se reintentará.".into(),
            ));
        }
        if e.simular_remoto_actualizado_veces > 0 {
            e.simular_remoto_actualizado_veces -= 1;
            return Ok(ResultadoPublicacion::RemotoActualizado);
        }
        Ok(ResultadoPublicacion::Publicado)
    }
}

// =========================================================
// ADAPTADOR 2: TRANSPORTE CARPETA COMPARTIDA (Syncthing, Dropbox, Drive)
// =========================================================

pub struct TransporteCarpeta {
    raiz: PathBuf,
}

impl TransporteCarpeta {
    pub fn nuevo(raiz: PathBuf) -> Self {
        Self { raiz }
    }

    fn dir_dispositivos(&self) -> PathBuf {
        self.raiz.join("dispositivos")
    }
}

fn listar_lotes_en_carpeta(
    dir_dispositivos: &Path,
) -> Result<Vec<ReferenciaLote>, ErrorTransporte> {
    if !dir_dispositivos.exists() {
        return Ok(Vec::new());
    }
    let mut salida = Vec::new();
    let entradas_disp = std::fs::read_dir(dir_dispositivos).map_err(|_| {
        ErrorTransporte::DestinoInaccesible(
            "No se pudo leer la carpeta de sincronización; se reintentará.".into(),
        )
    })?;

    for ent in entradas_disp {
        let ent = ent.map_err(|_| {
            ErrorTransporte::DestinoInaccesible(
                "No se pudo leer la carpeta de sincronización; se reintentará.".into(),
            )
        })?;
        let path_disp = ent.path();
        if !path_disp.is_dir() {
            continue;
        }
        let Some(dispositivo_id) = ent.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if dispositivo_id.starts_with('.') {
            continue;
        }

        let archivos = std::fs::read_dir(&path_disp).map_err(|_| {
            ErrorTransporte::DestinoInaccesible(
                "No se pudieron leer todos los lotes de sincronización; se reintentará.".into(),
            )
        })?;
        for arch in archivos {
            let arch = arch.map_err(|_| {
                ErrorTransporte::DestinoInaccesible(
                    "No se pudieron leer todos los lotes de sincronización; se reintentará.".into(),
                )
            })?;
            let path_arch = arch.path();
            if !path_arch.is_file() {
                continue;
            }
            let Some(nombre) = arch.file_name().to_str().map(str::to_string) else {
                continue;
            };
            // Ignora archivos temporales a medio escribir (.tmp-*) o archivos ocultos
            if nombre.starts_with('.') || !nombre.ends_with(EXTENSION_LOTE) {
                continue;
            }
            salida.push(ReferenciaLote {
                dispositivo_id: dispositivo_id.clone(),
                nombre_archivo: nombre,
            });
        }
    }

    salida.sort();
    Ok(salida)
}

fn escribir_lote_atomico_en_carpeta(
    dir_dispositivos: &Path,
    dispositivo_id: &str,
    nombre_archivo: &str,
    contenido: &[u8],
) -> Result<(), ErrorTransporte> {
    validar_segmento_seguro(dispositivo_id)?;
    validar_segmento_seguro(nombre_archivo)?;
    let carpeta_disp = dir_dispositivos.join(dispositivo_id);
    std::fs::create_dir_all(&carpeta_disp).map_err(|_| {
        ErrorTransporte::DestinoInaccesible(
            "No se pudo acceder a la carpeta de destino; se reintentará.".into(),
        )
    })?;
    let destino = carpeta_disp.join(nombre_archivo);
    if destino.exists() {
        return Err(ErrorTransporte::ErrorOperacion(
            "Los archivos de lote son inmutables y no pueden sobrescribirse.".into(),
        ));
    }
    // Escritura atómica: archivo temporal oculto + rename (Sección 2.5.2)
    escribir_archivo_atomico(&destino, contenido).map_err(|_| {
        ErrorTransporte::DestinoInaccesible(
            "No se pudo escribir el lote en la carpeta de destino; se reintentará.".into(),
        )
    })
}

impl TransporteSincronizacion for TransporteCarpeta {
    fn preparar(&mut self) -> Result<(), ErrorTransporte> {
        if !self.raiz.exists() || !self.raiz.is_dir() {
            return Err(ErrorTransporte::DestinoInaccesible(
                "No se pudo llegar a la carpeta compartida configurada; verifica que exista y sea accesible. Se reintentará."
                    .into(),
            ));
        }
        std::fs::create_dir_all(self.dir_dispositivos()).map_err(|_| {
            ErrorTransporte::DestinoInaccesible(
                "No se pudo preparar la carpeta compartida; verifica los permisos. Se reintentará."
                    .into(),
            )
        })?;
        Ok(())
    }

    fn listar_lotes(&self) -> Result<Vec<ReferenciaLote>, ErrorTransporte> {
        if !self.raiz.exists() {
            return Err(ErrorTransporte::DestinoInaccesible(
                "No se pudo llegar a la carpeta compartida; se reintentará.".into(),
            ));
        }
        listar_lotes_en_carpeta(&self.dir_dispositivos())
    }

    fn leer_lote(&self, lote: &ReferenciaLote) -> Result<Vec<u8>, ErrorTransporte> {
        validar_segmento_seguro(&lote.dispositivo_id)?;
        validar_segmento_seguro(&lote.nombre_archivo)?;
        let ruta = self
            .dir_dispositivos()
            .join(&lote.dispositivo_id)
            .join(&lote.nombre_archivo);
        std::fs::read(&ruta).map_err(|_| {
            ErrorTransporte::DestinoInaccesible(
                "No se pudo leer un lote desde la carpeta compartida; se reintentará.".into(),
            )
        })
    }

    fn escribir_lote(
        &mut self,
        dispositivo_id: &str,
        nombre_archivo: &str,
        contenido: &[u8],
    ) -> Result<(), ErrorTransporte> {
        if !self.raiz.exists() {
            return Err(ErrorTransporte::DestinoInaccesible(
                "No se pudo llegar a la carpeta compartida; se reintentará.".into(),
            ));
        }
        escribir_lote_atomico_en_carpeta(
            &self.dir_dispositivos(),
            dispositivo_id,
            nombre_archivo,
            contenido,
        )
    }

    fn publicar(&mut self, _mensaje: &str) -> Result<ResultadoPublicacion, ErrorTransporte> {
        // En carpeta compartida los archivos ya fueron renombrados atómicamente;
        // el servicio externo (Syncthing, Dropbox, Drive) los propaga.
        Ok(ResultadoPublicacion::Publicado)
    }
}

// =========================================================
// ADAPTADOR 3: TRANSPORTE GIT (CLI git, credenciales efímeras via GIT_ASKPASS)
// =========================================================

pub struct CredencialesGit {
    pub usuario: String,
    pub secreto: String,
}

pub struct TransporteGit {
    url_remota: String,
    rama: String,
    directorio_clon: PathBuf,
    credenciales: Option<CredencialesGit>,
}

impl TransporteGit {
    pub fn nuevo(
        url_remota: String,
        rama: String,
        directorio_clon: PathBuf,
        credenciales: Option<CredencialesGit>,
    ) -> Self {
        let rama_limpia = if rama.trim().is_empty() {
            "main".to_string()
        } else {
            rama.trim().to_string()
        };
        Self {
            url_remota: url_remota.trim().to_string(),
            rama: rama_limpia,
            directorio_clon,
            credenciales,
        }
    }

    fn dir_dispositivos(&self) -> PathBuf {
        self.directorio_clon.join("dispositivos")
    }

    /// Ejecuta un comando `git` pasando credenciales solo por memoria/entorno efímero (GIT_ASKPASS),
    /// jamás guardando credenciales en `.git/config`, en la URL ni en archivos persistentes.
    fn ejecutar_git(
        &self,
        cwd: Option<&Path>,
        args: &[&str],
    ) -> Result<(bool, String, String), ErrorTransporte> {
        let mut cmd = Command::new("git");
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        cmd.args(args);
        cmd.env("GIT_TERMINAL_PROMPT", "0");
        // Evita prompts interactivos de SSH o helpers externos si ya se proveen credenciales
        cmd.env("GCM_INTERACTIVE", "never");

        let mut script_temporal: Option<PathBuf> = None;
        if let Some(creds) = &self.credenciales {
            let ruta_askpass = crear_script_askpass_temporal()?;
            cmd.env("GIT_ASKPASS", &ruta_askpass);
            cmd.env("CC_GIT_USER", &creds.usuario);
            cmd.env("CC_GIT_PASS", &creds.secreto);
            script_temporal = Some(ruta_askpass);
        }

        let salida = cmd.output();
        if let Some(tmp) = script_temporal {
            let _ = std::fs::remove_file(tmp);
        }

        let output = salida.map_err(|_| {
            ErrorTransporte::ErrorOperacion(
                "No se encontró Git instalado en este dispositivo. Instala Git para sincronizar con un repositorio."
                    .into(),
            )
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Ok((output.status.success(), stdout, stderr))
    }

    fn clasificar_error_git(&self, stderr: &str) -> ErrorTransporte {
        let lower = stderr.to_lowercase();
        if lower.contains("authentication failed")
            || lower.contains("invalid username")
            || lower.contains("could not read username")
            || lower.contains("could not read password")
            || lower.contains("permission denied (publickey")
            || lower.contains("returned error: 401")
            || lower.contains("returned error: 403")
            || lower.contains("access denied")
        {
            return ErrorTransporte::CredencialesInvalidas;
        }
        ErrorTransporte::DestinoInaccesible(
            "No se pudo llegar al repositorio Git; verifica tu conexión y la dirección configurada. Se reintentará."
                .into(),
        )
    }

    fn rama_remota_existe(&self) -> Result<bool, ErrorTransporte> {
        let ref_remota = format!("refs/remotes/origin/{}", self.rama);
        let (ok, _, _) = self.ejecutar_git(
            Some(&self.directorio_clon),
            &["rev-parse", "--verify", &ref_remota],
        )?;
        Ok(ok)
    }

    fn head_tiene_commits(&self) -> Result<bool, ErrorTransporte> {
        let (ok, _, _) = self.ejecutar_git(
            Some(&self.directorio_clon),
            &["rev-parse", "--verify", "HEAD"],
        )?;
        Ok(ok)
    }
}

fn crear_script_askpass_temporal() -> Result<PathBuf, ErrorTransporte> {
    let dir = std::env::temp_dir();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let ruta = dir.join(format!("cc_askpass_{}.sh", uuid::Uuid::new_v4()));
        let contenido = "#!/bin/sh\ncase \"$1\" in\n  *Username*|*username*) printf '%s\\n' \"$CC_GIT_USER\" ;;\n  *) printf '%s\\n' \"$CC_GIT_PASS\" ;;\nesac\n";
        std::fs::write(&ruta, contenido).map_err(|_| {
            ErrorTransporte::ErrorOperacion("No se pudo preparar la autenticación de Git.".into())
        })?;
        std::fs::set_permissions(&ruta, std::fs::Permissions::from_mode(0o700)).map_err(|_| {
            ErrorTransporte::ErrorOperacion("No se pudo preparar la autenticación de Git.".into())
        })?;
        Ok(ruta)
    }
    #[cfg(windows)]
    {
        let ruta = dir.join(format!("cc_askpass_{}.cmd", uuid::Uuid::new_v4()));
        let contenido = "@echo off\r\necho %~1 | findstr /I \"username\" >nul\r\nif %errorlevel%==0 (\r\n  echo %CC_GIT_USER%\r\n) else (\r\n  echo %CC_GIT_PASS%\r\n)\r\n";
        std::fs::write(&ruta, contenido).map_err(|_| {
            ErrorTransporte::ErrorOperacion("No se pudo preparar la autenticación de Git.".into())
        })?;
        Ok(ruta)
    }
}

impl TransporteSincronizacion for TransporteGit {
    fn preparar(&mut self) -> Result<(), ErrorTransporte> {
        if self.url_remota.is_empty() {
            return Err(ErrorTransporte::DestinoInaccesible(
                "Configura la dirección del repositorio Git antes de sincronizar.".into(),
            ));
        }

        if let Some(padre) = self.directorio_clon.parent() {
            let _ = std::fs::create_dir_all(padre);
        }

        let git_dir = self.directorio_clon.join(".git");
        if !git_dir.exists() {
            if self.directorio_clon.exists() {
                return Err(ErrorTransporte::ErrorOperacion(
                    "La carpeta local de sincronización ya existe y no es un repositorio Git. Revisa su contenido antes de continuar.".into(),
                ));
            }
            let ruta_str = self.directorio_clon.to_string_lossy().to_string();
            let (ok, _, stderr) =
                self.ejecutar_git(None, &["clone", &self.url_remota, &ruta_str])?;
            if !ok {
                return Err(self.clasificar_error_git(&stderr));
            }
            // Asegurar que la rama local activa sea la configurada
            let _ = self.ejecutar_git(Some(&self.directorio_clon), &["checkout", "-B", &self.rama]);
            if self.rama_remota_existe()? {
                let origen = format!("origin/{}", self.rama);
                let _ =
                    self.ejecutar_git(Some(&self.directorio_clon), &["reset", "--hard", &origen]);
            }
        } else {
            // Actualizar URL remota por si el usuario la cambió (sin credenciales incrustadas)
            let _ = self.ejecutar_git(
                Some(&self.directorio_clon),
                &["remote", "set-url", "origin", &self.url_remota],
            );

            let (ok_fetch, _, stderr_fetch) =
                self.ejecutar_git(Some(&self.directorio_clon), &["fetch", "origin"])?;
            if !ok_fetch {
                return Err(self.clasificar_error_git(&stderr_fetch));
            }

            if self.rama_remota_existe()? {
                let origen = format!("origin/{}", self.rama);
                if self.head_tiene_commits()? {
                    let (ok_rebase, _, _) =
                        self.ejecutar_git(Some(&self.directorio_clon), &["rebase", &origen])?;
                    if !ok_rebase {
                        let _ =
                            self.ejecutar_git(Some(&self.directorio_clon), &["rebase", "--abort"]);
                        // Fallback limpio: merge no destructivo (los lotes viven en archivos únicos por dispositivo)
                        let (ok_merge, _, stderr_merge) = self.ejecutar_git(
                            Some(&self.directorio_clon),
                            &[
                                "-c",
                                "user.name=Cuentas Claras",
                                "-c",
                                "user.email=sync@cuentas-claras.local",
                                "merge",
                                "--no-edit",
                                &origen,
                            ],
                        )?;
                        if !ok_merge {
                            return Err(self.clasificar_error_git(&stderr_merge));
                        }
                    }
                } else {
                    let _ = self.ejecutar_git(
                        Some(&self.directorio_clon),
                        &["checkout", "-B", &self.rama, &origen],
                    );
                }
            }
        }

        std::fs::create_dir_all(self.dir_dispositivos()).map_err(|_| {
            ErrorTransporte::ErrorOperacion(
                "No se pudo preparar la carpeta local del repositorio Git.".into(),
            )
        })?;
        Ok(())
    }

    fn listar_lotes(&self) -> Result<Vec<ReferenciaLote>, ErrorTransporte> {
        listar_lotes_en_carpeta(&self.dir_dispositivos())
    }

    fn leer_lote(&self, lote: &ReferenciaLote) -> Result<Vec<u8>, ErrorTransporte> {
        validar_segmento_seguro(&lote.dispositivo_id)?;
        validar_segmento_seguro(&lote.nombre_archivo)?;
        let ruta = self
            .dir_dispositivos()
            .join(&lote.dispositivo_id)
            .join(&lote.nombre_archivo);
        std::fs::read(&ruta).map_err(|_| {
            ErrorTransporte::DestinoInaccesible(
                "No se pudo leer el lote desde el repositorio local.".into(),
            )
        })
    }

    fn escribir_lote(
        &mut self,
        dispositivo_id: &str,
        nombre_archivo: &str,
        contenido: &[u8],
    ) -> Result<(), ErrorTransporte> {
        escribir_lote_atomico_en_carpeta(
            &self.dir_dispositivos(),
            dispositivo_id,
            nombre_archivo,
            contenido,
        )
    }

    fn publicar(&mut self, mensaje: &str) -> Result<ResultadoPublicacion, ErrorTransporte> {
        let (ok_add, _, stderr_add) =
            self.ejecutar_git(Some(&self.directorio_clon), &["add", "dispositivos"])?;
        if !ok_add {
            return Err(ErrorTransporte::ErrorOperacion(format!(
                "No se pudieron preparar los archivos para publicar en Git: {}",
                stderr_add.trim()
            )));
        }

        let (ok_status, stdout_status, _) =
            self.ejecutar_git(Some(&self.directorio_clon), &["status", "--porcelain"])?;
        if ok_status && !stdout_status.trim().is_empty() {
            let (ok_commit, _, stderr_commit) = self.ejecutar_git(
                Some(&self.directorio_clon),
                &[
                    "-c",
                    "user.name=Cuentas Claras",
                    "-c",
                    "user.email=sync@cuentas-claras.local",
                    "commit",
                    "-m",
                    mensaje,
                ],
            )?;
            if !ok_commit {
                return Err(ErrorTransporte::ErrorOperacion(format!(
                    "No se pudo confirmar el lote local en Git: {}",
                    stderr_commit.trim()
                )));
            }
        }

        if !self.head_tiene_commits()? {
            return Ok(ResultadoPublicacion::Publicado);
        }

        let refspec = format!("HEAD:{}", self.rama);
        let (ok_push, _, stderr_push) = self.ejecutar_git(
            Some(&self.directorio_clon),
            &["push", "-u", "origin", &refspec],
        )?;
        if ok_push {
            return Ok(ResultadoPublicacion::Publicado);
        }

        let lower = stderr_push.to_lowercase();
        if lower.contains("non-fast-forward")
            || lower.contains("fetch first")
            || lower.contains("[rejected]")
            || lower.contains("failed to push some refs")
        {
            return Ok(ResultadoPublicacion::RemotoActualizado);
        }

        Err(self.clasificar_error_git(&stderr_push))
    }
}

// =========================================================
// CONFIGURACIÓN Y MOTOR DE SINCRONIZACIÓN INCREMENTAL
// =========================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TipoDestinoSync {
    Carpeta,
    Git,
}

/// Configuración pública del destino de sincronización personal.
/// NUNCA contiene la frase de paso ni contraseñas/tokens de Git (van en AlmacenSecretos).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigSyncPersonal {
    pub tipo: TipoDestinoSync,
    /// Ruta de carpeta compartida o URL del repositorio Git (sin credenciales).
    pub destino: String,
    /// Rama de Git (solo aplica si tipo == Git).
    pub rama: Option<String>,
    /// Usuario de Git opcional (el token/contraseña vive en el almacén de secretos del SO).
    pub usuario_git: Option<String>,
}

impl ConfigSyncPersonal {
    pub fn clave_destino(&self) -> String {
        let mut hasher = Sha256::new();
        let prefijo = match self.tipo {
            TipoDestinoSync::Carpeta => "carpeta",
            TipoDestinoSync::Git => "git",
        };
        hasher.update(prefijo.as_bytes());
        hasher.update(b":");
        hasher.update(self.destino.trim().as_bytes());
        hasher.update(b":");
        hasher.update(self.rama.as_deref().unwrap_or("main").trim().as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstadoConfigSyncPersonal {
    pub configurado: bool,
    pub dispositivo_id: String,
    pub tipo: Option<TipoDestinoSync>,
    pub destino: Option<String>,
    pub rama: Option<String>,
    pub usuario_git: Option<String>,
    pub tiene_frase: bool,
    pub tiene_credencial_git: bool,
    pub ultima_sincronizacion: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InformeSyncPersonal {
    pub enviados: usize,
    pub recibidos: usize,
    pub ya_existian: usize,
    pub rechazados: Vec<EventoRechazado>,
    pub ultima_sincronizacion: String,
}

/// Rechaza URLs de Git que contengan credenciales incrustadas (`https://user:pass@...`).
pub fn validar_url_sin_credenciales(url: &str) -> Result<(), ErrorDominio> {
    if let Some(pos_esquema) = url.find("://") {
        let resto = &url[pos_esquema + 3..];
        let autoridad = resto.split('/').next().unwrap_or("");
        if autoridad.contains('@')
            && (!url.starts_with("ssh://")
                || autoridad.split('@').next().unwrap_or("").contains(':'))
        {
            return Err(ErrorDominio::nuevo(
                "destino",
                "No incluyas usuario ni contraseña en la URL del repositorio. Usa los campos de credenciales para guardarlos de forma segura en el sistema operativo.",
            ));
        }
    }
    Ok(())
}

pub fn guardar_configuracion_sync(
    repo: &RepositorioSqlite,
    secretos: &dyn AlmacenSecretos,
    config: &ConfigSyncPersonal,
    frase: Option<&str>,
    credencial_git: Option<&str>,
) -> Result<EstadoConfigSyncPersonal, ErrorDominio> {
    if config.destino.trim().is_empty() {
        return Err(ErrorDominio::nuevo(
            "destino",
            "Indica la ruta de la carpeta compartida o la dirección del repositorio Git.",
        ));
    }

    if config.tipo == TipoDestinoSync::Git {
        validar_url_sin_credenciales(&config.destino)?;
    }

    // Validar y guardar frase de paso en el almacén de secretos del SO
    if let Some(f) = frase
        && !f.is_empty()
    {
        validar_frase_minima(f)?;
        secretos.guardar(CLAVE_SECRETO_FRASE, f)?;
    } else if secretos.leer(CLAVE_SECRETO_FRASE)?.is_none() {
        return Err(ErrorDominio::nuevo(
            "frase",
            "Ingresa una frase de paso de al menos 12 caracteres para cifrar la sincronización.",
        ));
    }

    if let Some(tok) = credencial_git {
        if tok.trim().is_empty() {
            secretos.eliminar(CLAVE_SECRETO_GIT_TOKEN)?;
        } else {
            secretos.guardar(CLAVE_SECRETO_GIT_TOKEN, tok)?;
        }
    }

    let limpia = ConfigSyncPersonal {
        tipo: config.tipo.clone(),
        destino: config.destino.trim().to_string(),
        rama: config
            .rama
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        usuario_git: config
            .usuario_git
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
    };

    let json = serde_json::to_string(&limpia)
        .map_err(|_| ErrorDominio::nuevo("general", "No se pudo guardar la configuración."))?;
    repo.guardar_config_local(CLAVE_CONFIG_SYNC, &json)
        .map_err(|e| ErrorDominio::nuevo("almacenamiento", &e))?;

    obtener_estado_config_sync(repo, secretos)
}

pub fn obtener_configuracion_sync(
    repo: &RepositorioSqlite,
) -> Result<Option<ConfigSyncPersonal>, ErrorDominio> {
    let raw = repo
        .obtener_config_local(CLAVE_CONFIG_SYNC)
        .map_err(|e| ErrorDominio::nuevo("almacenamiento", &e))?;
    match raw {
        Some(texto) => {
            let cfg: ConfigSyncPersonal = serde_json::from_str(&texto).map_err(|_| {
                ErrorDominio::nuevo(
                    "almacenamiento",
                    "La configuración de sincronización guardada no es válida.",
                )
            })?;
            Ok(Some(cfg))
        }
        None => Ok(None),
    }
}

pub fn obtener_estado_config_sync(
    repo: &RepositorioSqlite,
    secretos: &dyn AlmacenSecretos,
) -> Result<EstadoConfigSyncPersonal, ErrorDominio> {
    let dispositivo_id = repo
        .dispositivo_id()
        .map_err(|e| ErrorDominio::nuevo("almacenamiento", &e))?;
    let cfg = obtener_configuracion_sync(repo)?;
    let ultima = repo
        .obtener_config_local(CLAVE_ULTIMA_SYNC)
        .map_err(|e| ErrorDominio::nuevo("almacenamiento", &e))?;
    let tiene_frase = secretos.leer(CLAVE_SECRETO_FRASE).ok().flatten().is_some();
    let tiene_credencial_git = secretos
        .leer(CLAVE_SECRETO_GIT_TOKEN)
        .ok()
        .flatten()
        .is_some();

    Ok(EstadoConfigSyncPersonal {
        configurado: cfg.is_some() && tiene_frase,
        dispositivo_id,
        tipo: cfg.as_ref().map(|c| c.tipo.clone()),
        destino: cfg.as_ref().map(|c| c.destino.clone()),
        rama: cfg.as_ref().and_then(|c| c.rama.clone()),
        usuario_git: cfg.as_ref().and_then(|c| c.usuario_git.clone()),
        tiene_frase,
        tiene_credencial_git,
        ultima_sincronizacion: ultima,
    })
}

pub fn limpiar_configuracion_sync(
    repo: &RepositorioSqlite,
    secretos: &dyn AlmacenSecretos,
) -> Result<(), ErrorDominio> {
    repo.eliminar_config_local(CLAVE_CONFIG_SYNC)
        .map_err(|e| ErrorDominio::nuevo("almacenamiento", &e))?;
    let _ = secretos.eliminar(CLAVE_SECRETO_FRASE);
    let _ = secretos.eliminar(CLAVE_SECRETO_GIT_TOKEN);
    Ok(())
}

/// Motor de sincronización de 4 pasos (Sección 2.5.3):
/// 1. Traer lotes no leídos
/// 2. Fusionar eventos descifrados (Sección 2.1)
/// 3. Enviar únicamente los eventos locales que faltan en un lote cifrado nuevo (RNF-10)
/// 4. Publicar (si el remoto cambió en Git, vuelve al paso 1 y reintenta)
pub fn ejecutar_sincronizacion_personal(
    repo: &mut RepositorioSqlite,
    transporte: &mut dyn TransporteSincronizacion,
    destino_clave: &str,
    frase: &str,
    params_kdf: ParametrosKdf,
) -> Result<InformeSyncPersonal, ErrorDominio> {
    validar_frase_minima(frase)?;
    let mi_dispositivo = repo
        .dispositivo_id()
        .map_err(|e| ErrorDominio::nuevo("almacenamiento", &e))?;

    let mut total_recibidos = 0usize;
    let mut total_ya_existian = 0usize;
    let mut total_enviados = 0usize;
    let mut todos_rechazados: Vec<EventoRechazado> = Vec::new();
    let mut pendientes_publicados: Vec<String> = Vec::new();

    for intento in 0..MAX_REINTENTOS_PUBLICACION {
        // Paso 1: Traer
        transporte.preparar().map_err(|e| e.a_error_dominio())?;

        let lotes_disponibles = transporte.listar_lotes().map_err(|e| e.a_error_dominio())?;
        let leidos = repo
            .lotes_leidos_destino(destino_clave)
            .map_err(|e| ErrorDominio::nuevo("almacenamiento", &e))?;

        // Paso 2: Descifrar y recopilar todos los lotes nuevos antes de fusionar en conjunto.
        // Así las dependencias entre dispositivos (p. ej. CuentaCreada en laptop y Movimiento en teléfono)
        // siempre se proyectan juntas en orden determinista sin importar el orden de los lotes.
        let mut eventos_recibidos_lotes = Vec::new();
        let mut claves_lotes_nuevos = Vec::new();
        let mut ids_en_destino = Vec::new();

        for lote in lotes_disponibles {
            let clave_lote = lote.clave();
            if leidos.contains(&clave_lote) {
                continue;
            }
            let bytes = transporte
                .leer_lote(&lote)
                .map_err(|e| e.a_error_dominio())?;
            let plano_bytes = descifrar(&bytes, frase)?;
            let plano_texto = String::from_utf8(plano_bytes).map_err(|_| {
                ErrorDominio::nuevo("archivo", crate::respaldo::MENSAJE_FRASE_O_DANADO)
            })?;
            let (_enc, eventos) = parsear_eventos_ndjson(&plano_texto)?;

            for ev in &eventos {
                ids_en_destino.push(ev.id.clone());
            }
            eventos_recibidos_lotes.extend(eventos);
            claves_lotes_nuevos.push(clave_lote);
        }

        if !claves_lotes_nuevos.is_empty() {
            let res_fusion = repo
                .fusionar_eventos(&eventos_recibidos_lotes)
                .map_err(|e| ErrorDominio::nuevo("sincronizacion", &e))?;

            total_recibidos += res_fusion.nuevos;
            total_ya_existian += res_fusion.ya_existian;
            todos_rechazados.extend(res_fusion.rechazados);

            for clave_lote in claves_lotes_nuevos {
                repo.marcar_lote_leido(destino_clave, &clave_lote)
                    .map_err(|e| ErrorDominio::nuevo("almacenamiento", &e))?;
            }
            repo.marcar_eventos_enviados(destino_clave, &ids_en_destino)
                .map_err(|e| ErrorDominio::nuevo("almacenamiento", &e))?;
        }

        // Paso 3: Enviar eventos locales que el destino todavía no tiene (RNF-10)
        let ya_en_destino = repo
            .eventos_enviados_destino(destino_clave)
            .map_err(|e| ErrorDominio::nuevo("almacenamiento", &e))?;
        let ya_preparados: HashSet<&str> =
            pendientes_publicados.iter().map(String::as_str).collect();

        let locales = repo
            .obtener_eventos()
            .map_err(|e| ErrorDominio::nuevo("almacenamiento", &e))?;
        let faltantes: Vec<_> = locales
            .into_iter()
            .filter(|ev| !ya_en_destino.contains(&ev.id) && !ya_preparados.contains(ev.id.as_str()))
            .collect();

        if !faltantes.is_empty() {
            let ahora = Utc::now();
            let ndjson = serializar_eventos_ndjson(&faltantes, ahora)?;
            let cifrado = cifrar_con_parametros(ndjson.as_bytes(), frase, params_kdf)?;
            let nombre_lote = format!(
                "{}-{}{EXTENSION_LOTE}",
                ahora.timestamp_millis(),
                uuid::Uuid::new_v4()
            );

            transporte
                .escribir_lote(&mi_dispositivo, &nombre_lote, &cifrado)
                .map_err(|e| e.a_error_dominio())?;

            let ref_lote = ReferenciaLote {
                dispositivo_id: mi_dispositivo.clone(),
                nombre_archivo: nombre_lote,
            };
            repo.marcar_lote_leido(destino_clave, &ref_lote.clave())
                .map_err(|e| ErrorDominio::nuevo("almacenamiento", &e))?;

            for ev in &faltantes {
                pendientes_publicados.push(ev.id.clone());
            }
            total_enviados += faltantes.len();
        }

        // Paso 4: Publicar
        let mensaje = format!(
            "Sincronización Cuentas Claras ({} eventos nuevos)",
            pendientes_publicados.len()
        );
        match transporte
            .publicar(&mensaje)
            .map_err(|e| e.a_error_dominio())?
        {
            ResultadoPublicacion::Publicado => {
                if !pendientes_publicados.is_empty() {
                    repo.marcar_eventos_enviados(destino_clave, &pendientes_publicados)
                        .map_err(|e| ErrorDominio::nuevo("almacenamiento", &e))?;
                }
                let ahora_iso = Utc::now().to_rfc3339();
                repo.guardar_config_local(CLAVE_ULTIMA_SYNC, &ahora_iso)
                    .map_err(|e| ErrorDominio::nuevo("almacenamiento", &e))?;
                return Ok(InformeSyncPersonal {
                    enviados: total_enviados,
                    recibidos: total_recibidos,
                    ya_existian: total_ya_existian,
                    rechazados: todos_rechazados,
                    ultima_sincronizacion: ahora_iso,
                });
            }
            ResultadoPublicacion::RemotoActualizado => {
                if intento + 1 == MAX_REINTENTOS_PUBLICACION {
                    return Err(ErrorDominio::nuevo(
                        "destino",
                        "El repositorio cambió varias veces durante la sincronización. Vuelve a intentarlo en unos segundos.",
                    ));
                }
                continue;
            }
        }
    }

    Err(ErrorDominio::nuevo(
        "destino",
        "No se pudo completar la publicación en el destino; se reintentará.",
    ))
}

/// Construye el transporte real (Carpeta o Git) a partir de la configuración guardada y ejecuta la sincronización.
pub fn sincronizar_con_destino_configurado(
    repo: &mut RepositorioSqlite,
    secretos: &dyn AlmacenSecretos,
    directorio_datos_app: &Path,
    params_kdf: ParametrosKdf,
) -> Result<InformeSyncPersonal, ErrorDominio> {
    let cfg = obtener_configuracion_sync(repo)?.ok_or_else(|| {
        ErrorDominio::nuevo(
            "destino",
            "No hay ningún destino de sincronización configurado todavía.",
        )
    })?;
    let frase = secretos.leer(CLAVE_SECRETO_FRASE)?.ok_or_else(|| {
        ErrorDominio::nuevo(
            "frase",
            "No se encontró la frase de paso en el almacén de secretos. Configúrala nuevamente.",
        )
    })?;
    let clave_destino = cfg.clave_destino();

    match cfg.tipo {
        TipoDestinoSync::Carpeta => {
            let mut transporte = TransporteCarpeta::nuevo(PathBuf::from(&cfg.destino));
            ejecutar_sincronizacion_personal(
                repo,
                &mut transporte,
                &clave_destino,
                &frase,
                params_kdf,
            )
        }
        TipoDestinoSync::Git => {
            let cred_secreto = secretos.leer(CLAVE_SECRETO_GIT_TOKEN)?;
            let credenciales = cred_secreto.map(|secreto| CredencialesGit {
                usuario: cfg.usuario_git.clone().unwrap_or_else(|| "git".to_string()),
                secreto,
            });
            let dir_clon = directorio_datos_app
                .join("sync_git_clones")
                .join(&clave_destino[..16]);
            let mut transporte = TransporteGit::nuevo(
                cfg.destino.clone(),
                cfg.rama.clone().unwrap_or_else(|| "main".to_string()),
                dir_clon,
                credenciales,
            );
            ejecutar_sincronizacion_personal(
                repo,
                &mut transporte,
                &clave_destino,
                &frase,
                params_kdf,
            )
        }
    }
}
