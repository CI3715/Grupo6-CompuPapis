//! pruebas_entrega8.rs: Pruebas automatizadas de la Entrega 8
//! - Fusión: idempotencia, orden indistinto, conflicto de contenido, archivo antiguo, restauración, gastos de grupo
//! - Cifrado: ida y vuelta (producción y pruebas), frase incorrecta, archivo alterado, versión no soportada, privacidad
//! - Contrato del puerto TransporteSincronizacion para los 3 adaptadores: Memoria, Carpeta y Git (RNF-14, RNF-15)
//! - Convergencia exacta del Ejemplo 2.6 en ambos órdenes y sincronización incremental (RNF-10)

use crate::domain::{
    EventoLibro, RegistrarMovimiento, RepositorioEventos, ejecutar_crear_cuenta,
    ejecutar_listar_cuentas, ejecutar_obtener_detalle_cuenta, ejecutar_registrar_movimiento,
};
use crate::respaldo::{
    MENSAJE_FRASE_O_DANADO, ParametrosKdf, cifrar, cifrar_con_parametros, descifrar,
    es_archivo_cifrado, exportar_bytes, importar_bytes, parsear_eventos_ndjson,
};
use crate::sincronizacion_personal::{
    AlmacenSecretos, AlmacenSecretosMemoria, ConfigSyncPersonal, ErrorTransporte, ReferenciaLote,
    TipoDestinoSync, TransporteCarpeta, TransporteGit, TransporteMemoria, TransporteSincronizacion,
    ejecutar_sincronizacion_personal, guardar_configuracion_sync, validar_url_sin_credenciales,
};
use crate::sqlite_repo::{EventoCambioRecibido, RepositorioSqlite};
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

const FRASE_PRUEBA: &str = "mi-frase-secreta-segura-2026";
const FRASE_INCORRECTA: &str = "otra-frase-equivocada-9999";

struct DirTemp(PathBuf);

impl DirTemp {
    fn nuevo(prefijo: &str) -> Self {
        let ruta = std::env::temp_dir().join(format!("cc_e8_{prefijo}_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&ruta).unwrap();
        Self(ruta)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for DirTemp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn bd_en(dir: &Path, nombre: &str) -> RepositorioSqlite {
    RepositorioSqlite::inicializar(dir.join(format!("{nombre}.sqlite"))).unwrap()
}

fn crear_cuenta_y_movimientos_base(repo: &mut RepositorioSqlite) -> String {
    let disp = repo.dispositivo_id().unwrap();
    let cuenta =
        ejecutar_crear_cuenta(repo, "Efectivo USD", "efectivo", "USD", "151.67", &disp).unwrap();
    let cid = cuenta.id().to_string();
    ejecutar_registrar_movimiento(
        repo,
        RegistrarMovimiento {
            cuenta_id: &cid,
            tipo_str: "ingreso",
            monto_str: "100.00",
            fecha_str: "2026-11-20",
            categoria_str: "Salario",
            nota_opt: Some("Pago quincena".into()),
            dispositivo_id: &disp,
        },
    )
    .unwrap();
    ejecutar_registrar_movimiento(
        repo,
        RegistrarMovimiento {
            cuenta_id: &cid,
            tipo_str: "gasto",
            monto_str: "100.00",
            fecha_str: "2026-11-21",
            categoria_str: "Alimentación",
            nota_opt: Some("Mercado semanal".into()),
            dispositivo_id: &disp,
        },
    )
    .unwrap();
    cid
}

mod exportacion;
mod fusion;
mod respaldo;
mod sincronizacion;
