# ADR 013 - Cola offline e idempotencia de gastos grupales

## Estado

Aceptada para la entrega 7 e implementada en el código actual. La verificación de extremo a extremo con Tauri sigue pendiente.

## Contexto

Una persona debe poder registrar gastos grupales sin conexión y reenviarlos al volver la red, incluso si la respuesta de un envío anterior se perdió. No debe enviarse información del libro personal (RNF-17, ESC-09, INV-16).

## Decisión

- Al registrar un gasto de grupo, Tauri genera una clave UUID y guarda en SQLite la operación y su clave antes del primer envío. Si ya hay operaciones pendientes, la nueva permanece en cola para preservar el orden.
- La carga enviada contiene los campos del gasto grupal de la entrega 6. La selección de cuenta, los movimientos, los saldos y las tasas personales no forman parte de ella.
- El motor reenvía los pendientes en orden mediante `POST /grupos/{id}/gastos`, reutilizando la misma clave en `Idempotency-Key`. Al recibir éxito elimina la operación; ante `401` detiene la sincronización; ante fallo de red, `429` o error del servidor la conserva para reintentar. Las demás respuestas de rechazo dejan la operación marcada con un motivo y permiten continuar con las siguientes.
- El servidor comprueba sesión y membresía, y relaciona la clave con el usuario, el grupo y una huella determinista del gasto. La reserva de clave, el gasto, sus partes y los cambios se procesan transaccionalmente. Un reintento equivalente devuelve el gasto existente; una clave reutilizada con otros datos produce `409`. El contrato del servidor también admite solicitudes sin clave, aunque el cliente de esta entrega genera una para cada gasto.

## Alternativas consideradas

- Enviar antes de guardar la operación: descartado porque un cierre o una respuesta perdida podría impedir reutilizar la clave.
- Crear una clave nueva en cada intento: descartado porque el servidor no podría reconocer el reintento.
- Encolar todo el libro para sincronizarlo: descartado por privacidad y alcance.

## Consecuencias

La entrega al servidor puede diferirse sin bloquear las funciones personales. La clave persistida permite recuperar un envío cuya respuesta se perdió; el usuario puede consultar y descartar rechazos. La disponibilidad de red y una sesión válida siguen siendo necesarias para confirmar el gasto y descargar las partes. Las pruebas del servidor cubren idempotencia, concurrencia y respuestas HTTP con PostgreSQL; las pruebas del motor usan SQLite y un cliente falso para orden, reintentos, rechazo y sesión vencida.
