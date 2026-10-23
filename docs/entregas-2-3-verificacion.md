# Verificación de las Entregas 2 y 3

Fecha: 23 de octubre de 2026. Rama: `integration/entregas-2-3`.

## Integración y conservación del trabajo

- `main` y `origin/main`: `909b67f`; `dev` y `origin/dev`: `315e9aa`. Se actualizaron las referencias al comenzar y al finalizar. Las ramas originales no se modificaron.
- El merge `4e03dcc` tiene ambos commits como padres y conserva las historias completas. Los conflictos de `domain.rs` y `sqlite_repo.rs` se resolvieron semánticamente.
- De `dev`: UI de detalle, registro/corrección, categorías, composición, comandos y pruebas de movimientos. Se conservan sus nueve pruebas originales.
- De `main`: prueba de creación de cuentas, validaciones, subtotales exactos e idempotencia; regresiones de Maikel para correcciones de ingreso y gasto, adaptadas y conservadas con sus nombres en `pruebas_integracion.rs`.
- `58d7d3d` completa precisión, interoperabilidad del libro, `/ping`, compilación de escritorio y migración segura del directorio de datos.
- No se hizo push, no se abrió PR y no se fusionó nada en `main` ni se movió `dev`.

## Decisiones técnicas

Se conservó la implementación más completa de movimientos de `dev` y se incorporó un lector de los payloads históricos de `main`. Los movimientos históricos heredan moneda de su cuenta; las correcciones históricas se interpretan como efectos firmados y permiten posteriores correcciones con el formato nuevo. Un evento incompatible, una referencia inválida o una diferencia inconsistente detienen la proyección con error visible. No se presentan saldos parciales.

La evaluación del saldo negativo vive en Rust y utiliza `rust_decimal`. La UI solo presenta decimales mediante texto. Las operaciones que exceden el rango representable, incluidos subtotales, se rechazan antes de persistir. El historial se ordena por fecha civil y luego por orden inverso de registro. Rust proporciona también la fecha local predeterminada del formulario.

La protección de SQLite y la idempotencia se mantienen. Reutilizar un identificador con otro contenido es error. El identificador de aplicación pasó de `com.tauri.dev` (Tauri impide generar ejecutables con él) a `com.compupapis.cuentasclaras`. SQLite Backup copia y verifica el libro anterior, incluyendo WAL, sin alterar el origen ni sobrescribir un destino existente. Ver [ADR 007](adr/007-libro-local-compatible.md).

El flujo financiero no hace HTTP. `ping_servidor` solo se ejecuta desde su botón explícito y utiliza un estado independiente. El servidor conserva `GET /ping` y no recibe datos financieros.

## Verificaciones ejecutadas

| Comando                                                                | Resultado final                                                  |
| ---------------------------------------------------------------------- | ---------------------------------------------------------------- |
| `cargo check --manifest-path app/src-tauri/Cargo.toml`                 | PASÓ                                                             |
| `cargo test --manifest-path app/src-tauri/Cargo.toml`                  | PASÓ: 28 pruebas y 1 doctest de compilación negativa para INV-18 |
| `cargo fmt --manifest-path app/src-tauri/Cargo.toml -- --check`        | PASÓ                                                             |
| `cargo clippy --manifest-path app/src-tauri/Cargo.toml -- -D warnings` | PASÓ                                                             |
| `cargo check --manifest-path server/Cargo.toml`                        | PASÓ                                                             |
| `cargo test --manifest-path server/Cargo.toml`                         | PASÓ: 1 prueba del contrato de ping                              |
| `cargo fmt --manifest-path server/Cargo.toml -- --check`               | PASÓ                                                             |
| `cargo clippy --manifest-path server/Cargo.toml -- -D warnings`        | PASÓ                                                             |
| Desde `app`: `npm test`                                                | PASÓ: 3 pruebas de representación decimal                        |
| Desde `app`: `npm run lint`                                            | PASÓ, con cero advertencias de ESLint                            |
| Desde `app`: `npm run format:check`                                    | PASÓ, frontend y ambos paquetes Rust                             |
| Desde `app`: `npm run build`                                           | PASÓ, con comprobación de TypeScript y exportación estática      |
| Desde `app`: `npm run tauri -- build --debug --no-bundle`              | PASÓ, ejecutable con frontend incluido                           |
| `git diff --check`                                                     | PASÓ                                                             |

No se desactivaron reglas de ESLint ni de Clippy. El build usa soporte nativo de CSS y el comprobador TypeScript mediante API; no omite la comprobación de tipos. Las fuentes de sistema ya usadas por CSS evitan una descarga de Google Fonts durante el build. Las pruebas JavaScript emiten un aviso no bloqueante de Node por inferir ESM en `money.ts`.

Los fallos iniciales de formato/lint/Clippy y los bloqueos de build por loader CSS, salida del subproceso TypeScript e identificador predeterminado se corrigieron. La generación de instaladores y el build release no se ejecutaron.

Ejecutable Linux generado: `app/src-tauri/target/debug/app`. No se abrió la ventana durante esta verificación.

## Cumplimiento de Entrega 2

COMPLETO indica implementación y evidencia automatizada o inspección directa; las pruebas de interacción en escritorio se enumeran aparte.

| Requisito                                                              | Estado        | Evidencia                                                                      |
| ---------------------------------------------------------------------- | ------------- | ------------------------------------------------------------------------------ |
| Inicio en Mis cuentas, sin login                                       | COMPLETO      | `app/app/page.tsx`; ruta raíz exportada                                        |
| Crear efectivo, banco y billetera digital                              | COMPLETO      | `domain.rs`, prueba recuperada `probar_crear_cuentas`                          |
| VES, USD, EUR y USDT, símbolos y 2 decimales                           | COMPLETO      | `catalogo_monedas`                                                             |
| Nombre vacío/espacios o duplicado sin distinguir mayúsculas            | COMPLETO      | Dominio y pruebas en memoria                                                   |
| Moneda desconocida y saldo vacío/inválido/negativo/exceso de decimales | COMPLETO      | Dominio y pruebas en memoria                                                   |
| INV-01: monto inseparable de moneda                                    | COMPLETO      | `Monto` y regresiones de moneda/cantidad                                       |
| INV-03: dinero decimal exacto                                          | COMPLETO      | `rust_decimal`, suma 0.10 + 0.20, pruebas de centavos grandes y desbordamiento |
| INV-18: moneda inmutable                                               | COMPLETO      | Campo privado, sin mutador; doctest que exige rechazo al intentar modificarlo  |
| Lista, estado vacío, formato local y subtotal por moneda               | COMPLETO      | UI, formateador probado y proyección                                           |
| Comandos listar_cuentas/listar_monedas/crear_cuenta                    | COMPLETO      | Registro Tauri y llamadas desde UI                                             |
| SQLite rusqlite dentro de app_data_dir                                 | COMPLETO      | Inicialización Tauri; pruebas de persistencia                                  |
| CuentaCreada con id, tipo, momento, origen y datos                     | COMPLETO      | Evento persistido y conservado al reabrir                                      |
| Libro inmutable e idempotente                                          | COMPLETO      | Triggers SQLite, deduplicación y rechazo de colisiones de id                   |
| Persistencia al reabrir repositorio                                    | COMPLETO      | Pruebas SQLite con lectura de los mismos eventos                               |
| Datos financieros locales, sin HTTP ni PostgreSQL                      | COMPLETO      | Flujo UI → Tauri → dominio → repositorio local                                 |
| Reinicio de la ventana y demostración con red apagada                  | NO VERIFICADO | Pendiente manual con el ejecutable                                             |
| README con SQLite y pruebas                                            | COMPLETO      | README actualizado                                                             |

## Cumplimiento de Entrega 3

| Requisito                                                        | Estado        | Evidencia                                                                                                                      |
| ---------------------------------------------------------------- | ------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| Ingresos y gastos sobre cuentas existentes                       | COMPLETO      | Casos de uso y pruebas en memoria                                                                                              |
| Monto positivo, moneda heredada y precisión válida               | COMPLETO      | Validaciones del dominio y pruebas de rechazo                                                                                  |
| Fecha actual/pasada y rechazo de futura                          | COMPLETO      | Fecha civil local de Rust; prueba del cambio de día UTC en Caracas                                                             |
| Categorías predeterminadas exactas                               | COMPLETO      | 7 de gasto y 4 de ingreso; prueba de dev conservada                                                                            |
| Crear categorías y rechazar duplicados por tipo                  | COMPLETO      | Dominio y pruebas, mismo nombre permitido en tipos distintos                                                                   |
| Categoría obligatoria, existente y compatible                    | COMPLETO      | Pruebas de categorías vacías, inexistentes y cruzadas                                                                          |
| Evaluación exacta y confirmación previa de saldo negativo        | COMPLETO      | Comando Rust `evaluar_movimiento` conectado a advertencia; prueba de -0.01 con cantidades superiores al rango exacto de Number |
| Eventos MovimientoRegistrado/MovimientoCorregido/CategoriaCreada | COMPLETO      | SQLite y pruebas de ambos formatos                                                                                             |
| INV-04: original inmutable, visible y marcado                    | COMPLETO      | Pruebas de monto original y payloads; marca y referencia presentes en UI                                                       |
| Correcciones por diferencias referenciadas                       | COMPLETO      | Ajustes firmados para ingreso y gasto; regresiones de main                                                                     |
| Correcciones múltiples sobre valor vigente                       | COMPLETO      | Secuencias 500 → 50 → 80 → 0 → 25 para ingresos y gastos                                                                       |
| Corrección a cero; ajustes no corregibles                        | COMPLETO      | Pruebas de compensación y rechazos                                                                                             |
| Sin edición/eliminación en UI ni dominio                         | COMPLETO      | Inspección y triggers SQLite                                                                                                   |
| INV-05: saldo y composición desde el libro                       | COMPLETO      | Ejemplo obligatorio, ambas representaciones y reapertura                                                                       |
| Historial por fecha descendente y desempate estable              | COMPLETO      | Prueba de fechas fuera de orden y fechas iguales                                                                               |
| Comandos registrar_movimiento/corregir_movimiento                | COMPLETO      | Comandos registrados y conectados a UI                                                                                         |
| INV-13: replay sin duplicar movimientos/ajustes                  | COMPLETO      | Prueba con repositorio de lectura que devuelve eventos repetidos                                                               |
| Persistencia de cuentas/categorías/movimientos/ajustes           | COMPLETO      | Reapertura SQLite y migración con WAL                                                                                          |
| Compatibilidad main/dev sin reescribir eventos                   | COMPLETO      | Fixtures históricos, eventos actuales y correcciones entre formatos                                                            |
| Validaciones de RegistrarMovimiento y CorregirMovimiento         | COMPLETO      | Pruebas en memoria; sin escrituras ante rechazos                                                                               |
| Uso offline en ventana real y reinicio completo                  | NO VERIFICADO | Pendiente manual                                                                                                               |
| README con pruebas y ejemplo de 1750                             | COMPLETO      | Pasos y saldos documentados                                                                                                    |

El ejemplo obligatorio produce inicial 1000 + ingresos 800 − gastos 500 + ajustes 450 = **1750 USD**. El original mantiene -500, queda marcado y el ajuste +450 lo referencia. Esto se comprobó con eventos actuales, históricos y reapertura SQLite.

## Archivos modificados respecto a origin/main

- `README.md`.
- `app/app/globals.css`, `app/app/layout.tsx`, `app/app/page.tsx`.
- `app/lib/money.ts`, `app/tests/money.test.mjs`.
- `app/next.config.ts`, `app/package.json`.
- `app/src-tauri/Cargo.toml`, `app/src-tauri/tauri.conf.json`, `app/src-tauri/capabilities/default.json`.
- `app/src-tauri/src/domain.rs`, `lib.rs`, `sqlite_repo.rs`, `pruebas_movimientos.rs`, `pruebas_integracion.rs`.
- `server/src/main.rs`: se añade una prueba del contrato existente.
- `docs/adr/README.md`, `docs/adr/007-libro-local-compatible.md`.
- Este informe.

## Pendiente manual y límites

1. Abrir el ejecutable, comprobar el inicio sin login y crear los tres tipos de cuenta en las monedas de aceptación.
2. Interactuar con ingresos/gastos/categorías y validar errores, confirmación/cancelación de saldo negativo e historial por fecha.
3. Reproducir 1750 USD en la UI, corregir varias veces y a cero, verificar original y relaciones.
4. Cerrar la aplicación, apagar Axum, desactivar red y reabrir; verificar datos y nuevos registros sin Next.js.
5. Probar el botón de conexión con Axum encendido/apagado; no se ejecutó el recorrido real invoke → HTTP desde una ventana.
6. Probar actualización sobre la base personal del equipo. La migración se verificó con fixtures temporales y WAL; no se abrió ni modificó su base real.
7. Generar/probar instaladores release si son necesarios para la entrega. Se compiló un ejecutable debug con frontend incluido, sin instaladores.

No quedan funcionalidades identificadas de Entregas 2/3 pendientes de implementar. No se declara aprobada la demostración manual. Los eventos históricos sin fecha civil propia usan su momento de registro local para presentarse; no se puede recuperar una fecha original que nunca se guardó. La publicación segura de la copia requiere soporte de hard links en el sistema de archivos de datos y falla conservando el origen si no está disponible.

## Recomendación para el PR

Destino: `main`. Origen recomendado: `integration/entregas-2-3`. Título sugerido: `feat: complete local-first accounts and immutable ledger for deliveries 2 and 3`.

Antes de publicar, completar las comprobaciones manuales anteriores y confirmar autorización. Actualizar referencias y comprobar que los dos extremos remotos siguen siendo ancestros de la integración. Si alguien incorpora nuevos commits, integrarlos y volver a verificar antes de publicar.

```bash
git fetch origin --prune
git merge-base --is-ancestor origin/main integration/entregas-2-3
git merge-base --is-ancestor origin/dev integration/entregas-2-3
```

Solo después de autorización, publicar la rama de integración y abrir su PR hacia `main`. No hace falta mover `dev` para revisar esta integración. Si se desea actualizar también `dev`, se puede hacer fast-forward local a la integración después de comprobar que no avanzó por otra vía; su publicación requiere igualmente autorización.
