# ADR 005 - Arquitectura hexagonal

## Estado

Aceptada.

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
