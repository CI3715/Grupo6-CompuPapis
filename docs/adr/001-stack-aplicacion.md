# ADR 001 - Stack de la aplicación

## Estado

Aceptada.

## Contexto

Cuentas Claras necesita una aplicación de escritorio con una interfaz web estática y un núcleo local encargado de la lógica y el acceso a recursos del sistema.

## Decisión

Se utilizará Next.js con TypeScript para la interfaz, Tauri 2 para empaquetar la aplicación y Rust para el núcleo local.

Next.js utilizará `output: "export"` y la comunicación entre la interfaz y Rust se realizará mediante comandos de Tauri con `invoke()`.

## Alternativas consideradas

- React con Vite.
- Tauri con TypeScript como núcleo.
- Electron.

## Consecuencias

La aplicación puede compartir una interfaz basada en tecnologías web manteniendo la lógica local en Rust.

La interfaz no puede utilizar API Routes, Server Actions ni depender de un servidor de Next.js en producción.
