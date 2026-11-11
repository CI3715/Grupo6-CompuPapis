# ADR 003 - PostgreSQL para el servidor y Neon como proveedor previsto

## Estado

Aceptada la elección de PostgreSQL y de Neon como proveedor administrado previsto. El entorno local con Docker está implementado; Neon y el despliegue administrado aún no se han configurado ni verificado.

## Contexto

El servidor necesita persistencia de identidad y, en etapas posteriores, de relaciones y datos compartidos. El desarrollo y la Entrega 5 necesitan además una base accesible localmente y otra independiente para pruebas.

## Decisión

- Usar PostgreSQL en el servidor Rust. `DATABASE_URL` configura la conexión; el servidor ejecuta `server/migrations/001_usuarios.sql` al iniciar y necesita permisos para crear la tabla y el índice.
- En desarrollo, `server/compose.yaml` arranca PostgreSQL 16 en `127.0.0.1:55432`, con volumen persistente y bases separadas para la aplicación y las pruebas de integración. Las credenciales de Compose son exclusivamente locales.
- Mantener Neon como proveedor administrado previsto para una etapa posterior. La conexión real, la persistencia administrada, el despliegue y TLS requieren configuración y validación posteriores.
- Mantener cuentas financieras, movimientos, saldos y tasas personales en SQLite local; la autenticación del servidor no envía esos datos.

## Alternativas consideradas

- Usar solo un proveedor remoto durante el desarrollo: requiere red y credenciales compartidas incluso para pruebas locales.
- MongoDB Atlas, Turso o Supabase PostgreSQL: no son la elección actual del proyecto.

## Consecuencias

El equipo puede desarrollar sin Neon y ejecutar la prueba de integración en una base separada. PostgreSQL local requiere Docker o una instalación compatible; un volumen persistente no equivale a una estrategia de respaldo. El paso a Neon requiere su URL real, gestión de credenciales y verificación del despliegue. Ver [README](../../README.md#servidor-opcional-conexión-y-autenticación) y [ADR 010](010-autenticacion-servidor-tauri.md).
