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

Para crear y ver las cuentas solo necesitamos abrir la app, el servidor se usa aparte para probar el /ping de la Entrega 1.

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

## Entrega 2 - BD y Pruebas

### BD local

Para guardar las cuentas usamos el archivo `cuentas_claras.sqlite` en `app_data_dir` de Tauri, la ubicación cambia segun el sistema y si el archivo no existe, se crea cuando abrimos la app.

### Pruebas

Para hacer las pruebas de Rust, entramos en la carpeta de Tauri:

```bash
cd app/src-tauri
cargo test
```

Con estas pruebas revisamos que los datos incorrectos den error, que los subtotales salgan bien, que no se repitan las cuentas y que no se puedan cambiar ni borrar los eventos.

## Entrega 3 - Gastos e ingresos

En esta entrega hicimos el registro de ingresos y gastos, las categorías, la corrección de movimientos.

También hicimos unas pruebas para revisar que todo funcione correctamente.

### Pruebas

Para ejecutar las pruebas entramos en `app/src-tauri` y usamos:

```bash
cargo test
```

Si solo queremos ejecutar las pruebas de movimientos, usamos:

```bash
cargo test pruebas_movimientos
```

Aqui probamos las categorías y los movimientos, que los datos incorrectos den error, que los saldos por moneda esten bien y que no se repitan eventos.

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

## Demostración de las Entregas 2 y 3

Las cuentas, categorías, movimientos, ajustes y saldos son locales. No requieren registro, internet ni el servidor Axum. El botón **Conexión con el servidor · Entrega 1 → Probar conexión** invoca `ping_servidor` explícitamente; es independiente del libro financiero.

### Verificaciones automatizadas

Usar Node.js 22.18 o posterior (las pruebas del formateador ejecutan TypeScript directamente), Rust/Cargo y las dependencias del sistema de Tauri 2. Después de instalar las dependencias, desde la raíz:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml
cargo test --manifest-path app/src-tauri/Cargo.toml probar_crear_cuentas
cargo test --manifest-path app/src-tauri/Cargo.toml pruebas_integracion
cargo test --manifest-path app/src-tauri/Cargo.toml test_ejemplo_seccion_2_2
cargo check --manifest-path app/src-tauri/Cargo.toml
cargo fmt --manifest-path app/src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path app/src-tauri/Cargo.toml -- -D warnings
cargo check --manifest-path server/Cargo.toml
cargo test --manifest-path server/Cargo.toml
cargo fmt --manifest-path server/Cargo.toml -- --check
cargo clippy --manifest-path server/Cargo.toml -- -D warnings
```

Desde `app`:

```bash
npm test
npm run lint
npm run format:check
npm run build
npm run tauri build
```

El build usa el soporte nativo de CSS de Next.js y la API del comprobador TypeScript 5. No descarga Google Fonts: se conserva la tipografía de sistema definida en `globals.css`. No se omiten los errores de tipos. `npm run build` genera el frontend; `npm run tauri build` genera el ejecutable y los instaladores con ese frontend.

Las pruebas del dominio utilizan el repositorio en memoria, sin red ni base de datos. Las pruebas de persistencia usan archivos SQLite temporales y comprueban la reapertura, los payloads históricos y actuales, los ajustes y que los eventos originales no cambien. Los archivos temporales de prueba no reemplazan la base del usuario.

### Ubicación de SQLite

Archivo: `cuentas_claras.sqlite` dentro del `app_data_dir` de Tauri, con identificador actual `com.compupapis.cuentasclaras`. La aplicación imprime la ruta efectiva al iniciar.

Ubicaciones predeterminadas orientativas:

- Linux: `$XDG_DATA_HOME/com.compupapis.cuentasclaras/cuentas_claras.sqlite`, o `~/.local/share/com.compupapis.cuentasclaras/cuentas_claras.sqlite`.
- macOS: `~/Library/Application Support/com.compupapis.cuentasclaras/cuentas_claras.sqlite`.
- Windows: `%APPDATA%\com.compupapis.cuentasclaras\cuentas_claras.sqlite`.

Al actualizar desde el identificador anterior `com.tauri.dev`, la aplicación copia el libro mediante SQLite Backup si todavía no existe la nueva base. La copia incluye WAL, se verifica antes de adoptarla y nunca reemplaza un destino existente. El archivo anterior y sus eventos se conservan intactos. Una copia incompatible no se publica.

No borrar la base para actualizar las ramas. Los dos formatos históricos del libro se leen sin reescribir eventos. Si aparece un error de integridad, conservar el archivo y revisar sus eventos; no se presenta un saldo parcial. Ver [ADR 007](docs/adr/007-libro-local-compatible.md).

### Ejemplo obligatorio: saldo final 1750 USD

1. Ejecutar `cd app` y `npm run tauri dev`. La aplicación abre en **Mis cuentas**.
2. Crear **Billetera dólares**, tipo **billetera digital**, moneda **USD**, saldo inicial **1000.00**.
3. Abrir esa cuenta. Registrar ingreso **800.00**, categoría **Salario**, fecha de hoy o pasada. Saldo: **1800.00**.
4. Registrar gasto **500.00**, categoría **Alimentación**. Saldo: **1300.00**.
5. En el gasto original, elegir **Corregir** e indicar **50.00**. Se agrega un ajuste de **+450.00** y el saldo queda en **1750.00**.
6. Comprobar la composición: inicial **1000.00**, ingresos **800.00**, gastos **500.00**, ajustes **450.00**. El gasto original **−500.00** sigue visible y marcado como corregido; el ajuste referencia su identificador.
7. Cerrar y reabrir la aplicación. El historial y saldo deben mantenerse.

Las correcciones posteriores se calculan desde el monto vigente del original. Corregir a cero lo compensa completamente. No se pueden editar/eliminar movimientos ni corregir directamente ajustes.

### Prueba manual offline

1. Crear también efectivo en VES y banco en USD; verificar subtotales separados y una billetera en USDT.
2. Registrar dos gastos y un ingreso en cuentas de monedas diferentes. Comprobar errores de monto cero/negativo, más de dos decimales, fecha futura y categoría incompatible. Crear una categoría y comprobar duplicados por tipo.
3. Intentar un gasto superior al saldo. Comprobar que aparece la advertencia antes de guardar; cancelar y verificar que el libro no cambió. Repetir confirmando y comprobar el saldo negativo.
4. Registrar después un movimiento con fecha anterior y comprobar el orden por fecha. En fechas iguales, el último registrado aparece primero.
5. Cerrar la aplicación y detener Axum. Desactivar la red y volver a abrir; comprobar saldos, ajustes, categorías y creación de nuevos registros.
6. Para demostrar autonomía completa, abrir el ejecutable generado por `npm run tauri build`: no necesita servidores de Next.js ni Axum. `tauri dev` mantiene un servidor de desarrollo local de Next.js y no sustituye esta comprobación del ejecutable.
7. Encender Axum con `cargo run --manifest-path server/Cargo.toml` desde la raíz. Verificar `curl http://127.0.0.1:8000/ping` y el botón de conexión. Apagarlo nuevamente y comprobar que las cuentas siguen funcionando.

La validación de fechas usa la fecha civil del dispositivo, también para el valor predeterminado del formulario. Las pruebas incluyen el cambio de día UTC cuando en Caracas todavía es el día anterior.

Para compilar un ejecutable de demostración sin generar instaladores:

```bash
cd app
npm run tauri -- build --debug --no-bundle
```

En Linux, el ejecutable queda en `app/src-tauri/target/debug/app` (ruta desde la raíz). Este build incluye el frontend estático; no requiere ejecutar Next.js ni Axum para usar el libro.
