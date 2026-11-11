# ADR 005 - Arquitectura hexagonal

## Estado

Aceptada como dirección arquitectónica. La separación por capas debe seguir verificándose en cada módulo; este ADR no certifica conformidad completa.

## Contexto

La guía técnica del proyecto exige separar las reglas de negocio de detalles externos como bases de datos, red e interfaz.

## Decisión

El sistema seguirá una arquitectura hexagonal basada en dominio, aplicación, puertos, adaptadores e infraestructura.

## Alternativas consideradas

- Arquitectura organizada únicamente por tecnología.
- Acceso directo desde la lógica de negocio a infraestructura.

## Consecuencias

Las dependencias deben apuntar hacia las capas internas.

La infraestructura implementará puertos definidos por la aplicación y no será conocida por el dominio.

## Estado de implementación

El libro local separa las reglas y el repositorio SQLite en módulos Rust, y la interfaz llama comandos Tauri. El servidor actual concentra rutas, validaciones y acceso SQL de autenticación en `server/src/auth.rs`; la separación hexagonal completa de ese flujo no está demostrada. Mantener esta distinción permite documentar la decisión sin presentar una refactorización pendiente como terminada.
