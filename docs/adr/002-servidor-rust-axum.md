# ADR 002 - Servidor en Rust con Axum

## Estado

Aceptada.

## Contexto

Cuentas Claras necesita un servidor HTTP independiente para las funcionalidades compartidas entre usuarios.

## Decisión

El servidor será desarrollado en Rust utilizando Axum.

## Alternativas consideradas

- TypeScript con Hono.
- Python con FastAPI.

## Consecuencias

La aplicación y el servidor utilizan el mismo lenguaje para su núcleo.

El equipo debe manejar el modelo asíncrono utilizado por Axum y Tokio.
