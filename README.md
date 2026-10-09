# Cuentas Claras - CompuPapis

![Made with Love](https://img.shields.io/badge/Made%20with-Love-pink?style=for-the-badge&logo=data:image/svg%2bxml;base64,PHN2ZyByb2xlPSJpbWciIHZpZXdCb3g9IjAgMCAyNCAyNCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48dGl0bGU+R2l0SHViIFNwb25zb3JzIGljb248L3RpdGxlPjxwYXRoIGQ9Ik0xNy42MjUgMS40OTljLTIuMzIgMC00LjM1NCAxLjIwMy01LjYyNSAzLjAzLTEuMjcxLTEuODI3LTMuMzA1LTMuMDMtNS42MjUtMy4wM0MzLjEyOSAxLjQ5OSAwIDQuMjUzIDAgOC4yNDljMCA0LjI3NSAzLjA2OCA3Ljg0NyA1LjgyOCAxMC4yMjdhMzMuMTQgMzMuMTQgMCAwIDAgNS42MTYgMy44NzZsLjAyOC4wMTcuMDA4LjAwMy0uMDAxLjAwM2MuMTYzLjA4NS4zNDIuMTI2LjUyMS4xMjUuMTc5LjAwMS4zNTgtLjA0MS41MjEtLjEyNWwtLjAwMS0uMDAzLjAwOC0uMDAzLjAyOC0uMDE3YTMzLjE0IDMzLjE0IDAgMCAwIDUuNjE2LTMuODc2QzIwLjkzMiAxNi4wOTYgMjQgMTIuNTI0IDI0IDguMjQ5YzAtMy45OTYtMy4xMjktNi43NS02LjM3NS02Ljc1em0tLjkxOSAxNS4yNzVhMzAuNzY2IDMwLjc2NiAwIDAgMS00LjcwMyAzLjMxNmwtLjAwNC0uMDAyLS4wMDQuMDAyYTMwLjk1NSAzMC45NTUgMCAwIDEtNC43MDMtMy4xNjNjLTIuNjc3LTIuMzA3LTUuMDQ3LTUuMjk4LTUuMDQ3LTguNTIzIDAtMi43NTQgMi4xMjEtNC41IDQuMTI1LTQuNSAyLjA2IDAgMy45MTQgMS40NzkgNC41NDQgMy42ODQuMTQzLjQ5NS41OTYuNzk3IDEuMDg2Ljc5Ni40OS4wMDEuOTQzLS4zMDIgMS4wODUtLjc5Ni42My0yLjIwNSAyLjQ4NC0zLjY4NCA0LjU0NC0zLjY4NCAyLjAwNCAwIDQuMTI1IDEuNzQ2IDQuMTI1IDQuNSAwIDMuMjI1LTIuMzcgNi4yMTYtNS4wNDggOC41MjN6Ii8+PC9zdmc+)

Proyecto del laboratorio de Ingeniería de Software I (CI-3715) de la Universidad Simón Bolívar.

Cuentas Claras es una aplicación de finanzas personales multimoneda, orientada al manejo local de cuentas y movimientos, así como al registro de gastos compartidos entre usuarios.

## Equipo

**Nombre del equipo:** `CompuPapis`

## Stack

### Aplicación

- Next.js
- TypeScript
- Tauri 2
- Rust
- SQLite

### Servidor

- Rust
- Axum
- PostgreSQL
- Neon

## Entrega 1

La primera entrega consiste en configurar la estructura base del proyecto y verificar la comunicación entre la aplicación y el servidor.

El flujo inicial será:

`Next.js → Tauri/Rust → Axum → Tauri/Rust → Next.js`

El servidor expone el endpoint:

`GET /ping`

La aplicación debe utilizarlo para verificar la conexión y mostrar el estado, mensaje, versión y tiempo de respuesta del servidor.

## Instalación y ejecución

### Requisitos

Para ejecutar el proyecto es necesario tener instalado:

- Git
- Node.js
- npm
- Rust
- Cargo
- Dependencias del sistema requeridas por Tauri 2

Clonar el repositorio:

```bash
git clone https://github.com/CI3715/Grupo6-CompuPapis.git cuentas-claras
cd cuentas-claras
```

### Instalar las dependencias del frontend

```bash
cd app
npm install
```

## Ejecutar el proyecto

Para utilizar la aplicación completa es necesario ejecutar el servidor y la aplicación de escritorio en terminales separadas.

### Backend

En una terminal:

```bash
cd server
cargo run
```

El servidor estará disponible en:

```text
http://127.0.0.1:8000
```

El endpoint de prueba puede verificarse con:

```bash
curl http://127.0.0.1:8000/ping
```

### Aplicación de escritorio

En otra terminal:

```bash
cd app
npm run tauri dev
```

Este es el modo recomendado para desarrollar y ejecutar Cuentas Claras.

Tauri inicia el servidor de desarrollo de Next.js y abre la aplicación como una ventana de escritorio. La comunicación con el backend se realiza mediante comandos de Tauri:

```text
Next.js → Tauri/Rust → Axum → Tauri/Rust → Next.js
```

### Ejecutar únicamente Next.js

También es posible iniciar únicamente la interfaz web:

```bash
cd app
npm run dev
```

La interfaz estará disponible en:

```text
http://localhost:3000
```

Este modo sirve para trabajar únicamente en la parte visual del frontend. Las funcionalidades que utilizan comandos de Tauri, como la comunicación con el backend mediante `invoke()`, requieren ejecutar la aplicación con:

```bash
npm run tauri dev
```

### Generar el frontend estático

```bash
cd app
npm run build
```

El resultado se genera en:

```text
app/out/
```

## Formato y lint

Desde `app`:

```bash
npm run format
```

formatea tanto el frontend como los proyectos de Rust.

Para verificar el formato sin modificar archivos:

```bash
npm run format:check
```

Para ejecutar ESLint:

```bash
npm run lint
```

También se pueden formatear las partes por separado:

```bash
npm run format:front
npm run format:tauri
npm run format:back
npm run format:rust
```

## Estudiantes

- Maikel Delgado — `16-10287`
- Gabriel Orejarena — `18-10292` [@Arnold-Wesker](https://github.com/Arnold-Wesker)
- Keyber Sequera — `16-11120` [@keybersequera8](https://github.com/keybersequera8)
- Gabriel De Ornelas — `15-10377` [@gabodornelas](https://github.com/gabodornelas)
- Elías El Jaovich — `18-10641` [@ElJaovich](https://github.com/ElJaovich)
- Luis Isea — `19-10175` [@lmisea](https://github.com/lmisea)

## Licencia

Este proyecto está distribuido bajo la licencia [MIT](./LICENSE).
