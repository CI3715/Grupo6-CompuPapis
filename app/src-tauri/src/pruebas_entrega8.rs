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

// =========================================================
// 1. PRUEBAS DE CIFRADO (ARGON2ID + AES-256-GCM) Y PRIVACIDAD
// =========================================================

#[test]
fn cifrado_ida_y_vuelta_con_parametros_de_produccion_y_sal_unica() {
    let plano = b"Contenido secreto del libro financiero personal";
    let c1 = cifrar(plano, FRASE_PRUEBA).unwrap();
    let c2 = cifrar(plano, FRASE_PRUEBA).unwrap();

    // Dos cifrados del mismo contenido con la misma frase generan salidas distintas (sal y nonce nuevos)
    assert_ne!(c1, c2);
    assert!(es_archivo_cifrado(&c1));

    // La parte en claro no revela ningún dato del contenido ni la frase
    let texto_c1 = String::from_utf8_lossy(&c1);
    assert!(!texto_c1.contains("Contenido secreto"));
    assert!(!texto_c1.contains("financiero"));
    assert!(!texto_c1.contains(FRASE_PRUEBA));

    // Descifrado exacto
    let recuperado = descifrar(&c1, FRASE_PRUEBA).unwrap();
    assert_eq!(recuperado, plano);
}

#[test]
fn cifrado_frase_incorrecta_y_archivo_alterado_fallan_sin_modificar_libro() {
    let dir = DirTemp::nuevo("cifrado_err");
    let mut origen = bd_en(dir.path(), "origen");
    let cid = crear_cuenta_y_movimientos_base(&mut origen);

    let (bytes_cifrados, _, _) = exportar_bytes(
        &origen,
        true,
        Some(FRASE_PRUEBA),
        Some(FRASE_PRUEBA),
        ParametrosKdf::para_pruebas(),
    )
    .unwrap();

    // Privacidad: el archivo cifrado no revela nombres de cuentas, notas ni frase (RF-INT-05, RF-INT-06)
    let texto_cifrado = String::from_utf8_lossy(&bytes_cifrados);
    assert!(!texto_cifrado.contains("Efectivo USD"));
    assert!(!texto_cifrado.contains("Mercado semanal"));
    assert!(!texto_cifrado.contains("151.67"));
    assert!(!texto_cifrado.contains(FRASE_PRUEBA));

    let mut destino = bd_en(dir.path(), "destino");
    assert_eq!(destino.obtener_eventos().unwrap().len(), 0);

    // 1) Frase incorrecta -> mensaje exacto, libro vacío/intacto
    let err_frase = importar_bytes(&mut destino, &bytes_cifrados, Some(FRASE_INCORRECTA))
        .expect_err("Debe fallar con frase incorrecta");
    assert_eq!(err_frase.mensaje, MENSAJE_FRASE_O_DANADO);
    assert_eq!(destino.obtener_eventos().unwrap().len(), 0);

    // 2) Alterar 1 byte del ciphertext en JSON -> falla verificación autenticada
    let mut json_alterado: serde_json::Value = serde_json::from_slice(&bytes_cifrados).unwrap();
    let datos_orig = json_alterado["datos"].as_str().unwrap().to_string();
    let mut chars: Vec<char> = datos_orig.chars().collect();
    let pos = chars.len() / 2;
    chars[pos] = if chars[pos] == 'A' { 'B' } else { 'A' };
    json_alterado["datos"] = serde_json::Value::String(chars.into_iter().collect());
    let bytes_alterados = serde_json::to_vec(&json_alterado).unwrap();

    let err_alt = importar_bytes(&mut destino, &bytes_alterados, Some(FRASE_PRUEBA))
        .expect_err("Debe fallar si el ciphertext fue alterado");
    assert_eq!(err_alt.mensaje, MENSAJE_FRASE_O_DANADO);
    assert_eq!(destino.obtener_eventos().unwrap().len(), 0);

    // 3) Alterar parámetro del encabezado (AAD) -> también falla verificación autenticada
    let mut json_aad: serde_json::Value = serde_json::from_slice(&bytes_cifrados).unwrap();
    json_aad["kdf"]["iteraciones"] = serde_json::json!(2);
    let bytes_aad = serde_json::to_vec(&json_aad).unwrap();
    let err_aad = importar_bytes(&mut destino, &bytes_aad, Some(FRASE_PRUEBA))
        .expect_err("Debe fallar si se alteró el encabezado");
    assert_eq!(err_aad.mensaje, MENSAJE_FRASE_O_DANADO);

    // 4) Versión de formato no soportada -> mensaje claro indicando versión del archivo y soportada
    let mut json_ver: serde_json::Value = serde_json::from_slice(&bytes_cifrados).unwrap();
    json_ver["version"] = serde_json::json!(99);
    let bytes_ver = serde_json::to_vec(&json_ver).unwrap();
    let err_ver = importar_bytes(&mut destino, &bytes_ver, Some(FRASE_PRUEBA))
        .expect_err("Debe rechazar versión no soportada");
    assert!(err_ver.mensaje.contains("99"));
    assert!(err_ver.mensaje.contains('1'));

    // 5) Con frase correcta restaura todo exactamente igual
    let res = importar_bytes(&mut destino, &bytes_cifrados, Some(FRASE_PRUEBA)).unwrap();
    assert_eq!(res.nuevos, 3);
    assert_eq!(res.ya_existian, 0);
    assert!(res.rechazados.is_empty());
    let det = ejecutar_obtener_detalle_cuenta(&destino, &cid).unwrap();
    assert_eq!(det.saldo_actual, "151.67");
}

// =========================================================
// 2. PRUEBAS DE FUSIÓN, IDEMPOTENCIA, CONFLICTO Y GRUPOS
// =========================================================

#[test]
fn fusion_idempotente_archivo_antiguo_no_borra_y_conflicto_se_rechaza() {
    let dir = DirTemp::nuevo("fusion");
    let mut repo_a = bd_en(dir.path(), "a");
    let cid = crear_cuenta_y_movimientos_base(&mut repo_a);

    // Exportación inicial (3 eventos)
    let (respaldo_antiguo, total_exp, _) =
        exportar_bytes(&repo_a, false, None, None, ParametrosKdf::para_pruebas()).unwrap();
    assert_eq!(total_exp, 3);

    // Exportar no modifica el libro (RF-INT-01)
    assert_eq!(repo_a.obtener_eventos().unwrap().len(), 3);

    // Importar en instalación nueva (restauración completa)
    let mut repo_b = bd_en(dir.path(), "b");
    let r1 = importar_bytes(&mut repo_b, &respaldo_antiguo, None).unwrap();
    assert_eq!(r1.nuevos, 3);
    assert_eq!(r1.ya_existian, 0);
    assert!(r1.rechazados.is_empty());

    // Importar el mismo archivo dos veces -> 0 nuevos, 3 ya existían (RF-INT-02, ESC-04)
    let r2 = importar_bytes(&mut repo_b, &respaldo_antiguo, None).unwrap();
    assert_eq!(r2.nuevos, 0);
    assert_eq!(r2.ya_existian, 3);
    assert!(r2.rechazados.is_empty());

    // Agregar un movimiento nuevo en B y volver a importar el archivo ANTIGUO:
    // nunca borra eventos locales (INV-14)
    let disp_b = repo_b.dispositivo_id().unwrap();
    ejecutar_registrar_movimiento(
        &mut repo_b,
        RegistrarMovimiento {
            cuenta_id: &cid,
            tipo_str: "ingreso",
            monto_str: "50.00",
            fecha_str: "2026-11-22",
            categoria_str: "Ventas",
            nota_opt: None,
            dispositivo_id: &disp_b,
        },
    )
    .unwrap();
    let r_antiguo = importar_bytes(&mut repo_b, &respaldo_antiguo, None).unwrap();
    assert_eq!(r_antiguo.nuevos, 0);
    assert_eq!(r_antiguo.ya_existian, 3);
    assert_eq!(repo_b.obtener_eventos().unwrap().len(), 4);
    assert_eq!(
        ejecutar_obtener_detalle_cuenta(&repo_b, &cid)
            .unwrap()
            .saldo_actual,
        "201.67"
    );

    // Conflicto: mismo identificador con contenido distinto -> rechazado con motivo;
    // el resto de eventos nuevos válidos sí se aplica normalmente (Sección 2.1 y 2.7)
    let mut eventos_existentes = repo_b.obtener_eventos().unwrap();
    let mut evento_conflictivo = eventos_existentes.remove(1);
    evento_conflictivo.datos_json = evento_conflictivo.datos_json.replace("100.00", "999.99");

    // Crear un cuarto evento válido en A y mezclarlo con el conflictivo
    let disp_a = repo_a.dispositivo_id().unwrap();
    ejecutar_registrar_movimiento(
        &mut repo_a,
        RegistrarMovimiento {
            cuenta_id: &cid,
            tipo_str: "gasto",
            monto_str: "10.00",
            fecha_str: "2026-11-23",
            categoria_str: "Transporte",
            nota_opt: Some("Bus".into()),
            dispositivo_id: &disp_a,
        },
    )
    .unwrap();
    let evento_nuevo_valido = repo_a.obtener_eventos().unwrap().pop().unwrap();

    let res_conflicto = repo_b
        .fusionar_eventos(&[evento_conflictivo.clone(), evento_nuevo_valido])
        .unwrap();
    assert_eq!(res_conflicto.nuevos, 1);
    assert_eq!(res_conflicto.rechazados.len(), 1);
    assert_eq!(res_conflicto.rechazados[0].evento_id, evento_conflictivo.id);
    assert!(!res_conflicto.rechazados[0].motivo.is_empty());

    // El saldo refleja 201.67 - 10.00 = 191.67 (el conflictivo 999.99 NO modificó el libro)
    assert_eq!(
        ejecutar_obtener_detalle_cuenta(&repo_b, &cid)
            .unwrap()
            .saldo_actual,
        "191.67"
    );
}

#[test]
fn gastos_de_grupo_descargados_en_dos_dispositivos_convergen_una_sola_vez() {
    let dir = DirTemp::nuevo("grupo_dedup");
    let mut disp1 = bd_en(dir.path(), "disp1");
    let mut disp2 = bd_en(dir.path(), "disp2");

    // Ambos parten de la misma cuenta USD
    let id1 = disp1.dispositivo_id().unwrap();
    let cuenta = ejecutar_crear_cuenta(
        &mut disp1,
        "Efectivo USD",
        "efectivo",
        "USD",
        "200.00",
        &id1,
    )
    .unwrap();
    let cid = cuenta.id().to_string();
    let (exp, _, _) =
        exportar_bytes(&disp1, false, None, None, ParametrosKdf::para_pruebas()).unwrap();
    importar_bytes(&mut disp2, &exp, None).unwrap();

    let grupo_id = Uuid::new_v4().to_string();
    let evento_servidor_id = Uuid::new_v4().to_string();
    let gasto_id = Uuid::new_v4().to_string();

    disp1.elegir_cuenta_grupo(&grupo_id, &cid).unwrap();
    disp2.elegir_cuenta_grupo(&grupo_id, &cid).unwrap();

    let cambio_servidor = EventoCambioRecibido {
        id: evento_servidor_id,
        cursor: 1,
        tipo: "gasto_grupo".into(),
        grupo_id,
        grupo_nombre: "Viaje".into(),
        gasto_id,
        concepto: "Hotel".into(),
        fecha: "2026-11-20".into(),
        moneda: "USD".into(),
        monto_parte: "33.33".into(),
        incluye_centimos_redondeo: false,
    };

    // Ambos dispositivos descargan la misma parte del servidor por separado
    assert_eq!(
        disp1
            .aplicar_lote_cambios(1, vec![cambio_servidor.clone()])
            .unwrap(),
        1
    );
    assert_eq!(
        disp2
            .aplicar_lote_cambios(1, vec![cambio_servidor])
            .unwrap(),
        1
    );

    // Al fusionar disp2 en disp1, el evento derivado coincide y NO se duplica (Sección 2.1)
    let evs2 = disp2.obtener_eventos().unwrap();
    let res = disp1.fusionar_eventos(&evs2).unwrap();
    assert_eq!(res.nuevos, 0);
    assert_eq!(res.ya_existian, 2);
    assert!(res.rechazados.is_empty());

    let det = ejecutar_obtener_detalle_cuenta(&disp1, &cid).unwrap();
    assert_eq!(det.saldo_actual, "166.67");
    assert_eq!(det.movimientos.len(), 1);
}

// =========================================================
// 3. CONTRATO COMÚN PARA LOS 3 ADAPTADORES DE TRANSPORTE (RNF-14, RNF-15)
//    + CONVERGENCIA DEL EJEMPLO 2.6 EN AMBOS ÓRDENES + INCREMENTALIDAD (RNF-10)
// =========================================================

/// Prueba de contrato que TODOS los adaptadores de TransporteSincronizacion deben superar:
/// - Lectura/escritura por carpeta de dispositivo sin sobrescribir lotes inmutables
/// - Convergencia exacta tipo Ejemplo 2.6 (laptop y teléfono offline) en ambos órdenes
/// - Sincronización idempotente (repetir = 0 enviados, 0 recibidos)
/// - Solo envía eventos nuevos (RNF-10)
/// - Unión de un tercer dispositivo nuevo trae todo el libro
fn ejecutar_contrato_transporte<F, T>(mut crear_transporte: F, orden_telefono_primero: bool)
where
    F: FnMut(&str) -> T,
    T: TransporteSincronizacion,
{
    let dir = DirTemp::nuevo("contrato");
    let mut laptop = bd_en(dir.path(), "laptop");
    let mut telefono = bd_en(dir.path(), "telefono");
    let mut tablet_nueva = bd_en(dir.path(), "tablet_nueva");

    // Estado inicial compartido: Efectivo USD con saldo 151.67
    let cid = crear_cuenta_y_movimientos_base(&mut laptop);
    let (base_bytes, _, _) =
        exportar_bytes(&laptop, false, None, None, ParametrosKdf::para_pruebas()).unwrap();
    importar_bytes(&mut telefono, &base_bytes, None).unwrap();

    // Primera sincronización base para que ambos partan con los 3 eventos ya publicados en destino
    let mut t_l = crear_transporte("laptop");
    let mut t_t = crear_transporte("telefono");
    let clave_dest = "destino-contrato";

    ejecutar_sincronizacion_personal(
        &mut laptop,
        &mut t_l,
        clave_dest,
        FRASE_PRUEBA,
        ParametrosKdf::para_pruebas(),
    )
    .unwrap();
    ejecutar_sincronizacion_personal(
        &mut telefono,
        &mut t_t,
        clave_dest,
        FRASE_PRUEBA,
        ParametrosKdf::para_pruebas(),
    )
    .unwrap();

    // Sin conexión, laptop registra almuerzo 20.00 y café 5.00 (-> 126.67)
    let id_laptop = laptop.dispositivo_id().unwrap();
    ejecutar_registrar_movimiento(
        &mut laptop,
        RegistrarMovimiento {
            cuenta_id: &cid,
            tipo_str: "gasto",
            monto_str: "20.00",
            fecha_str: "2026-11-24",
            categoria_str: "Alimentación",
            nota_opt: Some("Almuerzo".into()),
            dispositivo_id: &id_laptop,
        },
    )
    .unwrap();
    ejecutar_registrar_movimiento(
        &mut laptop,
        RegistrarMovimiento {
            cuenta_id: &cid,
            tipo_str: "gasto",
            monto_str: "5.00",
            fecha_str: "2026-11-24",
            categoria_str: "Alimentación",
            nota_opt: Some("Café".into()),
            dispositivo_id: &id_laptop,
        },
    )
    .unwrap();
    assert_eq!(
        ejecutar_obtener_detalle_cuenta(&laptop, &cid)
            .unwrap()
            .saldo_actual,
        "126.67"
    );

    // Sin conexión, teléfono registra ingreso 50.00 y gasto 10.00 (-> 191.67)
    let id_tel = telefono.dispositivo_id().unwrap();
    ejecutar_registrar_movimiento(
        &mut telefono,
        RegistrarMovimiento {
            cuenta_id: &cid,
            tipo_str: "ingreso",
            monto_str: "50.00",
            fecha_str: "2026-11-24",
            categoria_str: "Ventas",
            nota_opt: None,
            dispositivo_id: &id_tel,
        },
    )
    .unwrap();
    ejecutar_registrar_movimiento(
        &mut telefono,
        RegistrarMovimiento {
            cuenta_id: &cid,
            tipo_str: "gasto",
            monto_str: "10.00",
            fecha_str: "2026-11-24",
            categoria_str: "Transporte",
            nota_opt: None,
            dispositivo_id: &id_tel,
        },
    )
    .unwrap();
    assert_eq!(
        ejecutar_obtener_detalle_cuenta(&telefono, &cid)
            .unwrap()
            .saldo_actual,
        "191.67"
    );

    // Sincronizar en el orden indicado (RNF-10: solo envía 2 eventos nuevos por dispositivo)
    if !orden_telefono_primero {
        let inf1 = ejecutar_sincronizacion_personal(
            &mut laptop,
            &mut t_l,
            clave_dest,
            FRASE_PRUEBA,
            ParametrosKdf::para_pruebas(),
        )
        .unwrap();
        assert_eq!(inf1.enviados, 2);
        assert_eq!(inf1.recibidos, 0);

        let inf2 = ejecutar_sincronizacion_personal(
            &mut telefono,
            &mut t_t,
            clave_dest,
            FRASE_PRUEBA,
            ParametrosKdf::para_pruebas(),
        )
        .unwrap();
        assert_eq!(inf2.enviados, 2);
        assert_eq!(inf2.recibidos, 2);

        let inf3 = ejecutar_sincronizacion_personal(
            &mut laptop,
            &mut t_l,
            clave_dest,
            FRASE_PRUEBA,
            ParametrosKdf::para_pruebas(),
        )
        .unwrap();
        assert_eq!(inf3.enviados, 0);
        assert_eq!(inf3.recibidos, 2);
    } else {
        let inf1 = ejecutar_sincronizacion_personal(
            &mut telefono,
            &mut t_t,
            clave_dest,
            FRASE_PRUEBA,
            ParametrosKdf::para_pruebas(),
        )
        .unwrap();
        assert_eq!(inf1.enviados, 2);
        assert_eq!(inf1.recibidos, 0);

        let inf2 = ejecutar_sincronizacion_personal(
            &mut laptop,
            &mut t_l,
            clave_dest,
            FRASE_PRUEBA,
            ParametrosKdf::para_pruebas(),
        )
        .unwrap();
        assert_eq!(inf2.enviados, 2);
        assert_eq!(inf2.recibidos, 2);

        let inf3 = ejecutar_sincronizacion_personal(
            &mut telefono,
            &mut t_t,
            clave_dest,
            FRASE_PRUEBA,
            ParametrosKdf::para_pruebas(),
        )
        .unwrap();
        assert_eq!(inf3.enviados, 0);
        assert_eq!(inf3.recibidos, 2);
    }

    // Ambos convergen exactamente a 7 eventos y saldo 166.67
    assert_eq!(laptop.obtener_eventos().unwrap().len(), 7);
    assert_eq!(telefono.obtener_eventos().unwrap().len(), 7);
    assert_eq!(
        ejecutar_obtener_detalle_cuenta(&laptop, &cid)
            .unwrap()
            .saldo_actual,
        "166.67"
    );
    assert_eq!(
        ejecutar_obtener_detalle_cuenta(&telefono, &cid)
            .unwrap()
            .saldo_actual,
        "166.67"
    );

    // Repetir sincronización no cambia nada (0 enviados, 0 recibidos)
    let inf_rep_l = ejecutar_sincronizacion_personal(
        &mut laptop,
        &mut t_l,
        clave_dest,
        FRASE_PRUEBA,
        ParametrosKdf::para_pruebas(),
    )
    .unwrap();
    assert_eq!(inf_rep_l.enviados, 0);
    assert_eq!(inf_rep_l.recibidos, 0);

    // Un dispositivo nuevo (tablet) se une configurando mismo destino y frase -> trae todo el libro
    let mut t_tab = crear_transporte("tablet");
    let inf_tab = ejecutar_sincronizacion_personal(
        &mut tablet_nueva,
        &mut t_tab,
        clave_dest,
        FRASE_PRUEBA,
        ParametrosKdf::para_pruebas(),
    )
    .unwrap();
    assert_eq!(inf_tab.enviados, 0);
    assert_eq!(inf_tab.recibidos, 7);
    assert_eq!(
        ejecutar_obtener_detalle_cuenta(&tablet_nueva, &cid)
            .unwrap()
            .saldo_actual,
        "166.67"
    );
    assert_eq!(
        ejecutar_listar_cuentas(&tablet_nueva)
            .unwrap()
            .cuentas
            .len(),
        1
    );

    // Verifica que un lote existente no puede sobrescribirse (inmutabilidad de lotes)
    let lotes = t_tab.listar_lotes().unwrap();
    assert!(!lotes.is_empty());
    let primero = &lotes[0];
    let err_sobrescribir = t_tab.escribir_lote(
        &primero.dispositivo_id,
        &primero.nombre_archivo,
        b"alteracion",
    );
    assert!(err_sobrescribir.is_err());
}

#[test]
fn contrato_transporte_memoria_ambos_ordenes_y_privacidad() {
    let mem1 = TransporteMemoria::nuevo();
    ejecutar_contrato_transporte(|_| mem1.clone(), false);

    // Todos los lotes almacenados en el destino están cifrados y no revelan datos del libro
    for bytes in mem1.todos_los_bytes() {
        assert!(es_archivo_cifrado(&bytes));
        let texto = String::from_utf8_lossy(&bytes);
        assert!(!texto.contains("Efectivo USD"));
        assert!(!texto.contains("Almuerzo"));
        assert!(!texto.contains(FRASE_PRUEBA));
    }

    let mem2 = TransporteMemoria::nuevo();
    ejecutar_contrato_transporte(|_| mem2.clone(), true);
}

#[test]
fn contrato_transporte_carpeta_compartida_ambos_ordenes_y_escritura_atomica() {
    let dir1 = DirTemp::nuevo("carpeta_1");
    ejecutar_contrato_transporte(
        |_| TransporteCarpeta::nuevo(dir1.path().to_path_buf()),
        false,
    );

    // Verificar que los archivos .tmp-* son ignorados (nunca se lee un archivo a medio escribir)
    let disp_dir = dir1.path().join("dispositivos").join("otro-disp");
    std::fs::create_dir_all(&disp_dir).unwrap();
    std::fs::write(disp_dir.join(".lote.tmp-incompleto"), b"basura parcial").unwrap();
    let tc = TransporteCarpeta::nuevo(dir1.path().to_path_buf());
    let lotes = tc.listar_lotes().unwrap();
    assert!(
        lotes
            .iter()
            .all(|l| !l.nombre_archivo.starts_with('.') && l.nombre_archivo.ends_with(".ccsync"))
    );

    let dir2 = DirTemp::nuevo("carpeta_2");
    ejecutar_contrato_transporte(
        |_| TransporteCarpeta::nuevo(dir2.path().to_path_buf()),
        true,
    );
}

#[test]
fn contrato_transporte_git_real_ambos_ordenes_y_sin_credenciales_en_disco() {
    for orden_tel_primero in [false, true] {
        let dir = DirTemp::nuevo("git_contrato");
        let remoto_bare = dir.path().join("remoto.git");
        let ok = Command::new("git")
            .args(["init", "--bare", remoto_bare.to_str().unwrap()])
            .output()
            .expect("git debe estar instalado para las pruebas de contrato Git")
            .status
            .success();
        assert!(ok, "Falló git init --bare");

        let url = remoto_bare.to_string_lossy().to_string();
        ejecutar_contrato_transporte(
            |nombre_instancia| {
                TransporteGit::nuevo(
                    url.clone(),
                    "main".into(),
                    dir.path().join(format!("clon_{nombre_instancia}")),
                    None,
                )
            },
            orden_tel_primero,
        );

        // Verificar privacidad: todos los archivos publicados en el repositorio Git están cifrados
        let clon_laptop = dir.path().join("clon_laptop");
        let t_git = TransporteGit::nuevo(url.clone(), "main".into(), clon_laptop.clone(), None);
        let lotes = t_git.listar_lotes().unwrap();
        assert!(lotes.len() >= 3);
        for lote in &lotes {
            let bytes = t_git.leer_lote(lote).unwrap();
            assert!(es_archivo_cifrado(&bytes));
            let s = String::from_utf8_lossy(&bytes);
            assert!(!s.contains("Efectivo USD"));
            assert!(!s.contains("Almuerzo"));
            assert!(!s.contains(FRASE_PRUEBA));
        }

        // Ninguna credencial ni frase aparece en .git/config
        let git_config = std::fs::read_to_string(clon_laptop.join(".git/config")).unwrap();
        assert!(!git_config.contains(FRASE_PRUEBA));
    }
}

#[test]
fn reintento_ante_remoto_actualizado_y_errores_sin_perdida_de_datos() {
    let dir = DirTemp::nuevo("reintento");
    let mut repo = bd_en(dir.path(), "disp");
    let cid = crear_cuenta_y_movimientos_base(&mut repo);

    let mut transporte = TransporteMemoria::nuevo();
    // 1) Sin conexión: informa error claro y no pierde nada
    transporte.establecer_inaccesible(true);
    let err_offline = ejecutar_sincronizacion_personal(
        &mut repo,
        &mut transporte,
        "dest-err",
        FRASE_PRUEBA,
        ParametrosKdf::para_pruebas(),
    )
    .expect_err("Debe fallar cuando el destino es inaccesible");
    assert!(err_offline.mensaje.contains("reintentará"));
    assert_eq!(repo.obtener_eventos().unwrap().len(), 3);

    // 2) Credenciales inválidas: informa error claro y conserva los pendientes
    transporte.establecer_inaccesible(false);
    transporte.establecer_credenciales_invalidas(true);
    let err_cred = ejecutar_sincronizacion_personal(
        &mut repo,
        &mut transporte,
        "dest-err",
        FRASE_PRUEBA,
        ParametrosKdf::para_pruebas(),
    )
    .expect_err("Debe fallar con credenciales rechazadas");
    assert_eq!(err_cred.campo, "credenciales");
    assert!(err_cred.mensaje.contains("rechazadas"));

    // 3) Al volver la red con conflicto concurrente simulado (RemotoActualizado), reintenta y publica los 3 pendientes
    transporte.establecer_credenciales_invalidas(false);
    transporte.simular_conflicto_remoto(2);
    let inf = ejecutar_sincronizacion_personal(
        &mut repo,
        &mut transporte,
        "dest-err",
        FRASE_PRUEBA,
        ParametrosKdf::para_pruebas(),
    )
    .unwrap();
    assert_eq!(inf.enviados, 3);
    assert_eq!(transporte.total_lotes(), 1);
    assert_eq!(
        ejecutar_obtener_detalle_cuenta(&repo, &cid)
            .unwrap()
            .saldo_actual,
        "151.67"
    );
}

#[test]
fn git_conserva_carpeta_existente_que_no_es_repositorio() {
    let dir = DirTemp::nuevo("git_carpeta_existente");
    let clon = dir.path().join("clon");
    std::fs::create_dir(&clon).unwrap();
    std::fs::write(clon.join("archivo"), b"conservar").unwrap();
    let mut transporte = TransporteGit::nuevo("remoto".into(), "main".into(), clon.clone(), None);
    assert!(transporte.preparar().is_err());
    assert_eq!(std::fs::read(clon.join("archivo")).unwrap(), b"conservar");
}

#[test]
fn secretos_no_se_guardan_en_sqlite_ni_en_url_git() {
    let dir = DirTemp::nuevo("secretos");
    let repo = bd_en(dir.path(), "secretos");
    let secretos = AlmacenSecretosMemoria::default();

    // Rechaza URL con credenciales incrustadas
    assert!(
        validar_url_sin_credenciales("https://usuario:token123@github.com/u/repo.git").is_err()
    );
    assert!(validar_url_sin_credenciales("https://github.com/u/repo.git").is_ok());
    assert!(validar_url_sin_credenciales("https://token123@github.com/u/repo.git").is_err());
    assert!(validar_url_sin_credenciales("ssh://git@github.com/u/repo.git").is_ok());
    assert!(validar_url_sin_credenciales("ssh://git:secreto@github.com/u/repo.git").is_err());

    let cfg = ConfigSyncPersonal {
        tipo: TipoDestinoSync::Git,
        destino: "https://github.com/beto/cuentas-claras-sync.git".into(),
        rama: Some("main".into()),
        usuario_git: Some("beto".into()),
    };
    let estado = guardar_configuracion_sync(
        &repo,
        &secretos,
        &cfg,
        Some(FRASE_PRUEBA),
        Some("ghp_token_super_secreto_12345"),
    )
    .unwrap();
    assert!(estado.configurado);
    assert!(estado.tiene_frase);
    assert!(estado.tiene_credencial_git);

    // El secreto vive en AlmacenSecretos, jamás en el archivo SQLite
    let sqlite_bytes = std::fs::read(dir.path().join("secretos.sqlite")).unwrap();
    let sqlite_str = String::from_utf8_lossy(&sqlite_bytes);
    assert!(!sqlite_str.contains(FRASE_PRUEBA));
    assert!(!sqlite_str.contains("ghp_token_super_secreto_12345"));
    assert_eq!(
        secretos.leer("frase_paso_sync").unwrap().as_deref(),
        Some(FRASE_PRUEBA)
    );
}

#[test]
fn sincronizacion_incremental_solo_envia_eventos_faltantes_rnf10() {
    let dir = DirTemp::nuevo("rnf10");
    let mut repo = bd_en(dir.path(), "grande");
    let disp = repo.dispositivo_id().unwrap();
    let cuenta = ejecutar_crear_cuenta(
        &mut repo,
        "Cuenta Principal",
        "banco",
        "USD",
        "1000.00",
        &disp,
    )
    .unwrap();
    let cid = cuenta.id().to_string();

    // Crear 25 movimientos iniciales
    for i in 0..25 {
        ejecutar_registrar_movimiento(
            &mut repo,
            RegistrarMovimiento {
                cuenta_id: &cid,
                tipo_str: "gasto",
                monto_str: "1.00",
                fecha_str: "2026-11-20",
                categoria_str: "Otros gastos",
                nota_opt: Some(format!("Gasto #{i}")),
                dispositivo_id: &disp,
            },
        )
        .unwrap();
    }

    let mut transporte = TransporteMemoria::nuevo();
    let inf1 = ejecutar_sincronizacion_personal(
        &mut repo,
        &mut transporte,
        "dest-rnf10",
        FRASE_PRUEBA,
        ParametrosKdf::para_pruebas(),
    )
    .unwrap();
    assert_eq!(inf1.enviados, 26);

    // Agregar solo 2 eventos nuevos
    for i in 0..2 {
        ejecutar_registrar_movimiento(
            &mut repo,
            RegistrarMovimiento {
                cuenta_id: &cid,
                tipo_str: "gasto",
                monto_str: "5.00",
                fecha_str: "2026-11-21",
                categoria_str: "Alimentación",
                nota_opt: Some(format!("Nuevo #{i}")),
                dispositivo_id: &disp,
            },
        )
        .unwrap();
    }

    let inf2 = ejecutar_sincronizacion_personal(
        &mut repo,
        &mut transporte,
        "dest-rnf10",
        FRASE_PRUEBA,
        ParametrosKdf::para_pruebas(),
    )
    .unwrap();
    assert_eq!(inf2.enviados, 2);

    // Verificar que el segundo lote realmente contiene SOLO 2 eventos, no los 28
    let lotes = transporte.listar_lotes().unwrap();
    assert_eq!(lotes.len(), 2);
    let mut tamanos: Vec<usize> = lotes
        .iter()
        .map(|l| {
            let bytes = transporte.leer_lote(l).unwrap();
            let plano = String::from_utf8(descifrar(&bytes, FRASE_PRUEBA).unwrap()).unwrap();
            parsear_eventos_ndjson(&plano).unwrap().1.len()
        })
        .collect();
    tamanos.sort();
    assert_eq!(tamanos, vec![2, 26]);
}

#[test]
fn validacion_de_transporte_rechaza_lotes_y_dispositivos_con_rutas_inseguras() {
    let mut mem = TransporteMemoria::nuevo();
    let res = mem.escribir_lote("../escape", "lote.ccsync", b"x");
    assert!(matches!(res, Err(ErrorTransporte::ErrorOperacion(_))));
    let res2 = mem.leer_lote(&ReferenciaLote {
        dispositivo_id: "d1".into(),
        nombre_archivo: "inexistente.ccsync".into(),
    });
    assert!(res2.is_err());

    // Verificar también que un texto plano corrupto es rechazado por parsear_eventos_ndjson
    let err = parsear_eventos_ndjson("{\"formato\":\"cuentas-claras-libro\",\"version\":2,\"fecha_exportacion\":\"2026-11-26T00:00:00Z\",\"cantidad_eventos\":0}\n")
        .expect_err("Debe rechazar versión 2");
    assert!(err.mensaje.contains('2'));
    assert!(err.mensaje.contains('1'));
    // Evitar warnings de importación no usada si cambian helpers
    let _ = std::mem::size_of::<EventoLibro>();
    let _ = cifrar_con_parametros as fn(&[u8], &str, ParametrosKdf) -> _;
}
