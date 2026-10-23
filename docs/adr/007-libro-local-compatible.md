# ADR 007 - Lectura compatible del libro local y decisiones monetarias exactas

## Estado

Aceptada durante la integración de las Entregas 2 y 3.

## Contexto

`main` y `dev` guardaron payloads distintos para movimientos y correcciones. Sustituir un lector por el otro podía omitir eventos y reconstruir saldos incorrectos. El libro existente es inmutable. Las advertencias calculadas mediante `Number` en React también podían perder centavos.

## Decisión

- Mantener SQLite local y su tabla de eventos, sin actualizar ni eliminar eventos anteriores.
- Leer ambos formatos y normalizarlos únicamente en memoria. Los eventos nuevos conservan el formato de la implementación de Entrega 3: `Monto`, tipo y fecha de movimiento; los ajustes incluyen monto correcto y diferencia firmada.
- Interpretar el formato histórico `tipo_movimiento` + `Decimal` con la moneda de su cuenta. Los ajustes históricos `monto_ajuste` son efectos firmados sobre el saldo, y se aplican en orden de inserción para reconstruir el valor vigente.
- Como los movimientos históricos no contienen fecha civil, usar la fecha de registro interpretada en la zona local del dispositivo como fecha de presentación. Se conserva el timestamp original.
- Detener la proyección con un error explícito ante payloads, referencias o diferencias inconsistentes; no mostrar un saldo parcial.
- Validar fechas con el reloj civil local de Rust; la UI obtiene su fecha predeterminada mediante un comando Tauri.
- Calcular la evaluación de saldo negativo con `rust_decimal` en Rust. React presenta decimales canónicos mediante operaciones sobre texto.
- Ordenar el historial por fecha civil descendente, con orden de registro descendente como desempate estable.

## Alternativas consideradas

- Reescribir los eventos antiguos mediante una migración: descartada porque cambia el libro original y aumenta el riesgo de pérdida.
- Ignorar eventos que no deserializan: descartada porque produce saldos parciales.
- Cálculos financieros con `Number`: descartada por pérdida de precisión.

El identificador predeterminado `com.tauri.dev` impide generar el ejecutable con Tauri. Se sustituye por `com.compupapis.cuentasclaras`. En escritorio se adopta la base anterior mediante SQLite Backup (incluyendo WAL), validación de integridad y proyección del libro, y publicación atómica sin reemplazo en el nuevo directorio. El original permanece intacto. La publicación requiere un sistema de archivos local con soporte de enlaces físicos en el directorio de datos; si falla, la migración se detiene conservando el origen. Una base nueva existente tiene prioridad y nunca se sobrescribe.

## Consecuencias

Las bases existentes se pueden abrir sin migración destructiva. Se prueban ambos formatos, las correcciones entre formatos y la reapertura de SQLite. Un libro inconsistente requiere diagnóstico conservando el archivo; la aplicación no intenta repararlo silenciosamente. Cambiar la zona horaria del dispositivo puede cambiar la fecha civil inferida de eventos históricos sin fecha propia.

El comando de red `ping_servidor` permanece separado del libro: solo realiza `GET /ping` cuando el usuario lo solicita y no transporta datos financieros.
