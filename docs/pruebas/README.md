# Guía de pruebas de Cuentas Claras

Esta guía explica cómo ejecutar las pruebas de las Entregas 2, 3 y 4, qué comprueba cada grupo y cómo demostrar el funcionamiento en la aplicación. Complementa el [README del proyecto](../../README.md) y el [informe de verificación de la integración](../entregas-2-3-verificacion.md).

## 1. Preparación y directorios

Se necesitan Rust/Cargo, Node.js 22.18 o posterior, npm y las dependencias del sistema de Tauri 2. La primera instalación o compilación puede necesitar internet para descargar dependencias; el libro financiero de la aplicación funciona localmente.

Clonar el repositorio y entrar en su raíz:

```bash
git clone https://github.com/CI3715/Grupo6-CompuPapis.git cuentas-claras
cd cuentas-claras
```

En esta guía, **raíz** significa la carpeta del repositorio que contiene `app/`, `server/` y `README.md`. Los bloques son independientes: antes de ejecutar uno, situarse en el directorio indicado. Si se está en `app/`, `cd ..` permite volver a la raíz.

Comprobar el estado del checkout sin cambiar de rama:

```bash
git branch --show-current
git status --short
```

No se necesita una rama especial para ejecutar las pruebas. Si hay cambios locales, conservarlos antes de cambiar de checkout.

Para instalar las dependencias JavaScript, **partiendo de la raíz**:

```bash
cd app
npm ci
```

Las pruebas automáticas de Rust no necesitan abrir Tauri ni arrancar Axum. Los tests de dominio usan memoria; los de persistencia crean archivos SQLite temporales con nombres únicos, sin utilizar la base personal.

## 2. Verificación automática completa

### Rust: ejecutar desde la raíz

```bash
cargo check --manifest-path app/src-tauri/Cargo.toml
cargo test --manifest-path app/src-tauri/Cargo.toml
cargo fmt --manifest-path app/src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path app/src-tauri/Cargo.toml -- -D warnings

cargo check --manifest-path server/Cargo.toml
cargo test --manifest-path server/Cargo.toml
cargo fmt --manifest-path server/Cargo.toml -- --check
cargo clippy --manifest-path server/Cargo.toml -- -D warnings
```

### Frontend: partir de la raíz y entrar en app

```bash
cd app

npm test
npm run lint
npm run format:check
npm run build
```

Ejecutar los comandos uno por uno. Si alguno falla, guardar su salida completa y corregir la causa antes de considerar aprobada la verificación.

| Comando                           | Qué comprueba                                                                                                   | Resultado esperado                                    |
| --------------------------------- | --------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| `cargo check`                     | Que el paquete Rust y sus dependencias compilan para comprobación de tipos. No ejecuta pruebas.                 | Termina con `Finished` sin errores.                   |
| `cargo test` de Tauri             | Validaciones, invariantes, saldos, compatibilidad, persistencia, migración y protección de moneda.              | 38 pruebas unitarias y 1 doctest aprobados.           |
| `cargo test` del servidor         | El contrato del manejador de ping: estado, mensaje, versión y timestamp válido. No hace una petición HTTP real. | 1 prueba aprobada.                                    |
| `cargo fmt ... -- --check`        | Formato Rust sin modificar archivos.                                                                            | Termina sin diferencias; normalmente no imprime nada. |
| `cargo clippy ... -- -D warnings` | Análisis estático Rust; las advertencias se consideran errores.                                                 | Sin errores ni advertencias.                          |
| `npm test`                        | Representación decimal exacta del dinero en la UI.                                                              | 4 pruebas aprobadas.                                  |
| `npm run lint`                    | Reglas de TypeScript/React y hooks mediante ESLint.                                                             | Sin errores ni advertencias.                          |
| `npm run format:check`            | Prettier del frontend y formato de ambos paquetes Rust. No modifica archivos.                                   | Todos los pasos aprobados.                            |
| `npm run build`                   | Compilación de Next.js, comprobación de TypeScript y exportación estática.                                      | Build exitoso; archivos generados en `app/out/`.      |

Los recuentos corresponden a los casos documentados; pueden aumentar cuando se agreguen pruebas. El doctest de moneda usa `compile_fail`: **pasa cuando el compilador rechaza modificar el campo privado**, que es el comportamiento esperado de INV-18. Los bloques que dicen `running 0 tests` para otros targets no significan que se omitieran las 38 pruebas de la biblioteca.

Node puede mostrar un aviso `MODULE_TYPELESS_PACKAGE_JSON` al inferir ESM en `money.ts`; no representa un fallo de las pruebas. Un error de descarga o una dependencia nativa de Tauri ausente bloquea la ejecución: no cuenta como prueba aprobada.

## 3. Qué prueban los casos automatizados

Los archivos Rust están en [`app/src-tauri/src/`](../../app/src-tauri/src/). La siguiente tabla identifica sus pruebas por nombre para poder buscarlas y ejecutarlas por separado.

### Cuentas y dominio: domain.rs

| Prueba                                                       | Qué hace                                                                                                                                                                                                                                         |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `probar_crear_cuentas`                                       | Crea cuentas de distintos tipos/monedas; comprueba nombre vacío/espacios/duplicado, tipo y moneda inválidos, saldo negativo y exceso de decimales. Verifica subtotal USD `0.10 + 0.20 = 0.30`, moneda conservada y replay de un evento repetido. |
| `test_ejemplo_seccion_2_2_correccion_e_inv_04_inv_05`        | Reproduce `1000 + 800 - 500 + 450 = 1750`; comprueba composición, original visible/corregido y referencia del ajuste.                                                                                                                            |
| `test_validaciones_registrar_movimiento`                     | Rechaza cero, negativos, exceso de decimales, categorías vacías/inexistentes/incompatibles y fecha futura.                                                                                                                                       |
| `test_validaciones_corregir_movimiento_y_compensacion_total` | Rechaza corrección negativa y corrección directa de ajustes; compensa a cero y vuelve a corregir el original desde su monto vigente.                                                                                                             |
| Doctest de `Cuenta`                                          | Intenta cambiar directamente la moneda y exige que el código no compile (INV-18).                                                                                                                                                                |

### Movimientos y categorías: pruebas_movimientos.rs

| Prueba                   | Qué hace                                                                                                                                                                                |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `probar_categorias`      | Verifica 7 categorías de gasto y 4 de ingreso, creación con espacios normalizados, rechazo de duplicados y aceptación del mismo nombre para otro tipo. Comprueba los eventos generados. |
| `probar_movimientos`     | Registra ingreso y gasto con categorías del mismo nombre y distinto tipo; comprueba saldos y eventos.                                                                                   |
| `probar_datos_invalidos` | Rechaza tipo, monto, fecha, categoría y cuenta inválidos. Verifica que los rechazos no agreguen eventos.                                                                                |
| `probar_saldos_monedas`  | Registra dos gastos y un ingreso en USD/VES y verifica subtotales independientes.                                                                                                       |
| `probar_evento_repetido` | Guarda otra vez el mismo evento y comprueba que el repositorio en memoria no lo duplica.                                                                                                |

### Regresiones de persistencia y compatibilidad: pruebas_integracion.rs

| Prueba                                                                        | Qué hace                                                                                                                                                                                              |
| ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `probar_error_correccion_ingreso_reportado`                                   | Lee un ingreso con formato histórico y lo corrige de 800 a 700; saldo final 1700, ajuste -100.                                                                                                        |
| `probar_correccion_gasto_reportado`                                           | Lee un gasto histórico de 500 y lo corrige a 50; con inicial 1000, saldo final 950, ajuste +450.                                                                                                      |
| `formatos_mixtos_persisten_sin_reescribir_eventos_y_saldo_1750`               | Reproduce el ejemplo con formatos históricos y actuales en SQLite, reabre la base, verifica categorías, composición, original y ajuste, y realiza nuevas correcciones sin cambiar eventos anteriores. |
| `correcciones_multiples_ingresos_y_gastos_preservan_original_inv04_inv05`     | Corrige ingresos y gastos por la secuencia 500 → 50 → 80 → 0 → 25, verificando saldo vigente e inmutabilidad del original.                                                                            |
| `validaciones_corregir_y_categorias_no_escriben_al_fallar`                    | Rechaza monto vacío/inválido/negativo/exceso de decimales/sin cambio, cuentas o movimientos inexistentes, movimiento de otra cuenta y categorías inválidas; comprueba que no se escriba al fallar.    |
| `inv01_inv03_inv18_cantidad_moneda_exactas_y_sin_mutadores`                   | Verifica cantidad + moneda, suma decimal exacta, rechazo de suma entre monedas distintas y moneda conservada después de registrar movimientos.                                                        |
| `fechas_movimiento_ordenadas_con_desempate_estable`                           | Registra fechas fuera de orden y repetidas; exige fecha descendente y último registro primero en empates.                                                                                             |
| `advertencia_decimal_exacta_incluso_mas_alla_de_number`                       | Evalúa un gasto que deja -0.01 usando cantidades superiores al rango exacto de Number; la evaluación no escribe eventos y el registro conserva el centavo.                                            |
| `eventos_incompatibles_y_referencias_invalidas_producen_error_explicito`      | Exige error ante eventos desconocidos, payloads incompatibles o referencias inválidas, evitando mostrar saldos parciales.                                                                             |
| `creacion_rechaza_texto_vacio_invalido_y_fuera_de_rango`                      | Rechaza saldos vacíos, no numéricos y fuera de rango sin agregar cuentas.                                                                                                                             |
| `fecha_civil_caracas_rechaza_manana_aunque_utc_ya_cambio_de_dia`              | Simula el cambio de día UTC cuando en Caracas todavía es el día anterior; rechaza mañana y acepta hoy/pasado.                                                                                         |
| `inv13_reproyectar_eventos_duplicados_no_duplica_movimientos_ni_ajustes`      | Reproyecta eventos repetidos con un repositorio de lectura: mantiene 1750 y tres movimientos; detecta un mismo id con contenido distinto.                                                             |
| `desbordamiento_no_agrega_eventos_ni_deja_saldos_parciales`                   | Intenta exceder el rango decimal y comprueba rechazo antes de guardar.                                                                                                                                |
| `id_reutilizado_con_otro_contenido_es_error_en_memoria_y_sqlite`              | Verifica que ambos repositorios acepten reintentar el mismo evento y rechacen reutilizar su id con otros datos.                                                                                       |
| `ajustes_historicos_de_ingresos_se_encadenan_y_admiten_correccion_actual`     | Aplica ajustes históricos -100 y +50 sobre un ingreso de 800, reconstruye vigente 750 y permite compensarlo a cero mediante el formato actual.                                                        |
| `subtotal_fuera_de_rango_se_rechaza_antes_de_guardar`                         | Rechaza cuentas o ingresos que desbordarían el subtotal de una moneda aunque la cuenta individual admita el monto.                                                                                    |
| `cambio_identificador_copia_wal_sin_alterar_original_ni_sobrescribir_destino` | Migra una base con WAL al directorio nuevo, conserva original y saldo 1750, y comprueba que un segundo intento no sobrescriba los datos nuevos.                                                       |
| `migracion_incompatible_no_publica_destino_y_conserva_origen`                 | Intenta migrar un libro incompatible; exige error, ausencia de destino publicado y origen intacto.                                                                                                    |

### SQLite, servidor y presentación

| Archivo / prueba                                         | Qué hace                                                                                                                                                                        |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `sqlite_repo.rs`: `probar_inmutabilidad_eventos`         | Intenta UPDATE y DELETE sobre un evento de una base temporal; exige que los triggers lo impidan y el evento permanezca.                                                         |
| `server/src/main.rs`: `ping_conserva_contrato_entrega_1` | Invoca el manejador de ping directamente y verifica estado, mensaje, versión y timestamp RFC3339.                                                                               |
| `app/tests/money.test.mjs`: centavos grandes             | Comprueba que el formato local conserva centavos por encima del rango exacto de Number y admite el máximo decimal representable.                                                |
| `app/tests/money.test.mjs`: formato, signos y cero       | Comprueba separadores venezolanos, signo negativo/positivo, símbolos y cero sin signo.                                                                                          |
| `app/tests/money.test.mjs`: texto inválido               | Exige rechazo de exceso de decimales, NaN, notación exponencial, vacío y coma como entrada al formateador canónico. Esto no cambia que el dominio acepte coma en el formulario. |

### Ejecutar un grupo o caso específico

Desde la raíz:

```bash
# Enumerar los casos disponibles sin ejecutarlos.
cargo test --manifest-path app/src-tauri/Cargo.toml -- --list

# Cuentas de Entrega 2.
cargo test --manifest-path app/src-tauri/Cargo.toml probar_crear_cuentas

# Movimientos y categorías.
cargo test --manifest-path app/src-tauri/Cargo.toml pruebas_movimientos

# Regresiones de persistencia, invariantes y compatibilidad.
cargo test --manifest-path app/src-tauri/Cargo.toml pruebas_integracion

# Ejemplo obligatorio del PDF.
cargo test --manifest-path app/src-tauri/Cargo.toml test_ejemplo_seccion_2_2

# Compatibilidad y reapertura de SQLite.
cargo test --manifest-path app/src-tauri/Cargo.toml formatos_mixtos

# Migración del directorio de datos.
cargo test --manifest-path app/src-tauri/Cargo.toml cambio_identificador

# Protección de SQLite ante UPDATE/DELETE.
cargo test --manifest-path app/src-tauri/Cargo.toml probar_inmutabilidad_eventos

# Doctest que protege la moneda de la cuenta.
cargo test --manifest-path app/src-tauri/Cargo.toml --doc
```

Cargo filtra por fragmentos del nombre. Revisar que el filtro realmente ejecutó casos: `0 passed` con todos filtrados no demuestra ese requisito.

## 4. Compilar y abrir la aplicación para pruebas manuales

Partiendo de la raíz, entrar en `app`:

```bash
cd app
npm run tauri -- build --debug --no-bundle
```

Compila un ejecutable debug con el frontend estático incluido, sin generar instaladores. En Linux se abre así, desde esa misma carpeta:

```bash
./src-tauri/target/debug/app
```

En otros sistemas, usar la ruta del ejecutable que imprime Tauri al terminar. No necesita que Next.js ni Axum estén ejecutándose para usar las cuentas. Compilar no demuestra que se hayan probado la ventana ni sus interacciones.

Para desarrollo también se puede usar `npm run tauri dev` desde `app`; ese modo inicia Next.js localmente. Abrir solo `npm run dev` en el navegador no permite comprobar los comandos Tauri.

## 5. Pruebas manuales y resultados esperados

Usar nombres nuevos para distinguir las cuentas de prueba. No borrar la base personal ni ejecutar SQL de limpieza: el libro es inmutable.

### Entrega 2: cuentas

| Acción                                                          | Qué debe ocurrir                                                                               |
| --------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| Abrir el ejecutable                                             | Aparece Mis cuentas sin login. Si no hay datos, invita a crear la primera cuenta.              |
| Crear efectivo VES, banco USD y billetera digital USDT          | Aparecen nombre, tipo, moneda y saldo; subtotales separados por moneda, sin total consolidado. |
| Probar también EUR y saldo inicial cero                         | La moneda está disponible y el cero se acepta.                                                 |
| Nombre vacío/espacios o igual al existente cambiando mayúsculas | Se rechaza con error de nombre.                                                                |
| Saldo `-1`, `abc` o `10.555`                                    | Se rechaza con error de saldo inicial; no se crea la cuenta.                                   |
| Cerrar y reabrir                                                | Cuentas y saldos permanecen.                                                                   |

La moneda desconocida y el tipo de cuenta inválido se comprueban automáticamente en el dominio: el selector normal de la UI solo ofrece valores admitidos. No hace falta alterar la base para probarlos.

### Entrega 3: ejemplo obligatorio

Crear una cuenta nueva llamada, por ejemplo, **Prueba entrega 3**, tipo billetera digital, moneda USD:

| Paso | Acción                                                               | Saldo esperado  |
| ---- | -------------------------------------------------------------------- | --------------- |
| 1    | Saldo inicial `1000.00`                                              | 1000.00 USD     |
| 2    | Registrar ingreso `800.00`, categoría Salario, fecha actual o pasada | 1800.00 USD     |
| 3    | Registrar gasto `500.00`, categoría Alimentación                     | 1300.00 USD     |
| 4    | Sobre ese gasto original, elegir Corregir e indicar `50.00`          | **1750.00 USD** |

Verificar composición: inicial 1000, ingresos 800, gastos 500, ajustes +450. El gasto original sigue visible como -500 y marcado como corregido; existe un ajuste +450 que referencia su identificador.

Cerrar y reabrir antes de continuar: el saldo y la relación deben mantenerse. Después se puede corregir el mismo original a 80 (saldo 1720), a cero (saldo 1800) y a 25 (saldo 1775). Estas correcciones agregan ajustes; no editan ni eliminan el original. Los ajustes no deben ofrecer el botón Corregir.

### Validaciones, categorías, advertencia e historial

| Acción                                                                                   | Qué debe ocurrir                                                                                                                     |
| ---------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Registrar al menos dos gastos y un ingreso en cuentas de monedas distintas               | Cada cuenta cambia exactamente por su monto y mantiene su moneda; subtotales separados.                                              |
| Movimiento con `0`, `-1` o `10.555`                                                      | Error de monto, sin registro nuevo.                                                                                                  |
| Fecha de mañana                                                                          | Error de fecha, sin registro nuevo.                                                                                                  |
| Crear categoría con nombre nuevo; repetir cambiando mayúsculas en el mismo tipo          | La primera se guarda, la duplicada se rechaza. El mismo nombre en el otro tipo se permite.                                           |
| Abrir formulario de ingreso/gasto                                                        | Las categorías ofrecidas corresponden al tipo. Las categorías inexistentes/cruzadas se prueban automáticamente invocando el dominio. |
| Gasto mayor que el saldo y cancelar la advertencia                                       | Antes de guardar aparece la advertencia; cancelar no agrega movimiento ni cambia saldo.                                              |
| Repetir el gasto y confirmarlo                                                           | Se registra y el saldo negativo es exacto.                                                                                           |
| Registrar primero un movimiento del día 20 y luego otro del día 10, usando un mes pasado | El día 20 aparece primero. En fechas iguales, aparece primero el último registrado.                                                  |
| Corregir con negativo, texto inválido o exceso de decimales                              | Error de monto; no se agrega ajuste.                                                                                                 |

### Funcionamiento offline y persistencia

1. Cerrar el ejecutable. Si Axum estaba abierto en una terminal, detenerlo con `Ctrl+C`.
2. Detener también Next.js si se estaba usando en desarrollo. Desactivar la red.
3. Abrir el ejecutable debug generado, no el navegador ni `tauri dev`.
4. Comprobar cuentas, categorías, saldos, historial, marcas de corregido y referencias de ajustes.
5. Crear otra cuenta, registrar un ingreso/gasto y corregirlo. Todo debe funcionar sin red ni login.
6. Cerrar y reabrir otra vez. Los nuevos datos también deben permanecer.

Las pruebas de reapertura del repositorio no sustituyen esta demostración del proceso y la ventana completos.

### Servidor y conexión de Entrega 1

Volver a habilitar la red si se necesita para el resto del entorno; el endpoint usa localhost. En una terminal, desde la raíz:

```bash
cargo run --manifest-path server/Cargo.toml
```

En otra terminal:

```bash
curl http://127.0.0.1:8000/ping
```

Debe devolver JSON con `estado: "ok"`, mensaje, versión y timestamp. En la aplicación, abrir **Conexión con el servidor · Entrega 1** y pulsar **Probar conexión**: debe mostrar esos datos y la latencia.

Apagar Axum con `Ctrl+C` y repetir el botón: debe informar que no está disponible. Crear/listar cuentas y registrar/corregir movimientos deben seguir funcionando. El ping solo se solicita con ese botón y no transporta datos financieros.

## 6. SQLite y actualización de datos existentes

La base se llama `cuentas_claras.sqlite`, dentro del `app_data_dir` de Tauri para `com.compupapis.cuentasclaras`. En Linux suele estar en:

```text
$XDG_DATA_HOME/com.compupapis.cuentasclaras/cuentas_claras.sqlite
```

Si no se definió `XDG_DATA_HOME`, normalmente es `~/.local/share/com.compupapis.cuentasclaras/cuentas_claras.sqlite`. Al iniciar desde terminal, la aplicación imprime la ruta efectiva. Ubicaciones predeterminadas en otros sistemas:

- macOS: `~/Library/Application Support/com.compupapis.cuentasclaras/cuentas_claras.sqlite`.
- Windows: `%APPDATA%\com.compupapis.cuentasclaras\cuentas_claras.sqlite`.

Al cambiar desde `com.tauri.dev`, la aplicación copia y verifica la base anterior si aún no existe la nueva. Conserva el origen y nunca sobrescribe un destino existente. Las pruebas automatizadas comprueban la copia con WAL, el rechazo de datos incompatibles y la preservación de eventos; la actualización sobre la base real del equipo debe comprobarse manualmente conservando una copia de respaldo y con la aplicación cerrada.

No borrar, editar ni eliminar eventos para preparar una demostración. Ante un error de integridad, conservar el archivo y reportar el mensaje; la aplicación evita mostrar un saldo parcial.

## 7. Cómo registrar el resultado

Para cada ejecución guardar: commit probado (`git rev-parse --short HEAD`), sistema operativo, comando o caso manual, resultado y salida/mensaje si falla. No marcar como aprobadas pruebas que no se ejecutaron.

Esta guía describe procedimientos y resultados esperados, no certifica una ejecución particular. Los resultados históricos están en el [informe de verificación](../entregas-2-3-verificacion.md). Las comprobaciones manuales de ventana, red apagada y base real del equipo siguen pendientes hasta que quien haga la demostración las ejecute y registre. La generación de instaladores release se prueba por separado con `npm run tauri build` desde `app`; no está incluida en el build debug sin bundle.

## Entrega 4: monedas y tasas

Auditoría, capas técnicas y contratos: [backend local de monedas, tasas y patrimonio](../entrega-4-backend.md).
Todos los comandos siguientes parten de la raíz del repositorio y funcionan
sin depender del nombre de una rama ni de directorios personales.

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml
cargo test --manifest-path app/src-tauri/Cargo.toml pruebas_tasas_cambio
cargo clippy --manifest-path app/src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path app/src-tauri/Cargo.toml -- --check
cargo test --manifest-path server/Cargo.toml
cargo fmt --manifest-path server/Cargo.toml -- --check
cd app
npm test
npm run lint
npx tsc --noEmit
npm run format:check
npm run build
npm run tauri -- build --debug --no-bundle
```

Resultado esperado del backend de Entrega 4: 38 pruebas unitarias Rust y 1
doctest; el filtro `pruebas_tasas_cambio` ejecuta 8 de ellas. Servidor: 1 prueba
del ping. Frontend: 4 pruebas de representación exacta. Todos deben pasar,
sin tests ignorados. Las suites anteriores de E2/E3 siguen incluidas y
comprueban el saldo 1750, formatos históricos, inmutabilidad e idempotencia.

| Caso automatizado de tasas y patrimonio                             | Evidencia esperada                                                                                                                                                |
| ------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `registro_exacto_validaciones_por_campo_y_rechazos_sin_eventos`     | Registro con 8 decimales exactos, catálogo personalizado, fuente recortada; rechazo por origen, destino, valor, fecha y fuente sin eventos nuevos.                |
| `entrada_comando_ausente_o_null_devuelve_campo_sin_escribir`        | Los cinco campos omitidos/null devuelven su nombre; un decimal demasiado largo no se redondea para aceptarlo.                                                     |
| `historial_desempate_anulacion_y_movimientos_intactos`              | Varias fechas y misma fecha, último registro primero; anulación por evento y rechazo de repetida/inexistente sin nuevas escrituras; detalle/movimientos intactos. |
| `ejemplo_23_inv02_inversa_sin_cruces_y_cambio_base`                 | 316100.00 VES, falta USDT→USD sin total pese a posible cruce, luego 350.00 USD; eliminación excluye la tasa; cuentas intactas.                                    |
| `tasa_vigente_y_saldos_a_fecha_redondeo_solo_al_final`              | Se excluyen tasas y movimientos posteriores; 0.004 + 0.004 se redondea a 0.01 al final.                                                                           |
| `sqlite_reapertura_conserva_tasas_anulaciones_y_rechazos`           | Tasa exacta tras reabrir, evento nuevo al anular, replay sin tasa activa, rechazos sin eventos.                                                                   |
| `sqlite_reapertura_conserva_catalogo_base_y_patrimonio`             | Moneda de 8 decimales, base, saldo y consulta idénticos tras reabrir.                                                                                             |
| `tasas_reproyeccion_idempotente_y_eventos_incompatibles_explicitos` | Guardar otra vez el mismo evento no duplica efectos; referencia incompatible produce error de almacenamiento explícito.                                           |

Las pruebas SQLite crean archivos temporales únicos; nunca abren ni borran
el libro personal de la aplicación. El repositorio existente sigue usando
solo INSERT para operaciones financieras y triggers contra UPDATE/DELETE.

### Demostración manual pendiente

1. Con un perfil de prueba vacío, abre la aplicación desde `app` con
   `npm run tauri -- dev`. No borres una base existente para preparar el caso;
   usa un usuario/perfil del sistema separado si necesitas datos limpios.
2. Crea Efectivo VES `88700`, Banco USD `200` y Billetera USDT `50`.
3. Abre Tasas. Registra USD→VES `887`, fecha `2026-10-20`, fuente `BCV`;
   USDT→VES `1000`, fecha `2026-10-21`, fuente `casa de cambio`.
4. Abre Patrimonio, elige fecha de hoy y base VES. Comprueba saldos por
   cuenta, subtotales, tasas usadas y total `316.100,00 VES`.
5. Cambia a USD. Debe aparecer el par faltante USDT→USD y ningún total
   consolidado. El saldo VES se convierte mediante la inversa de 887.
6. Registra USDT→USD `1`, fecha `2026-10-21`, fuente `estimación propia`.
   Patrimonio debe mostrar `350,00 USD`.
7. Elimina esa tasa. Debe desaparecer del historial activo y volver el aviso,
   sin alterar cuentas, movimientos ni saldos. Regístrala de nuevo para
   continuar. El libro conserva ambos eventos y no modifica el primero.
8. Registra otra USD→VES en la misma fecha y otra en fecha anterior. Comprueba
   el orden del historial y el desempate por último registro. Anula la más
   reciente y verifica que vuelva a aplicar la anterior correspondiente.
9. Intenta registrar sin fecha/fuente, con espacios en fuente, valor cero,
   negativo, más de 8 decimales, fecha futura y monedas iguales. Comprueba el
   campo del error y que el historial no aumente.
10. Crea una moneda `ABC`, nombre `Moneda de prueba`, símbolo `A`, decimales
    `3`; vuelve a Mis cuentas y crea una cuenta `ABC` con saldo `12.345`.
    Registra ABC→USD `2` y confirma su participación en Patrimonio.
11. Consulta una fecha pasada: solo deben aparecer cuentas ya creadas en ese
    día y sus movimientos/tasas hasta esa fecha. No cambian los saldos actuales.
12. Compila el ejecutable con el último comando anterior, cierra la aplicación,
    detén el servidor, desactiva la red y abre `app/src-tauri/target/debug/app`
    (en Windows/macOS usa el ejecutable de tu plataforma). Comprueba mismos
    datos, moneda base, tasas y patrimonio; registra/anula otra tasa sin red y
    vuelve a abrir. Esto está pendiente de demostración en una ventana real.
13. Por separado, levanta el servidor y verifica el ping de Entrega 1. Su caída
    debe afectar únicamente a ese indicador; los flujos financieros son locales.

Pendiente del frontend: ajustar la presentación de monedas propias con cero
posiciones (el formateador todavía muestra un mínimo de dos), revisar visualmente
formularios, cambios de base/errores y tamaños de ventana. El formato ya conserva
hasta 28 posiciones recibidas para conversiones sin pasar por Number; no debe
redondear cada fila antes de sumar.
