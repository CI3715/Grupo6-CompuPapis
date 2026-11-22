# ADR 012 - Sincronización de gastos grupales al libro local

## Estado

Aceptada para la entrega 7 e implementada en el código actual. La comprobación completa desde la ventana Tauri y la cobertura mínima de la capa de aplicación siguen pendientes.

## Contexto

Cada participante debe poder reflejar su parte de un gasto compartido en su libro sin duplicarla al repetir la sincronización. El servidor no debe conocer las cuentas, movimientos, saldos ni tasas personales (RF-COM-10, INV-13 a INV-18).

## Decisión

- PostgreSQL registra un evento de cambio por participante de la división. `GET /cambios?desde=<cursor>` requiere sesión y devuelve los eventos del usuario autenticado en orden, con cursor y `hay_mas`, en lotes de hasta 100. La selección de destinatarios excluye a quienes no participan.
- Tauri consume la API mediante el puerto `ClienteSocial` y su adaptador HTTP. El motor envía primero la cola local y después descarga páginas desde el cursor persistido. El frontend solicita la sincronización al iniciar una sesión, después de registrar un gasto, al reconectar, manualmente y cada cinco minutos. Una respuesta `401` detiene el proceso con el aviso de sesión vencida.
- SQLite conserva la elección de cuenta y la moneda del grupo obtenida del servidor. El comando comprueba que la cuenta exista y tenga la misma moneda; si aún no se conoce la moneda, exige actualizar los datos del grupo. El dominio actual no modela cuentas cerradas, por lo que no se comprueba un estado de cierre independiente.
- La interfaz guarda por usuario una copia local de la lista y los detalles de grupos consultados. Si la red falla durante una sesión, permite abrir un grupo cuyo detalle ya se haya guardado y registrar un gasto en cola. Para consultar un grupo por primera vez sigue siendo necesaria la conexión. La sesión permanece en memoria y no sobrevive al cierre de la aplicación.
- Las asociaciones, partes pendientes, cola y cursor se guardan en tablas separadas por identificador del usuario autenticado. El libro personal sigue siendo el libro local del dispositivo. Los datos anteriores de sincronización que no identifican a su dueño se conservan sin asignarlos automáticamente; la interfaz permite vincularlos a la sesión actual tras una confirmación explícita. La migración es transaccional y rechaza sobrescribir un destino que ya contiene datos.
- Una parte sin cuenta elegida queda en `pendientes_aplicar`. Elegir una cuenta aplica las partes pendientes en la misma transacción que guarda la asociación. Cambiarla afecta a las partes posteriores, sin trasladar movimientos anteriores.
- El evento local `GastoDeGrupoAplicado` conserva el identificador de origen, la cuenta, el gasto, la fecha, el monto y el concepto compuesto. La proyección lo muestra como gasto de grupo y el núcleo rechaza su corrección. Se comprueba el identificador del servidor antes de aplicar, incluso para duplicados dentro de un lote.
- Los eventos locales nuevos, los pendientes y el cursor de cada lote se escriben en una transacción SQLite. Los eventos locales anteriores no se borran ni reescriben. Los tipos de cambio desconocidos se omiten; un gasto conocido inválido produce error y revierte el lote. La fecha de última sincronización exitosa se registra solo al completar cola y páginas.

## Alternativas consideradas

- Sincronizar el libro personal completo con el servidor: descartado por privacidad y porque excede el alcance de la entrega.
- Aplicar cambios sin persistir el identificador del servidor: descartado porque repetir una página duplicaría movimientos.
- Avanzar el cursor por separado de los movimientos: descartado porque una interrupción podría perder partes.

## Consecuencias

La aplicación mantiene el libro disponible sin conexión y puede reprocesar páginas sin duplicar gastos. Los datos de cuenta quedan en el dispositivo; la sincronización no sube el libro. El cliente debe conservar el estado SQLite y una sesión válida para comunicarse con el servidor. Las pruebas existentes cubren transacciones, reintentos, cursor, pendientes y correcciones con SQLite y un cliente falso; la prueba integrada en Tauri requiere ejecución manual.

El código conserva además una implementación local alternativa de persistencia de la entrega 7 con tablas propias. Los comandos de la interfaz usan el repositorio y las tablas `cuentas_grupos`, `pendientes_aplicar` y `estado_sincronizacion`; la consolidación de ambas implementaciones no se da por realizada en esta decisión.
