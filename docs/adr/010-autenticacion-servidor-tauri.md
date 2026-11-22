# ADR 010 - Identidad en el servidor y sesión en Tauri

## Estado

Implementada en el código actual de la Entrega 5; documentada retrospectivamente. La integración desde las pantallas y su comprobación de extremo a extremo permanecen pendientes.

## Contexto

La aplicación necesita registro, inicio de sesión y consulta de perfil sin trasladar el libro financiero personal al servidor. El equipo de frontend necesita un contrato para llamar estas operaciones desde Tauri.

## Decisión

- El servidor Rust/Axum usa PostgreSQL para usuarios de autenticación, con nombre de usuario único e índice único de correo normalizado. Las contraseñas se almacenan con hash Argon2 y sal; no se almacenan en texto plano.
- Exponer `POST /usuarios`, `POST /sesiones` y `GET /usuarios/yo`. El servidor firma JWT con `JWT_SECRET`, valida el bearer token y establece un vencimiento de 60 minutos. La clave se configura en el entorno del servidor y requiere al menos 32 bytes.
- Tauri ofrece `registrar_usuario`, `iniciar_sesion`, `obtener_perfil` y `cerrar_sesion`. El frontend dispone de funciones tipadas en `app/lib/autenticacion.ts`. Las llamadas usan la API HTTP del servidor; el frontend no accede directamente a PostgreSQL.
- Guardar el token solo en memoria del proceso Tauri. Cerrar sesión lo descarta localmente y cerrar la aplicación obliga a iniciar sesión de nuevo. El cierre local no revoca el JWT en el servidor.
- Usar `http://127.0.0.1:8000` en el adaptador actual para desarrollo de escritorio. Un despliegue o dispositivo remoto requiere configuración de URL y HTTPS apropiada; no está implementado aquí.

## Alternativas consideradas

- Conectar el frontend directamente a PostgreSQL: expondría credenciales y eludiría las reglas de la API.
- Persistir la sesión en el cliente: aún no existe un mecanismo de almacenamiento seguro definido para esta implementación.
- Enviar el libro financiero al servidor: contradice el modelo local del producto.

## Consecuencias

El servidor debe estar disponible para autenticar y consultar el perfil, mientras el libro personal conserva su funcionamiento local. Los comandos, adaptadores y pantallas están implementados; la validación completa desde la ventana Tauri requiere comprobación manual. La sesión en memoria no sobrevive a un reinicio. Ver [OpenAPI](../../openapi.yaml), `server/src/auth.rs` y `app/src-tauri/src/autenticacion.rs`.
