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

En Linux también son necesarias las dependencias del sistema requeridas por Tauri 2.

Una vez clonado el repositorio:

```bash
git clone https://github.com/CI3715/Grupo6-CompuPapis.git
cd cuentas-claras
```

### Frontend

La aplicación se encuentra en la carpeta `app`.

Primero se deben instalar las dependencias:

```bash
cd app
npm install
```

#### Ejecutar con Next.js

Para ejecutar únicamente la interfaz web:

```bash
npm run dev
```

La aplicación estará disponible normalmente en:

```text
http://localhost:3000
```

#### Ejecutar como aplicación de escritorio con Tauri

Para ejecutar la aplicación completa utilizando Tauri:

```bash
npm run tauri dev
```

Tauri iniciará automáticamente el servidor de desarrollo de Next.js y abrirá la aplicación en una ventana de escritorio.

#### Generar el frontend estático

```bash
npm run build
```

El resultado se genera en:

```text
app/out/
```

### Backend

El servidor se encuentra en la carpeta `server` y está desarrollado en Rust utilizando Axum.

Desde la raíz del repositorio:

```bash
cd server
cargo run
```

El servidor se ejecutará en:

```text
http://127.0.0.1:8000
```

Para verificar que el servidor está funcionando:

```bash
curl http://127.0.0.1:8000/ping
```

La respuesta tendrá la siguiente estructura:

```json
{
  "estado": "ok",
  "mensaje": "Servidor Cuentas Claras activo",
  "version": "0.1.0",
  "timestamp": "<fecha y hora de la respuesta>"
}
```

El campo `timestamp` se genera automáticamente al momento de cada solicitud.

## Estudiantes

- Maikel Delgado — `16-10287`
- Gabriel Orejarena — `18-10292`
- Keyber Sequera — `16-11120`
- Gabriel De Ornelas — `15-10377`
- Elías El Jaovich — `18-10641`
- Luis Isea — `19-10175` [@lmisea](https://github.com/lmisea)

## Licencia

Este proyecto está distribuido bajo la licencia [MIT](./LICENSE).
