# Formato de Exportación y Reconstrucción del Libro (RNF-04)

Este documento describe el formato de archivo utilizado por **Cuentas Claras** al exportar copias de seguridad del libro personal (tanto sin cifrar como cifradas) y en los lotes de sincronización entre dispositivos propios.

El objetivo es que cualquier persona pueda leer sus datos y **reconstruir exactamente sus cuentas, movimientos, tasas y saldos sin necesidad de la aplicación** (RNF-04).

---

## 1. Principios de Privacidad y Determinismo

- El archivo exportado contiene **únicamente eventos inmutables del libro personal**.
- **Nunca** incluye contraseñas, tokens de sesión, credenciales de Git ni la frase de paso (RF-INT-05).
- Cada evento posee:
  - `id`: identificador único (UUID).
  - `tipo`: nombre del tipo de evento.
  - `momento_registro`: fecha y hora UTC en formato RFC 3339 (ISO 8601).
  - `dispositivo_origen`: identificador único de la instalación que lo originó (o `"servidor"` para partes de gastos de grupo derivadas del servidor).
  - `datos`: objeto JSON con la carga útil específica del tipo de evento.
- **Orden determinista de proyección**: para derivar los saldos, los eventos se deduplican por `id` y se ordenan ascendentemente por la tupla:
  1. `momento_registro` (instante UTC)
  2. `dispositivo_origen` (orden lexicográfico)
  3. `id` (orden lexicográfico)

---

## 2. Formato en Claro (NDJSON — JSON Lines)

Cuando se exporta sin cifrar (o una vez descifrado un respaldo cifrado), el contenido es texto UTF-8 estructurado en **NDJSON** (un objeto JSON válido por línea).

### Línea 1: Encabezado

```json
{"formato":"cuentas-claras-libro","version":1,"fecha_exportacion":"2026-11-26T22:00:00Z","cantidad_eventos":42}
```

| Campo | Tipo | Descripción |
|---|---|---|
| `formato` | `string` | Siempre `"cuentas-claras-libro"`. |
| `version` | `number` | Versión entera del esquema. Versión actual soportada: `1`. |
| `fecha_exportacion` | `string` | Marca temporal RFC 3339 UTC en la que se generó el archivo. |
| `cantidad_eventos` | `number` | Número exacto de líneas de eventos que siguen al encabezado. |

### Líneas 2..N+1: Eventos

Cada línea posterior representa un evento inmutable:

```json
{"id":"...","tipo":"CuentaCreada","momento_registro":"2026-11-20T10:00:00Z","dispositivo_origen":"8f9c...","datos":{...}}
```

---

## 3. Catálogo de Tipos de Evento

Todos los montos y tasas se serializan como **cadenas decimales exactas** (por ejemplo `"151.67"`), nunca como números de punto flotante (`float`).

### 3.1. `CuentaCreada`
Registra una cuenta personal nueva. Su moneda es inmutable.

```json
{
  "cuenta_id": "uuid-cuenta",
  "nombre": "Efectivo USD",
  "tipo": "efectivo",
  "saldo_inicial": { "cantidad": "151.67", "moneda": "USD" }
}
```
- `tipo`: `"efectivo"`, `"banco"` o `"billetera digital"`.
- Efecto: crea la cuenta con `saldo = saldo_inicial.cantidad`.

### 3.2. `CategoriaCreada`
Registra una categoría personalizada adicional a las de fábrica.

```json
{
  "nombre": "Mascotas",
  "tipo": "gasto"
}
```
- `tipo`: `"gasto"` o `"ingreso"`.

### 3.3. `MovimientoRegistrado`
Registra un ingreso o un gasto sobre una cuenta existente.

```json
{
  "movimiento_id": "uuid-movimiento",
  "cuenta_id": "uuid-cuenta",
  "tipo": "gasto",
  "monto": { "cantidad": "20.00", "moneda": "USD" },
  "fecha": "2026-11-24",
  "categoria": "Alimentación",
  "nota": "Almuerzo"
}
```
- Efecto sobre la cuenta `cuenta_id`:
  - Si `tipo == "ingreso"`: `saldo += monto.cantidad`.
  - Si `tipo == "gasto"`: `saldo -= monto.cantidad`.

### 3.4. `MovimientoCorregido`
Corrige el monto de un movimiento previo mediante un ajuste inmutable (nunca borra ni edita el original).

```json
{
  "ajuste_id": "uuid-ajuste",
  "movimiento_original_id": "uuid-movimiento",
  "cuenta_id": "uuid-cuenta",
  "monto_correcto": { "cantidad": "25.00", "moneda": "USD" },
  "diferencia": "-5.00",
  "fecha": "2026-11-24",
  "nota": "Corrección de movimiento"
}
```
- `diferencia`: cadena decimal con signo que indica el efecto neto sobre el saldo (`+` incrementa, `-` disminuye).
- Efecto sobre `cuenta_id`: `saldo += diferencia`.

### 3.5. `GastoDeGrupoAplicado`
Registra la parte personal de un gasto compartido descargado de un grupo. Su `id` de evento y `movimiento_id` se derivan determinísticamente de `evento_servidor_id`.

```json
{
  "movimiento_id": "uuid-derivado-mov",
  "cuenta_id": "uuid-cuenta",
  "evento_servidor_id": "uuid-evento-servidor",
  "grupo_id": "uuid-grupo",
  "gasto_id": "uuid-gasto",
  "concepto": "Viaje · Hotel",
  "fecha": "2026-11-20",
  "monto": { "cantidad": "33.33", "moneda": "USD" },
  "incluye_centimos_redondeo": false
}
```
- Efecto sobre `cuenta_id`: `saldo -= monto.cantidad`.

### 3.6. `MonedaCreada`
Agrega una moneda propia al catálogo base (`USD`, `VES`, `EUR`, `USDT`).

```json
{
  "codigo": "BTC",
  "nombre": "Bitcoin",
  "simbolo": "₿",
  "decimales": 8
}
```

### 3.7. `TasaRegistrada`
Registra una tasa de cambio con fecha y fuente.

```json
{
  "id": "uuid-tasa",
  "origen": "USD",
  "destino": "VES",
  "valor": "45.50",
  "fecha": "2026-11-24",
  "fuente": "BCV"
}
```

### 3.8. `TasaEliminada`
Anula una tasa previamente registrada.

```json
{
  "tasa_id": "uuid-tasa"
}
```

### 3.9. `MonedaBaseCambiada`
Cambia la moneda base usada para consolidar el patrimonio.

```json
{
  "codigo": "EUR"
}
```

---

## 4. Formato Cifrado (`.ccbackup` y lotes `.ccsync`)

Cuando el archivo está cifrado, es un documento JSON con el siguiente envoltorio:

```json
{
  "formato": "cuentas-claras-respaldo-cifrado",
  "version": 1,
  "kdf": {
    "algoritmo": "argon2id",
    "memoria_kib": 65536,
    "iteraciones": 3,
    "paralelismo": 1
  },
  "cifrado": "aes-256-gcm",
  "sal": "<base64 de 16 bytes>",
  "nonce": "<base64 de 12 bytes>",
  "datos": "<base64 de texto_cifrado || tag_gcm_16_bytes>"
}
```

- La parte en claro **no revela ningún dato del libro** (ni cantidad de eventos, ni nombres, ni fechas).
- La clave de 32 bytes se deriva con **Argon2id v0x13** usando `sal` y los parámetros `kdf`.
- El texto plano autenticado es exactamente el NDJSON de la Sección 2, cifrado con **AES-256-GCM** usando `nonce` y el siguiente AAD UTF-8:
  ```text
  cuentas-claras-respaldo-cifrado|v1|argon2id|65536|3|1|aes-256-gcm|<sal_b64>|<nonce_b64>
  ```

---

## 5. Cómo Reconstruir Saldos sin la Aplicación

1. Si el archivo está cifrado, descifra `datos` con Argon2id + AES-256-GCM usando los parámetros del envoltorio y el AAD anterior.
2. Verifica la línea 1 (`formato == "cuentas-claras-libro"` y `version == 1`).
3. Lee las líneas 2..N+1, deduplica por `id` y ordena por `(momento_registro, dispositivo_origen, id)`.
4. Usa aritmética decimal exacta (p. ej. `BigInt` escalado a `10^8` o `Decimal` de Python) aplicando:
   - `CuentaCreada` → `saldo = saldo_inicial`
   - `MovimientoRegistrado` → `+monto` si ingreso, `-monto` si gasto
   - `MovimientoCorregido` → `+diferencia`
   - `GastoDeGrupoAplicado` → `-monto`

El repositorio incluye una herramienta de referencia independiente en `tools/reconstruir_saldos.mjs`:

```bash
node tools/reconstruir_saldos.mjs ruta/al/respaldo.ndjson
```
