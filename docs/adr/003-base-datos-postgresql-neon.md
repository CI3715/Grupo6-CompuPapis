# ADR 003 - PostgreSQL con Neon

## Estado

Aceptada.

## Contexto

El servidor necesita persistencia para identidad, relaciones sociales, solicitudes y datos compartidos.

## Decisión

Se utilizará PostgreSQL como base de datos del servidor y Neon como proveedor administrado.

## Alternativas consideradas

- MongoDB Atlas.
- Turso.
- Supabase PostgreSQL.

## Consecuencias

El servidor dispone de una base de datos relacional administrada independientemente del despliegue del backend.

Los datos financieros personales continuarán almacenándose únicamente de forma local y no se enviarán a esta base de datos.
