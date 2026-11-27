use super::*;

#[test]
fn importar_cantidad_declarada_excesiva_rechaza_sin_reservar_memoria() {
    let texto = "{\"formato\":\"cuentas-claras-libro\",\"version\":1,\"fecha_exportacion\":\"2026-11-26T00:00:00Z\",\"cantidad_eventos\":18446744073709551615}\n";
    assert!(parsear_eventos_ndjson(texto).is_err());
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
