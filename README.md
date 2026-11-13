# Cuentas Claras - CompuPapis

![Made with Love](https://img.shields.io/badge/Made%20with-Love-pink?style=for-the-badge&logo=data:image/svg%2bxml;base64,PHN2ZyByb2xlPSJpbWciIHZpZXdCb3g9IjAgMCAyNCAyNCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48dGl0bGU+R2l0SHViIFNwb25zb3JzIGljb248L3RpdGxlPjxwYXRoIGQ9Ik0xNy42MjUgMS40OTljLTIuMzIgMC00LjM1NCAxLjIwMy01LjYyNSAzLjAzLTEuMjcxLTEuODI3LTMuMzA1LTMuMDMtNS42MjUtMy4wM0MzLjEyOSAxLjQ5OSAwIDQuMjUzIDAgOC4yNDljMCA0LjI3NSAzLjA2OCA3Ljg0NyA1LjgyOCAxMC4yMjdhMzMuMTQgMzMuMTQgMCAwIDAgNS42MTYgMy44NzZsLjAyOC4wMTcuMDA4LjAwMy0uMDAxLjAwM2MuMTYzLjA4NS4zNDIuMTI2LjUyMS4xMjUuMTc5LjAwMS4zNTgtLjA0MS41MjEtLjEyNWwtLjAwMS0uMDAzLjAwOC0uMDAzLjAyOC0uMDE3YTMzLjE0IDMzLjE0IDAgMCAwIDUuNjE2LTMuODc2QzIwLjkzMiAxNi4wOTYgMjQgMTIuNTI0IDI0IDguMjQ5YzAtMy45OTYtMy4xMjktNi43NS02LjM3NS02Ljc1em0tLjkxOSAxNS4yNzVhMzAuNzY2IDMwLjc2NiAwIDAgMS00LjcwMyAzLjMxNmwtLjAwNC0uMDAyLS4wMDQuMDAyYTMwLjk1NSAzMC45NTUgMCAwIDEtNC43MDMtMy4xNjNjLTIuNjc3LTIuMzA3LTUuMDQ3LTUuMjk4LTUuMDQ3LTguNTIzIDAtMi43NTQgMi4xMjEtNC41IDQuMTI1LTQuNSAyLjA2IDAgMy45MTQgMS40NzkgNC41NDQgMy42ODQuMTQzLjQ5NS41OTYuNzk3IDEuMDg2Ljc5Ni40OS4wMDEuOTQzLS4zMDIgMS4wODUtLjc5Ni42My0yLjIwNSAyLjQ4NC0zLjY4NCA0LjU0NC0zLjY4NCAyLjAwNCAwIDQuMTI1IDEuNzQ2IDQuMTI1IDQuNSAwIDMuMjI1LTIuMzcgNi4yMTYtNS4wNDggOC41MjN6Ii8+PC9zdmc+)

Proyecto del laboratorio de Ingeniería de Software I (CI-3715) de la Universidad Simón Bolívar, desarrollado por **CompuPapis**.

Cuentas Claras permite crear cuentas personales multimoneda, registrar ingresos, gastos y correcciones, gestionar tasas de cambio y consultar el patrimonio consolidado en el dispositivo, sin registro de usuario ni conexión a internet. Las funcionalidades sociales y de gastos compartidos corresponden a etapas futuras.

## Arquitectura y entregas

- **Aplicación:** Next.js + TypeScript, Tauri 2 y Rust. SQLite mediante `rusqlite` almacena el libro local; `rust_decimal` realiza los cálculos monetarios exactos.
- **Entrega 1:** servidor Rust + Axum con `GET /ping`, consultado mediante `Next.js → invoke('ping_servidor') → Tauri/Rust → Axum` desde el botón de conexión.
- **Entrega 2:** inicio en Mis cuentas sin login; creación y validación de efectivo, banco y billetera digital, catálogo VES/USD/EUR/USDT, subtotales por moneda y persistencia local.
- **Entrega 3:** ingresos/gastos, categorías, correcciones mediante ajustes inmutables, composición del saldo e historial. El ejemplo obligatorio produce **1750 USD** conservando el gasto original.
- **Entrega 4 (monedas, tasas y patrimonio):** monedas propias, registro, anulación e historial activo de tasas por par, validaciones de fecha/fuente/valor y moneda base predeterminada USD. La consulta de patrimonio usa saldos y tasas a la fecha elegida, aplica tasas directas o inversas sin cruces y redondea el total una sola vez. Si falta una tasa, muestra los subtotales y el par pendiente sin devolver un total consolidado. Los eventos se conservan en SQLite local.
- **Servidor:** PostgreSQL + Neon para funcionalidades sociales. Las cuentas, movimientos y saldos personales permanecen en SQLite local y no se envían al servidor.

El flujo financiero es `Next.js → invoke() → Tauri/Rust → dominio → SQLite`. Axum no participa en él. Las decisiones arquitectónicas están en [docs/adr](docs/adr/README.md).

## Instalación

Se requieren Git, Node.js **22.18 o posterior**, npm, Rust/Cargo y las dependencias del sistema requeridas por Tauri 2.

```bash
git clone https://github.com/CI3715/Grupo6-CompuPapis.git cuentas-claras
cd cuentas-claras
```

Desde la raíz del repositorio, instalar las dependencias del frontend:

```bash
cd app
npm ci
```

En las instrucciones siguientes, **raíz** es la carpeta que contiene `app/`, `server/` y este README. Los bloques son independientes y especifican su directorio de partida; desde `app/` se vuelve a la raíz con `cd ..`.

## Ejecución

### Desarrollo de escritorio

Partiendo de la raíz:

```bash
cd app
npm run tauri dev
```

Este modo inicia Next.js como servidor de desarrollo local y abre la ventana Tauri. No requiere iniciar Axum para usar el libro financiero, pero necesita mantener el servidor de desarrollo de Next.js.

### Ejecutable independiente y prueba offline

Partiendo de la raíz, compilar con el frontend estático incluido:

```bash
cd app
npm run tauri -- build --debug --no-bundle
```

En Linux, desde esa misma carpeta `app/`:

```bash
./src-tauri/target/debug/app
```

En otros sistemas, utilizar la ruta que imprime Tauri al terminar. El ejecutable permite usar el libro sin Next.js, sin Axum y sin internet. Es el modo indicado para demostrar autonomía: `tauri dev` no sustituye esa comprobación. La primera compilación puede necesitar internet para descargar dependencias.

Para generar un build release e instaladores, ejecutar `npm run tauri build` desde `app/`. La generación de instaladores se verifica por separado del ejecutable debug.

### Servidor opcional: conexión y autenticación

El servidor usa PostgreSQL. Para desarrollo local, inicia Docker y ejecuta desde la raíz:

```bash
docker compose -f server/compose.yaml up -d
test -f server/.env || cp server/.env.example server/.env
```

Genera un secreto privado de al menos 32 bytes con `python3 -c 'import secrets; print(secrets.token_hex(32))'` y colócalo en `JWT_SECRET` dentro de `server/.env`. Las credenciales incluidas son exclusivamente para el contenedor local. Carga el archivo en la terminal e inicia el servidor:

```bash
set -a
source server/.env
set +a
cargo run --manifest-path server/Cargo.toml
```

Por defecto, los tokens emitidos por el servidor duran **60 minutos**. Para demostrar el vencimiento en un minuto, inicia el servidor con esta opción después de cargar `server/.env`:

```bash
TOKEN_DEMO_1_MINUTO=1 cargo run --manifest-path server/Cargo.toml
```

Inicia una sesión nueva con ese servidor: la duración se fija cuando se emite el token. Para volver al comportamiento normal, detén el servidor y ejecútalo sin la opción; los tokens ya emitidos conservan su vencimiento original.

En **Fish**, desde la raíz, carga el mismo archivo y ejecuta el servidor así:

```fish
bash -c 'set -a; source server/.env; set +a; exec cargo run --manifest-path server/Cargo.toml'
```

La base de pruebas `cuentas_claras_pruebas` se crea junto con la base principal. Los datos persisten en un volumen Docker. El archivo `.env` no se sube a Git. Para un despliegue futuro se puede configurar PostgreSQL administrado (ver [ADR 003](docs/adr/003-base-datos-postgresql-neon.md)).

El servidor crea la tabla de usuarios y su índice único al iniciar; la conexión debe tener permisos para ello. Usar un secreto aleatorio real, no el ejemplo. Para Neon, usar la URL de conexión segura proporcionada por el proveedor. En un despliegue se requiere HTTPS mediante un proxy TLS; el servidor escucha solo en `127.0.0.1:8000`.

Las rutas de autenticación y funciones compartidas están documentadas en [openapi.yaml](openapi.yaml). Para ejecutar las pruebas del servidor y las de PostgreSQL, desde la raíz y después de cargar `server/.env`:

```bash
cargo test --manifest-path server/Cargo.toml
cargo test --manifest-path server/Cargo.toml -- --ignored --test-threads=1
```

La prueba de registro y sesión se ejecuta en la suite normal y requiere PostgreSQL. Ella y las pruebas ignoradas requieren que `E5_TEST_DATABASE_URL` apunte a una base PostgreSQL dedicada de pruebas; crean y eliminan datos de prueba. No usar una base con datos personales. El servidor aplica las migraciones al iniciar.

En la aplicación Tauri, **Amigos y grupos** permite gestionar amistades, invitaciones y gastos compartidos. El servidor verifica la sesión, la membresía y la división; el libro personal permanece en el dispositivo. La sesión actual se conserva en memoria hasta cerrar la aplicación o sesión.

Comprobar desde cualquier terminal:

```bash
curl http://127.0.0.1:8000/ping
```

En la aplicación, abrir **Conexión con el servidor · Entrega 1 → Probar conexión**. El ping muestra estado, mensaje, versión, timestamp y latencia. Apagar Axum no impide usar las cuentas ni los movimientos.

### Interfaz web y build estático

`npm run dev`, desde `app/`, permite trabajar en la interfaz en `http://localhost:3000`. Abrirla solo en el navegador no permite comprobar los comandos Tauri ni SQLite local.

`npm run build`, desde `app/`, compila y comprueba TypeScript, y genera el frontend estático en `app/out/` (ruta desde la raíz). Este comando no genera el ejecutable de escritorio.

## Pruebas y demostración

La [guía de pruebas](docs/pruebas/README.md) reúne los comandos, qué comprueba cada caso, resultados esperados, ubicación de SQLite y procedimientos manuales de las Entregas 2, 3 y 4. Incluye los ejemplos de **1750 USD** y **316.100 VES → tasa faltante → 350 USD**, persistencia, categorías, advertencia de saldo negativo y prueba offline con el ejecutable.

Las pruebas de dominio usan un repositorio en memoria. Las de persistencia y migración usan bases SQLite temporales; no reemplazan la base personal. Pasar las pruebas automáticas o compilar el ejecutable no certifica las interacciones de la ventana: **la demostración manual sigue pendiente hasta ejecutarse y registrarse**.

El archivo personal `cuentas_claras.sqlite` reside en `app_data_dir` de Tauri. La actualización desde el identificador antiguo conserva la base anterior y no sobrescribe un destino existente; ver la [guía de SQLite](docs/pruebas/README.md#6-sqlite-y-actualización-de-datos-existentes) y el [ADR 007](docs/adr/007-libro-local-compatible.md). No se deben borrar ni editar eventos para preparar las pruebas.

El [informe de verificación de las Entregas 2 y 3](docs/entregas-2-3-verificacion.md) conserva el historial de la integración y los resultados registrados en ese momento. El [informe del backend de Entrega 4](docs/entrega-4-backend.md) documenta requisitos, contratos Tauri, persistencia, consolidación, precisión decimal y verificaciones de tasas y patrimonio.

### Pruebas del backend

Desde la raíz del repositorio, ejecutar toda la suite Rust o filtrar los casos de tasas de cambio y patrimonio:

```bash
cargo test --manifest-path app/src-tauri/Cargo.toml
cargo test --manifest-path app/src-tauri/Cargo.toml pruebas_tasas_cambio
cargo test --manifest-path app/src-tauri/Cargo.toml pruebas_patrimonio
```

Resultado esperado: **51 pruebas unitarias y 1 doctest** en la suite completa;
el filtro `pruebas_tasas_cambio` ejecuta **8 pruebas** de validaciones, historial, anulación,
precisión, consolidación y reapertura de SQLite. La suite completa también
incluye cuentas y movimientos con monedas propias y las regresiones de las
Entregas 2 y 3. El filtro `pruebas_patrimonio` ejecuta **6 regresiones** de
gastos, ajustes históricos, precisión, contrato JSON y persistencia SQLite.
La [verificación de integración del backend](docs/entrega-4-integracion-backend.md)
detalla la resolución de conflictos y las comprobaciones actuales.

## Formato

Desde `app/`, `npm run format` aplica Prettier y formato Rust a ambos paquetes. `npm run format:check` verifica sin modificar archivos y `npm run lint` ejecuta ESLint. La secuencia completa de comprobaciones está en la guía de pruebas.

## Estudiantes

- Maikel Delgado — `16-10287`
- Gabriel Orejarena — `18-10292` [@Arnold-Wesker](https://github.com/Arnold-Wesker)
- Keyber Sequera — `16-11120` [@keybersequera8](https://github.com/keybersequera8)
- Gabriel De Ornelas — `15-10377` [@gabodornelas](https://github.com/gabodornelas)
- Elías El Jaovich — `18-10641` [@ElJaovich](https://github.com/ElJaovich)
- Luis Isea — `19-10175` [@lmisea](https://github.com/lmisea)

## Licencia

Este proyecto está distribuido bajo la licencia [MIT](LICENSE).
