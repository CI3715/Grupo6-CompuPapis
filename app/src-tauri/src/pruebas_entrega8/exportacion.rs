use super::*;
use crate::respaldo::exportar_a_archivo;

#[test]
fn exportar_libro_vacio() {
    let dir = DirTemp::nuevo("libro_vacio");
    let repo = bd_en(dir.path(), "libro");
    let ruta = dir.path().join("respaldo.ndjson");

    // Exportamos sin cifrar para leer el archivo
    exportar_a_archivo(&repo, &ruta, false, None, None).unwrap();

    let texto = std::fs::read_to_string(&ruta).unwrap();
    let (encabezado, eventos) = parsear_eventos_ndjson(&texto).unwrap();

    assert_eq!(encabezado.formato, "cuentas-claras-libro");
    assert_eq!(encabezado.version, 1);
    assert_eq!(encabezado.cantidad_eventos, 0);
    assert!(eventos.is_empty());
    assert!(repo.obtener_eventos().unwrap().is_empty());
}

#[test]
fn exportar_conserva_eventos() {
    let dir = DirTemp::nuevo("exportar_eventos");
    let mut repo = bd_en(dir.path(), "libro");
    crear_cuenta_y_movimientos_base(&mut repo);

    // Guardamos los eventos para compararlos luego
    let antes = repo.obtener_eventos().unwrap();
    let ruta = dir.path().join("respaldo.ndjson");

    exportar_a_archivo(&repo, &ruta, false, None, None).unwrap();

    let texto = std::fs::read_to_string(&ruta).unwrap();
    let (encabezado, eventos) = parsear_eventos_ndjson(&texto).unwrap();

    //Corroboramos los datos del encabezado
    assert_eq!(encabezado.formato, "cuentas-claras-libro");
    assert_eq!(encabezado.version, 1);
    assert_eq!(encabezado.cantidad_eventos, 3);
    assert!(chrono::DateTime::parse_from_rfc3339(&encabezado.fecha_exportacion).is_ok());
    assert_eq!(eventos.len(), antes.len());

    // Vemos si los eventos siguien siendo los mismos
    for ev in &antes {
        let exp = eventos.iter().find(|e| e.id == ev.id).unwrap();

        assert_eq!(exp.tipo, ev.tipo);
        assert_eq!(exp.momento_registro, ev.momento_registro);
        assert_eq!(exp.dispositivo_origen, ev.dispositivo_origen);

        let datos: serde_json::Value = serde_json::from_str(&ev.datos_json).unwrap();
        let datos_exp: serde_json::Value = serde_json::from_str(&exp.datos_json).unwrap();

        assert_eq!(datos, datos_exp);
    }

    // Chequeamos que el libro no haya cambiado
    assert_eq!(repo.obtener_eventos().unwrap(), antes);
}

#[test]
fn reconstruir_saldo_exportado() {
    use crate::domain::ejecutar_corregir_movimiento;

    let dir = DirTemp::nuevo("reconstruir_saldo");
    let mut repo = bd_en(dir.path(), "libro");
    let cid = crear_cuenta_y_movimientos_base(&mut repo);
    let disp = repo.dispositivo_id().unwrap();

    let det = ejecutar_obtener_detalle_cuenta(&repo, &cid).unwrap();
    let gasto = det.movimientos.iter().find(|m| m.tipo == "gasto").unwrap();

    // Cambiamos el monto del gasto
    let det = ejecutar_corregir_movimiento(&mut repo, &cid, &gasto.id, "75.00", &disp).unwrap();

    let ruta = dir.path().join("respaldo.ndjson");
    exportar_a_archivo(&repo, &ruta, false, None, None).unwrap();

    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/reconstruir_saldos.mjs");

    // Probamos la herramienta con el archivo que exportamos
    let salida = Command::new("node")
        .arg(script)
        .arg(&ruta)
        .output()
        .unwrap();

    assert!(salida.status.success());

    let datos: serde_json::Value = serde_json::from_slice(&salida.stdout).unwrap();

    assert_eq!(datos["cantidadEventos"], 4);
    assert_eq!(datos["cuentas"].as_array().unwrap().len(), 1);
    assert_eq!(datos["cuentas"][0]["id"].as_str().unwrap(), cid.as_str());

    // Vemos si nos da el mismo saldo
    let saldo = datos["cuentas"][0]["saldo"].as_str().unwrap();
    assert_eq!(saldo, det.saldo_actual.as_str());
    assert_eq!(saldo, "176.67");
}

#[test]
fn importar_archivos_invalidos() {
    use crate::respaldo::importar_desde_archivo;

    let dir = DirTemp::nuevo("archivos_invalidos");
    let mut origen = bd_en(dir.path(), "origen");
    let mut destino = bd_en(dir.path(), "destino");

    crear_cuenta_y_movimientos_base(&mut origen);

    let disp = destino.dispositivo_id().unwrap();
    ejecutar_crear_cuenta(
        &mut destino,
        "Cuenta local",
        "efectivo",
        "USD",
        "50.00",
        &disp,
    )
    .unwrap();

    let antes = destino.obtener_eventos().unwrap();
    let ruta = dir.path().join("respaldo.ndjson");

    exportar_a_archivo(&origen, &ruta, false, None, None).unwrap();

    let texto = std::fs::read_to_string(&ruta).unwrap();
    let mut lineas: Vec<String> = texto.lines().map(str::to_string).collect();
    let n = lineas.len() - 1;

    // Cambiamos la version y probamos con un evento menos
    let version = texto.replacen("\"version\":1", "\"version\":99", 1);
    let incompleto = lineas[..n].join("\n");

    // Le quitamos los datos al ultimo evento
    let mut ev: serde_json::Value = serde_json::from_str(&lineas[n]).unwrap();
    ev["datos"] = serde_json::json!({});
    lineas[n] = ev.to_string();
    let sin_datos = lineas.join("\n");

    lineas[n] = "{".to_string();
    let mal_json = lineas.join("\n");

    // Vemos si alguno de estos archivos cambia el libro
    for archivo in [version, incompleto, sin_datos, mal_json] {
        std::fs::write(&ruta, archivo).unwrap();
        assert!(importar_desde_archivo(&mut destino, &ruta, None).is_err());
        assert_eq!(destino.obtener_eventos().unwrap(), antes);
    }

    // Probamos ahora con el archivo sin modificar
    std::fs::write(&ruta, texto).unwrap();
    let res = importar_desde_archivo(&mut destino, &ruta, None).unwrap();

    assert_eq!(res.nuevos, 3);
    assert!(res.rechazados.is_empty());
}

#[test]
fn exportar_importar_moneda_propia() {
    use crate::domain::{
        ejecutar_corregir_movimiento, ejecutar_crear_categoria, ejecutar_listar_categorias,
        ejecutar_listar_monedas, ejecutar_listar_tasas, ejecutar_obtener_moneda_base,
        eventos_equivalentes,
    };
    use crate::respaldo::importar_desde_archivo;
    use crate::tasas_monedas::{
        ejecutar_cambiar_moneda_base, ejecutar_crear_moneda, ejecutar_eliminar_tasa,
        ejecutar_registrar_tasa,
    };

    let dir = DirTemp::nuevo("moneda_propia");
    let mut repo = bd_en(dir.path(), "origen");
    let disp = repo.dispositivo_id().unwrap();
    let reloj = crate::RelojSistema;
    let ids = crate::GeneradorUuid;

    // Creamos una moneda con 8 decimales
    let moneda =
        ejecutar_crear_moneda(&mut repo, "BTC", "Bitcoin", "BTC", 8, &disp, &reloj, &ids).unwrap();

    let cat = ejecutar_crear_categoria(&mut repo, "Compras", "gasto", &disp).unwrap();

    let cuenta = ejecutar_crear_cuenta(
        &mut repo,
        "Billetera BTC",
        "efectivo",
        "BTC",
        "1.23456789",
        &disp,
    )
    .unwrap();

    let cid = cuenta.id();

    // Hacemos un gasto y lo corregimos
    let detalle = ejecutar_registrar_movimiento(
        &mut repo,
        RegistrarMovimiento {
            cuenta_id: cid,
            tipo_str: "gasto",
            monto_str: "0.12345678",
            fecha_str: "2026-11-20",
            categoria_str: "Compras",
            nota_opt: Some("Compra".into()),
            dispositivo_id: &disp,
        },
    )
    .unwrap();

    let mov = &detalle.movimientos[0].id;

    ejecutar_corregir_movimiento(&mut repo, cid, mov, "0.12345679", &disp).unwrap();

    let tasa = ejecutar_registrar_tasa(
        &mut repo,
        "BTC",
        "USD",
        "2.12345678",
        "2026-11-20",
        "Prueba",
        &disp,
        &reloj,
        &ids,
    )
    .unwrap();

    ejecutar_registrar_tasa(
        &mut repo,
        "BTC",
        "USD",
        "3.12345678",
        "2026-11-21",
        "Prueba",
        &disp,
        &reloj,
        &ids,
    )
    .unwrap();

    ejecutar_eliminar_tasa(&mut repo, &tasa.id, &disp, &reloj, &ids).unwrap();
    ejecutar_cambiar_moneda_base(&mut repo, "BTC", &disp, &reloj, &ids).unwrap();

    let antes = repo.obtener_eventos().unwrap();
    let ruta = dir.path().join("respaldo.ndjson");

    // Exportamos e importamos en otro libro
    exportar_a_archivo(&repo, &ruta, false, None, None).unwrap();

    let mut destino = bd_en(dir.path(), "destino");
    let res = importar_desde_archivo(&mut destino, &ruta, None).unwrap();

    assert_eq!(res.nuevos, antes.len());
    assert!(res.rechazados.is_empty());

    let despues = destino.obtener_eventos().unwrap();
    assert_eq!(antes.len(), despues.len());

    // Vemos si se perdio algun dato
    for ev in &antes {
        let imp = despues.iter().find(|e| e.id == ev.id).unwrap();
        assert!(eventos_equivalentes(ev, imp));
    }

    assert!(ejecutar_listar_monedas(&destino).unwrap().contains(&moneda));
    assert!(ejecutar_listar_categorias(&destino).unwrap().contains(&cat));

    let tasas = ejecutar_listar_tasas(&destino).unwrap();
    assert_eq!(tasas.len(), 1);
    assert_eq!(tasas[0].valor.to_string(), "3.12345678");
    assert_eq!(ejecutar_obtener_moneda_base(&destino).unwrap(), "BTC");

    // Chequeamos que el saldo siga siendo el correcto
    let detalle = ejecutar_obtener_detalle_cuenta(&destino, cid).unwrap();
    assert_eq!(detalle.saldo_actual, "1.11111110");
}
