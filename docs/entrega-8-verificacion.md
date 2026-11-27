# Verificación automatizada de la Entrega 8

La ejecución de `cargo test --manifest-path app/src-tauri/Cargo.toml pruebas_entrega8 -- --test-threads=1` completó 24 pruebas con éxito, sin fallos ni pruebas ignoradas. Usa bases SQLite y destinos temporales, sin modificar el libro personal.

Los escenarios comprobados incluyen:

- Exportación sin alterar los eventos, encabezado y reconstrucción del saldo con una herramienta independiente.
- Cifrado y descifrado, frase incorrecta, archivo alterado, restauración de cuentas, tasas y saldos.
- Importación repetida y concurrente, rechazo de conflictos, conservación de eventos y rollback ante un fallo de SQLite.
- Sincronización incremental y convergencia entre libros separados, en ambos órdenes, con los adaptadores de memoria, carpeta y un repositorio Git local real. La repetición no duplica eventos.
- Reintentos y publicación fallida simulados, rutas inválidas y comprobaciones de privacidad en los lotes y la configuración.

El contrato de Git se ejecuta sobre un repositorio local temporal. La interrupción de publicación y los errores de conexión se simulan; esta ejecución no comprueba credenciales de Git remotas ni una interrupción real durante el envío. Tampoco acredita la interacción o presentación visual en ventanas Tauri. La [guía de sincronización](guia-sincronizacion-personal.md) describe el flujo con dos instalaciones.
