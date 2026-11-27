# ADR 014: Cifrado Autenticado para Respaldos y Sincronización Personal

- **Estado**: Aceptada
- **Fecha**: 2026-11-26
- **Requerimientos relacionados**: RF-INT-01 a RF-INT-07, RNF-01, RNF-04, RNF-05, RNF-10, RNF-14, RNF-15

## Contexto

La Entrega 8 permite exportar copias de seguridad del libro de eventos local a un archivo controlado por el usuario y sincronizar el libro entre varios dispositivos propios mediante un repositorio Git o una carpeta compartida (Syncthing, Dropbox, Google Drive), sin que el servidor de Cuentas Claras participe ni reciba datos financieros.

Dado que los archivos de copia de seguridad y los lotes publicados en un repositorio Git o carpeta en la nube pueden almacenarse fuera del dispositivo, es necesario garantizar:
1. **Confidencialidad**: quien acceda al archivo o al repositorio no puede leer ningún dato del libro (ni siquiera cuántos eventos contiene).
2. **Integridad y autenticidad**: cualquier frase de paso incorrecta o alteración de un solo bit (en los datos cifrados o en el encabezado) debe detectarse de inmediato antes de modificar el libro local.
3. **Resistencia a fuerza bruta**: la derivación de clave a partir de una frase de paso elegida por el usuario (mínimo 12 caracteres) debe ser costosa en memoria y CPU.
4. **Cero secretos en texto plano**: ni la frase de paso ni las credenciales de Git deben guardarse en SQLite, en las exportaciones ni en el destino.

## Decisión

### 1. Derivación de clave: Argon2id (v0x13)
Se elige **Argon2id** (`argon2 = "0.5"`) con los siguientes parámetros por defecto en producción:
- **Memoria (`m_cost`)**: `65536` KiB (64 MiB)
- **Iteraciones (`t_cost`)**: `3`
- **Paralelismo (`p_cost`)**: `1`
- **Longitud de clave**: `32` bytes (256 bits)
- **Sal aleatoria**: `16` bytes generados con `ring::rand::SystemRandom`, nueva en cada exportación y en cada lote de sincronización.

Los parámetros de KDF se guardan en el encabezado en claro del envoltorio para garantizar compatibilidad futura si se endurecen en nuevas versiones, y al descifrar se acotan dentro de rangos seguros (`1 MiB..512 MiB`, `1..20` iteraciones) para evitar agotamiento de memoria ante archivos manipulados.

### 2. Cifrado autenticado: AES-256-GCM con AAD
Se elige **AES-256-GCM** (`ring::aead::AES_256_GCM`) con:
- **Nonce aleatorio**: `12` bytes (96 bits) generados con `ring::rand::SystemRandom`, único por cada archivo/lote (al derivar una clave distinta por cada sal aleatoria, no existe riesgo de reutilización de par clave-nonce).
- **Datos Asociados Autenticados (AAD)**: se autentica la representación canónica de todos los campos en claro del envoltorio (`formato`, `version`, algoritmo y parámetros KDF, algoritmo de cifrado, `sal` y `nonce`). Así, cualquier modificación del encabezado invalida la etiqueta GCM.
- **Limpieza en memoria**: las claves derivadas de 32 bytes se envuelven en `zeroize::Zeroizing` para borrarlas al salir de ámbito.

### 3. Almacén de secretos del sistema operativo
La frase de paso de sincronización y el token/contraseña de Git se almacenan exclusivamente mediante el crate `keyring` en el almacén nativo del sistema operativo:
- **Linux**: Secret Service (D-Bus / GNOME Keyring / KWallet)
- **macOS**: Keychain
- **Windows**: Windows Credential Manager

Para el transporte Git, las credenciales se inyectan únicamente en tiempo de ejecución mediante un script temporal efímero `GIT_ASKPASS` (permisos `0700` en Unix) y variables de entorno del proceso hijo que se eliminan inmediatamente al terminar cada operación, evitando escribir secretos en `.git/config` o en la URL remota.

## Alternativas Consideradas

- **PBKDF2-HMAC-SHA256 / scrypt**: PBKDF2 carece de dureza frente a memoria (vulnerable a GPU/ASIC); Argon2id es el ganador de PHC y la recomendación actual de OWASP (combina resistencia contra ataques de canal lateral de Argon2i y resistencia GPU de Argon2d).
- **XChaCha20-Poly1305**: excelente alternativa, igualmente segura. Se eligió **AES-256-GCM** mediante `ring` porque `ring` ya forma parte de las dependencias verificadas del binario Tauri (`rustls-tls`), minimiza la superficie de dependencias criptográficas, cuenta con aceleración AES-NI en hardware moderno y no presenta problemas de nonce corto porque cada exportación/lote deriva una clave nueva con una sal fresca de 16 bytes.
- **Guardar secretos cifrados en SQLite**: descartado por RF-INT-05; delegar al llavero del sistema operativo evita almacenar material sensible junto al libro.

## Consecuencias

- **Positivas**:
  - Confidencialidad e integridad fuertes con estándares auditados (Argon2id + AES-256-GCM + AAD).
  - La parte en claro del archivo cifrado no filtra metadatos del libro (RF-INT-06).
  - El mismo envoltorio sirve tanto para respaldos manuales (`.ccbackup`) como para lotes de sincronización (`.ccsync`), simplificando el mantenimiento.
- **Trade-offs**:
  - Cada exportación/lote tarda una fracción de segundo debido a Argon2id (64 MiB), por lo que las operaciones se ejecutan en hilos bloqueantes (`tokio::task::spawn_blocking`) para no congelar la interfaz.
  - Si el usuario olvida la frase de paso, el respaldo es irrecuperable por diseño; la interfaz muestra una advertencia explícita antes de exportar y exige confirmar la frase dos veces.
