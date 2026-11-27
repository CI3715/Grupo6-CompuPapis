# ADR 015 - Proyección y sincronización del libro personal

## Estado

Aceptada para la Entrega 8.

## Contexto

Las instalaciones deben fusionar eventos inmutables y obtener los mismos saldos sin compartir el libro con el servidor de Cuentas Claras. La recuperación de un libro incompatible debe conservar sus datos para revisión.

## Decisión

- La proyección ordena los eventos por momento de registro, dispositivo de origen e identificador. Los eventos repetidos con contenido diferente se rechazan y se informa el motivo.
- Los eventos recién creados localmente reciben un momento posterior al último evento existente si el reloj retrocede o empata. En SQLite, la lectura y la inserción ocurren en una transacción. La importación conserva los metadatos originales.
- La importación valida el archivo antes de fusionar y guarda los eventos aceptados en una transacción. Los eventos locales anteriores no se reescriben ni se borran.
- El puerto `TransporteSincronizacion` admite Git, carpeta compartida y un adaptador en memoria. Los lotes son cifrados, inmutables y se escriben en la carpeta del dispositivo; la sincronización trae, fusiona, envía y publica de forma incremental.
- Si la proyección falla, el diagnóstico identifica el primer evento cuyo prefijo no puede proyectarse. Se puede exportar una copia de los eventos legibles para revisarlos sin modificar el libro.

## Consecuencias

Un libro histórico con dependencias anteriores a sus eventos de origen puede requerir revisión: el diagnóstico no modifica ni repara eventos. Una copia conserva los eventos legibles, pero un archivo con contenido incompatible no necesariamente puede importarse. Una marca temporal futura en el libro adelanta las marcas de los eventos locales posteriores para conservar el orden causal.

Las pruebas automatizadas cubren importación repetida y concurrente, rollback, restauración, cifrado y contratos de transporte. El procedimiento de comprobación con dos instalaciones está en la [guía de sincronización](../guia-sincronizacion-personal.md).
