# Entrega 4: integración y verificación del backend local

Verificación: 29/10/2026. Alcance: backend Tauri/Rust, dominio y repositorio
SQLite existentes. El frontend no se modifica en esta integración.

## Referencias y aportes reales

| Referencia al comenzar     | Commit    | Estado                                                                                                                          |
| -------------------------- | --------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `origin/dev`               | `a72b518` | Contiene frontend, libro compatible E2/E3, tasas, comandos Tauri, consulta de patrimonio, documentación y parches de seguridad. |
| `origin/e4-backend-keyber` | `cf4214c` | Conserva siete commits exclusivos posteriores a `a87f2e3`, base común de ambas ramas.                                           |
| Respaldo local             | `cf4214c` | `backup/e4-backend-keyber-antes-integracion-20261029` conserva la referencia anterior.                                          |

Los commits `94954d8` y `a87f2e3` ya estaban integrados en `dev`. Los siete
posteriores añaden consultas históricas de cuentas, selección reutilizable de
tasas, una propuesta de consulta de patrimonio y pruebas de moneda propia,
cambio de base, fechas, inversas y ESC-01. Parte de esas funcionalidades ya
existía en `dev` con otros contratos: se unificaron, no se duplicaron los motores.

No se modificaron las ramas locales `main` ni `dev`. La rama de trabajo es
`e4-backend-keyber`; el merge `602a4c4` conserva ambas historias.

## Resolución semántica

- Conflictos textuales: `app/src-tauri/src/domain.rs` y `tasas_monedas.rs`.
  Se conservan errores estructurados, validaciones exactas, historial activo,
  consultas del catálogo y las simplificaciones compatibles con Clippy.
- El merge automático de `lib.rs` y `domain.rs` duplicaba
  `consultar_patrimonio` y su caso de uso, con DTO incompatibles. Quedó una sola
  consulta y un único registro Tauri, conservando el contrato consumido por
  `app/app/patrimonio.tsx`: `{ monedaBase, fecha, filas, subtotales, total,
faltantes }` y el argumento `fecha` opcional.
- `EstadoCuentaInterno::saldo_en_fecha` suma una vez los efectos firmados del
  libro. Corrige la doble inversión de signo de gastos del motor anterior.
  `calcular_saldos_en_fecha` se comparte con la consulta de patrimonio y el
  listado histórico. Las consultas excluyen cuentas que aún no existían en la fecha solicitada.
- `buscar_tasa_aplicable` opera sobre tasas activas en orden de inserción;
  conserva la última de la fecha vigente, directa antes de inversa. Informa
  el par faltante y no construye cruces.
- `convertir_monto_a_base` usa multiplicación comprobada o división directa
  del saldo por la tasa inversa. No multiplica por `1/tasa` aproximado ni
  redondea equivalentes por fila. El total se redondea una sola vez, al final.
- Se adaptaron las pruebas adicionales al DTO existente, conservando las
  comprobaciones de saldos, pares, fuentes, fechas y cantidades. Los fixtures
  históricos tienen fecha explícita de creación. Una función ESC-01 carecía
  de `#[test]`: se añadió para que participe en la suite.
- El repositorio SQLite y su esquema se conservan sin cambios. No se añade
  infraestructura alternativa ni se usa Axum para operaciones financieras.
- Se mantienen Next.js/ESLint 16.4.1 y Zod 4.6.7 del lockfile integrado.

## Requisitos contrastados con el PDF

| Requisito                                                                                                   | Estado del backend y persistencia                                | Evidencia                                                                                                               |
| ----------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| RF-CAM-01: catálogo propio, código único 3–5 mayúsculas, nombre/símbolo, escala 0–8, fábrica protegida      | Implementado y probado                                           | `domain.rs`, `tasas_monedas.rs`, `pruebas_tasas_monedas.rs`, `pruebas_patrimonio.rs`                                    |
| RF-CAM-02: base USD por defecto, cambio sin alterar movimientos/saldos                                      | Implementado y probado; VES persiste tras reabrir                | Proyección `MonedaBaseCambiada` y suite SQLite                                                                          |
| RF-CAM-03: registro con par válido/distinto, valor positivo de hasta 8 decimales, fecha/fuente obligatorias | Implementado y probado; campos ausentes/null dan error por campo | `EntradaTasa`, `validar_tasa`, `pruebas_tasas_cambio.rs`                                                                |
| RF-CAM-06 básico: varias tasas, historial por par, desempate y anulación                                    | Implementado y probado                                           | Historial descendente estable; `TasaEliminada` excluye la tasa sin reescribir eventos                                   |
| RF-CAM-04, RF-INF-01, INV-02: saldos/subtotales, tasas usadas y total solo si hay todos los pares           | Implementado y probado                                           | Contrato JSON real, consulta histórica y ESC-01 en memoria y SQLite                                                     |
| INV-03: `rust_decimal`, inversas sin cruces, redondeo final                                                 | Implementado y probado                                           | Selección vigente, división directa, total único y errores por desbordamiento                                           |
| Gastos, ajustes y compatibilidad E2/E3                                                                      | Corregido y probado                                              | 100 − 20 = 80; correcciones a 5/0; consultas anteriores; fixtures actuales e históricos                                 |
| Libro inmutable y persistencia                                                                              | Implementado y probado con archivos temporales                   | Triggers contra UPDATE/DELETE, INSERT idempotente, reapertura, catálogo/base/tasas, anulación y rechazos sin escrituras |
| Operaciones locales sin Axum                                                                                | Cableado comprobado, sin HTTP en finanzas                        | Todos los comandos financieros son síncronos y usan el estado SQLite; el ping conserva su estado de red separado        |
| Cierre de ventana y red desactivada                                                                         | Pendiente de demostración manual                                 | Las pruebas de reapertura SQLite no sustituyen la ejecución de escritorio                                               |

## Verificaciones ejecutadas

| Comprobación                                                                    | Resultado                                                                     |
| ------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| Tauri: `cargo check`, `cargo test`, `cargo clippy --all-targets -- -D warnings` | Aprobados: 51 pruebas unitarias y 1 doctest, sin pruebas ignoradas            |
| Servidor: `cargo check`, `cargo test`                                           | Aprobados: 1 prueba del ping                                                  |
| Frontend: `npm test`, `npm run lint`, `npm run format:check`, `npm run build`   | Aprobados: 4 pruebas, TypeScript y exportación estática incluidos en el build |
| `npm run tauri -- build --debug --no-bundle`                                    | Aprobado: ejecutable debug generado; no se abrió la ventana                   |
| `git diff --check`                                                              | Aprobado                                                                      |

Las 51 pruebas incluyen las 38 anteriores, siete adicionales de la rama
integrada y seis regresiones de `pruebas_patrimonio.rs`. No se quitaron pruebas
para conseguir un resultado positivo. Durante la reconciliación hubo errores
de compilación de fixtures adaptados al nuevo DTO; se corrigieron y se volvió
a ejecutar la suite completa.

La [guía de pruebas](pruebas/README.md#entrega-4-monedas-y-tasas) contiene
comandos, resultados esperados y la demostración de escritorio. El escenario
316100 VES → falta USDT/USD → 350 USD se ejecuta también con reapertura SQLite.

## Pendientes y límites

No quedan defectos conocidos de implementación de backend o SQLite en el
alcance comprobado. La base personal no se abrió ni se modificó. Las pruebas
usan memoria o archivos temporales nuevos.

El frontend mantiene dos pendientes identificados antes de integrar:

1. Presentar importes conforme a los decimales del catálogo: el formateador
   todavía fuerza un mínimo de dos, incluso en monedas de cero/un decimal.
2. Invalidar u ocultar el patrimonio anterior durante nuevas consultas y al
   fallar un cambio de fecha/base, para no presentar resultados obsoletos.

También falta demostrar el flujo de formularios, cierre/reapertura de ventana
y autonomía real con red desactivada y Axum apagado. No se declara esa prueba
manual aprobada por haber pasado la suite automática.

## Publicación del PR existente

La integración y las correcciones son locales. No se hizo push ni se aprobó
ni fusionó el PR. Antes de publicar, actualizar referencias y comprobar que
no haya cambios remotos nuevos que requieran otra integración:

```bash
git switch e4-backend-keyber
git fetch origin
git merge-base --is-ancestor origin/dev HEAD &&
git merge-base --is-ancestor origin/e4-backend-keyber HEAD &&
git push origin e4-backend-keyber
```

El push normal actualiza el PR existente hacia `dev`; no se necesita force push.
Si alguna comprobación de ascendencia falla, detener la publicación y revisar
los commits nuevos. Después del push se podrán revisar los checks y aprobar
el PR por separado.
