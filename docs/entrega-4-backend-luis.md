# Entrega 4: auditoría, integración y backend de Luis

## Fuentes y alcance

Se leyeron completas las especificaciones de las Entregas 1, 2, 3 y 4. La
Entrega 4 exige operaciones locales, eventos inmutables y decimales exactos;
Axum conserva únicamente el ping explícito. No se añaden proveedores de tasas,
cruces, transferencias, sincronización ni registro de usuarios.

## Integración revisada

Al hacer fetch el 28/10/2026, el árbol estaba limpio y no existía
`4-backend-luis`. Remoto: `https://github.com/CI3715/Grupo6-CompuPapis.git`.

| Referencia revisada        | Commit    | Aporte y decisión                                                                                                                                                  |
| -------------------------- | --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `dev` anterior             | `315e9aa` | Respaldado en `backup/dev-antes-entrega-4-20261028`; 0 commits exclusivos frente a main y 14 por detrás.                                                           |
| `main` / `origin/main`     | `31b18e6` | Integración de Entregas 2/3, libro compatible, SQLite inmutable, regresiones, guía y trabajo inicial de tasas. Incorporado por fast-forward, sin modificar main.   |
| `origin/gabriel-tasas`     | `684ac35` | Ancestro de main mediante PR #9: monedas, tasas, proyección, comandos iniciales y pruebas de moneda propia. No requería otro merge.                                |
| `origin/front`             | `ed2427c` | Un commit exclusivo desde `2d1e1ca`: pantallas Tasas/Patrimonio, navegación y tipos. Merge `e2b28d4`.                                                              |
| `origin/e4-backend-keyber` | `a87f2e3` | Dos commits propios desde Gabriel: validación y progreso de catálogo/base/decimales dinámicos. Merge `b88e1f4`; se conserva su autoría. No contiene consolidación. |

Se compararon grafo, ascendencia, diffs de contenido y `git cherry`: Gabriel
ya estaba integrado por ascendencia, front tenía un parche propio y los
cambios de Keyber combinaban ajustes semánticos, comentarios y formato sobre
el mismo dominio de Gabriel. No se sustituyeron archivos de backend por los
más antiguos del punto de partida de front. Los merges no produjeron
conflictos textuales. Otras ramas históricas se inspeccionaron en el grafo;
no se mezclaron versiones anteriores del dominio dividido en paralelo.

La revisión de contenido detectó que front reintroducía Google Fonts y el
layout de plantilla: el primer build falló por descarga de fuentes. Se
restauró el layout local/español de main y se normalizó el formato de cinco
archivos importados en `76d35f1`. Ese es el **dev final y base de la implementación**.
La rama de Luis se creó inicialmente en `b88e1f4` y avanzó por fast-forward
al `76d35f1` antes de confirmar su implementación. No hubo reset, rebase,
push, PR, cambio de main ni acceso a bases personales.

## Auditoría del código y del cableado

Rutas relativas a la raíz. “Inicial” corresponde al código integrado antes
de completar los comandos de Luis; “final” al código de esta contribución.

| Requisito                                                                   | Estado inicial → final                                                               | Evidencia en archivos                                                                                                                                                                             | Área responsable                                                                                 |
| --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| Registro de tasas, validaciones y campos ausentes                           | Parcial → implementado                                                               | `app/src-tauri/src/domain.rs`: `validar_tasa`, `TasaCambio`; `tasas_monedas.rs`: `EntradaTasa`, `registrar_tasa_desde_entrada`; `lib.rs`: comando registrado                                      | Luis, dominio/backend local; base previa de Gabriel                                              |
| Anulación por evento sin cambiar movimientos                                | Implementado sin pruebas específicas → implementado y probado                        | `domain.rs`: proyección `TasaEliminada`; `tasas_monedas.rs`: `ejecutar_eliminar_tasa`; `pruebas_tasas_luis.rs`                                                                                    | Luis, dominio/backend local; base previa de Gabriel                                              |
| Historial por par y desempate estable                                       | Parcial → implementado                                                               | `domain.rs`: `ejecutar_listar_tasas`; `tasas_monedas.rs`: `ejecutar_historial_tasas`; `lib.rs`: `listar_tasas`, `historial_tasas`                                                                 | Luis, dominio/backend local                                                                      |
| Catálogo propio, base USD y cambio de base, cuentas con decimales dinámicos | Implementado; contrato de decimales incompatible → cableado corregido y probado      | `domain.rs`: `MonedaCreada`, `MonedaBaseCambiada`, `Monto`; `tasas_monedas.rs`: casos de uso existentes; `lib.rs`: `crear_moneda`, `listar_monedas`, `cambiar_moneda_base`, `obtener_moneda_base` | Keyber, dominio; Gabriel, base previa; Luis, contrato y cableado                                 |
| Patrimonio por fecha, INV-02, inversa, ausencia de cruces y redondeo final  | Pendiente → motor mínimo implementado y probado                                      | `domain.rs`: `ejecutar_consultar_patrimonio`; `lib.rs`: comando registrado; `pruebas_tasas_luis.rs`: ejemplo 2.3 y pruebas compartidas                                                            | Motor previsto para Keyber; dependencia indispensable completada aquí, identificada por separado |
| Guardado exacto y replay de eventos E2/E3/E4                                | Implementado genérico, E4 sin prueba → implementado y probado                        | `sqlite_repo.rs`: INSERT, orden `rowid`, triggers; `pruebas_integracion.rs`; `pruebas_tasas_luis.rs`: reaperturas                                                                                 | Persistencia SQLite, Gabriel/equipo; Luis, pruebas. Sin nuevo esquema/repositorio                |
| Pantallas, formularios, historial agrupado y tasas usadas/faltantes         | Parcial → comandos conectados; validación visual/manual pendiente                    | `app/app/tasas.tsx`, `patrimonio.tsx`, `page.tsx`, `app/lib/tipos.ts`; `lib.rs`: `generate_handler!`                                                                                              | Frontend                                                                                         |
| Presentación de monedas con 0–8 decimales                                   | Parcial → precisión recibida conservada; ajuste visual para cero decimales pendiente | `app/lib/money.ts`: ya admite hasta 28 posiciones para conversiones, sin Number; conserva mínimo visual de 2 posiciones                                                                           | Frontend; corrección mínima de Luis para evitar error al recibir monedas propias/conversiones    |
| Autonomía, cierre/reapertura de la ventana y flujo completo                 | Pendiente → persistencia automatizada aprobada; demostración manual pendiente        | `docs/pruebas/README.md`, suites SQLite, `ping_servidor` separado                                                                                                                                 | Prueba manual del equipo                                                                         |

No quedan dependencias de Keyber que bloqueen estas pruebas. Su motor de
consolidación no existía en las referencias consultadas: se añadió únicamente
la consulta requerida, sin sustituir su catálogo, moneda base ni casos de
uso de monedas propias. La revisión y evolución de ese motor siguen siendo
trabajo coordinado con Keyber, no un trabajo previo suyo declarado terminado.

## Decisiones explícitas de aplicación

- El libro conserva `TasaRegistrada` y `TasaEliminada`. Historial y listado
  devuelven **tasas activas**; es una decisión de esta implementación, no una
  exigencia adicional del PDF. No se introduce una pantalla de auditoría.
- Orden: fecha civil descendente, y para igual fecha el último evento
  insertado primero. SQLite devuelve `rowid ASC`; memoria conserva inserción.
  No se desempata con UUID ni con marcas temporales que podrían coincidir.
- Eliminar una tasa inexistente o ya anulada falla con `campo: "tasa_id"`,
  respetando la convención existente. No es un éxito idempotente del comando;
  el replay del mismo evento sí es idempotente.
- La consulta usa tasas activas con fecha <= consulta. Una tasa anulada queda
  excluida incluso de consultas pasadas. No se implementa “lo que se sabía
  en aquel momento”. Una directa vigente tiene prioridad; si no existe, se
  usa la inversa vigente. No se buscan rutas por terceras monedas.
- La fecha de creación de una cuenta se obtiene de `momento_registro` local;
  antes de ese día no aparece. Su saldo al día consultado incluye inicial y
  movimientos/ajustes cuya fecha civil es <= consulta. Una corrección posterior
  no cambia el saldo consultado de un día anterior.
- Catálogo y moneda base corresponden a la configuración actual. Elegir otra
  fecha no revierte preferencias. Cambiar la base no toca los eventos de cuentas.
- Importes, tasas y conversiones viajan como texto decimal. Las conversiones
  por fila/subtotal se devuelven sin redondeo de presentación; solo el total
  consolidado se redondea al final (`rust_decimal`, midpoint nearest-even).
  Se divide el saldo por la tasa inversa directamente, evitando redondear antes
  `1 / tasa`. Desbordamientos devuelven error; no producen un total parcial.
- No se inventa un total cuando faltan tasas, incluso si existe una ruta cruzada.
  Se devuelve `total: null` y todos los pares faltantes, junto a saldos y subtotales.
- Las tasas nunca se envían al servidor. Todos los comandos financieros son
  síncronos y usan el mismo `Mutex<RepositorioSqlite>`; solo el ping usa red.

## Contratos Tauri verificados contra front

Todos los errores de dominio tienen `{ campo: string, mensaje: string }`.
Los nombres de argumentos IPC son camelCase; los comandos conservan snake_case.

| Comando                | Argumentos de invoke                                                                             | Respuesta                                                                                        |
| ---------------------- | ------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------ |
| `registrar_tasa`       | `{ origen, destino, valor, fecha, fuente }`, todos texto; ausencia/null devuelve error del campo | `Tasa { id, origen, destino, valor: string, fecha, fuente }`                                     |
| `eliminar_tasa`        | `{ tasaId: string }`                                                                             | `null`                                                                                           |
| `listar_tasas`         | ninguno                                                                                          | `Tasa[]` activas en orden de historial; usado por front                                          |
| `historial_tasas`      | `{ origen: string, destino: string }`                                                            | `Tasa[]` activas del par, valida catálogo y par distinto                                         |
| `listar_monedas`       | ninguno                                                                                          | `{ codigo, nombre, simbolo, decimales: number }[]`, fábrica + propias                            |
| `crear_moneda`         | `{ codigo, nombre, simbolo, decimales: string }`                                                 | moneda creada; decimales se valida como entero 0–8, tal como lo envía front                      |
| `obtener_moneda_base`  | ninguno                                                                                          | código, USD por defecto                                                                          |
| `cambiar_moneda_base`  | `{ codigo: string }`                                                                             | código persistido                                                                                |
| `consultar_patrimonio` | `{ fecha?: string }`, ISO civil; omitida/null usa hoy local                                      | `{ monedaBase, fecha, filas, subtotales, total: string \| null, faltantes: [{origen,destino}] }` |

Cada fila contiene `{ cuentaId, nombre, moneda, saldo: string, tasa, inversa,
enBase: string | null }`. Cada subtotal contiene `{ codigo, simbolo,
subtotal: string, tasa, inversa, enBase }`. `tasa` es `Tasa | null`; incluye
el valor, fecha y fuente originales, también al aplicar inversa. El id extra
es compatible estructuralmente con `TasaUsada` de front.

Los argumentos presentes con un tipo JSON incorrecto siguen sujetos a la
validación de transporte de Tauri. Para registro de tasas, los campos
**ausentes o null** pasan como `Option` a la aplicación y producen el error
estructurado por campo. La suite prueba esta entrada compartida, no simula
un runtime IPC ni una ventana.

## Verificación

La guía permanente contiene los comandos y el flujo manual:
[pruebas/README.md](pruebas/README.md#entrega-4-monedas-y-tasas).
Las pruebas automatizadas no sustituyen la demostración de escritorio sin
red. No se omiten tests fallidos ni se marca esa demostración como aprobada.

### Resultados ejecutados (28/10/2026)

| Comprobación                                      | Resultado                                                                                           |
| ------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| dev final aislado en worktree                     | 30 pruebas Rust + 1 doctest, lint, 3 pruebas monetarias y build Next aprobados, sin cambios de Luis |
| Rama de Luis, `cargo test`                        | 38 pruebas + 1 doctest aprobados, 0 fallidas, 0 ignoradas; incluye las 8 nuevas de Luis             |
| `cargo clippy --all-targets -- -D warnings`       | Aprobado en Tauri                                                                                   |
| Servidor                                          | 1 prueba del contrato de ping y formato aprobados                                                   |
| `npm test`, lint y `tsc --noEmit`                 | 4 pruebas aprobadas, lint y TypeScript sin errores                                                  |
| `npm run format:check` y `git diff --check`       | Aprobados                                                                                           |
| `npm run build`                                   | Exportación estática aprobada                                                                       |
| `npm run tauri -- build --debug --no-bundle`      | Ejecutable debug generado; no se abrió la ventana                                                   |
| Demostración con red desactivada/servidor apagado | Pendiente manual; la persistencia y el dominio sí se comprobaron automáticamente                    |

Incidencias corregidas durante la verificación: el build inicial falló al
descargar Google Fonts; Prettier detectó cinco archivos importados sin
formato; Clippy estricto detectó advertencias heredadas (inicializaciones,
rango, comparación booleana y posición del módulo de pruebas). Se corrigieron
y se repitieron los checks. Se preservaron las firmas existentes que reciben
reloj/identificadores con excepciones puntuales de `too_many_arguments`, sin
desactivar Clippy globalmente. El primer build del worktree temporal falló
porque Turbopack no permite resolver su `node_modules` fuera de la raíz;
se copió la dependencia local y el build aislado pasó. Ninguna prueba de
comportamiento fallida fue ocultada o excluida.
