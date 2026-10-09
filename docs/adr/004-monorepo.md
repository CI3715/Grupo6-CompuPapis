# ADR 004 - Organización en monorepo

## Estado

Aceptada.

## Contexto

El proyecto contiene una aplicación cliente, un servidor y documentación que evolucionarán conjuntamente.

## Decisión

Se utilizará un único repositorio con las carpetas principales:

- `app/`
- `server/`
- `docs/`

## Alternativas consideradas

- Repositorios separados para aplicación y servidor.

## Consecuencias

La aplicación, el servidor y la documentación pueden versionarse y revisarse conjuntamente.

Las fronteras arquitectónicas deben mantenerse explícitas dentro del repositorio.
