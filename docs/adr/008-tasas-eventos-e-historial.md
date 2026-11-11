# ADR 008 - Tasas de cambio mediante eventos y consulta activa

## Estado

Implementada en la Entrega 4; documentada retrospectivamente según el código actual.

## Contexto

Las tasas deben conservarse en el libro personal, permitir consultas por fecha y mantener el historial de cambios sin modificar eventos ya registrados.

## Decisión

- Registrar tasas y anulaciones como eventos `TasaRegistrada` y `TasaEliminada` en el libro SQLite local. La anulación no borra el evento original.
- Devolver tasas activas en el listado y el historial de la aplicación. Una tasa anulada queda excluida también de consultas con fecha anterior; la consulta no reconstruye qué se sabía en una fecha pasada.
- Ordenar por fecha civil descendente y, ante empate, priorizar el evento insertado más recientemente. Rechazar la anulación de una tasa inexistente o ya anulada; la repetición del mismo evento conserva la idempotencia del repositorio.
- Para una consulta, elegir una tasa activa con fecha igual o anterior: primero la tasa directa más reciente y, si no existe, la inversa. No calcular rutas por monedas intermedias.

## Alternativas consideradas

- Borrar o modificar físicamente la tasa: perdería la secuencia de eventos.
- Reconstruir el conocimiento histórico incluyendo anulaciones posteriores o buscar conversiones cruzadas: no es el contrato implementado.

## Consecuencias

Las tasas se conservan localmente y las consultas tienen reglas reproducibles. El historial visible no es una auditoría de todos los eventos; la anulación puede cambiar una consulta pasada. Ver [Entrega 4: backend local](../entrega-4-backend.md#decisiones-explícitas-de-aplicación), `app/src-tauri/src/domain.rs` y `app/src-tauri/src/tasas_monedas.rs`.
