use super::*;

#[test]
fn publicacion_fallida_no_marca_lote_local_como_leido() {
    struct FalloPublicacion(TransporteMemoria);
    impl TransporteSincronizacion for FalloPublicacion {
        fn preparar(&mut self) -> Result<(), ErrorTransporte> {
            self.0.preparar()
        }
        fn listar_lotes(&self) -> Result<Vec<ReferenciaLote>, ErrorTransporte> {
            self.0.listar_lotes()
        }
        fn leer_lote(&self, lote: &ReferenciaLote) -> Result<Vec<u8>, ErrorTransporte> {
            self.0.leer_lote(lote)
        }
        fn escribir_lote(
            &mut self,
            dispositivo: &str,
            nombre: &str,
            datos: &[u8],
        ) -> Result<(), ErrorTransporte> {
            self.0.escribir_lote(dispositivo, nombre, datos)
        }
        fn publicar(
            &mut self,
            _: &str,
        ) -> Result<crate::sincronizacion_personal::ResultadoPublicacion, ErrorTransporte> {
            Err(ErrorTransporte::DestinoInaccesible("fallo simulado".into()))
        }
    }
    let dir = DirTemp::nuevo("fallo_publicacion");
    let mut repo = bd_en(dir.path(), "origen");
    crear_cuenta_y_movimientos_base(&mut repo);
    let mut transporte = FalloPublicacion(TransporteMemoria::nuevo());
    assert!(
        ejecutar_sincronizacion_personal(
            &mut repo,
            &mut transporte,
            "destino",
            FRASE_PRUEBA,
            ParametrosKdf::para_pruebas()
        )
        .is_err()
    );
    assert!(repo.lotes_leidos_destino("destino").unwrap().is_empty());
    assert!(repo.eventos_enviados_destino("destino").unwrap().is_empty());
    let mut recuperacion = transporte.0;
    ejecutar_sincronizacion_personal(
        &mut repo,
        &mut recuperacion,
        "destino",
        FRASE_PRUEBA,
        ParametrosKdf::para_pruebas(),
    )
    .unwrap();
    let mut otro = bd_en(dir.path(), "otro");
    ejecutar_sincronizacion_personal(
        &mut otro,
        &mut recuperacion,
        "destino",
        FRASE_PRUEBA,
        ParametrosKdf::para_pruebas(),
    )
    .unwrap();
    assert_eq!(otro.obtener_eventos().unwrap().len(), 3);
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
