# Guía de Usuario: Cómo sincronizar con Git o con una carpeta compartida

Esta guía explica cómo respaldar tu libro financiero de **Cuentas Claras** y mantenerlo sincronizado entre varios dispositivos propios (por ejemplo, tu laptop y otra computadora) usando **una carpeta compartida** o **un repositorio Git privado**.

> **Privacidad garantizada:** El servidor de Cuentas Claras no participa en esta sincronización. Tus datos viajan siempre **cifrados de extremo a extremo** con tu frase de paso y solo hacia el destino que tú eliges y controlas.

---

## Antes de empezar: tu frase de paso

Para cifrar tus copias de seguridad y tus lotes de sincronización elegirás una **frase de paso de al menos 12 caracteres** (por ejemplo, cuatro o cinco palabras fáciles de recordar para ti).

- **Todos tus dispositivos deben usar la misma frase de paso** para poder descifrar los lotes de los demás.
- **No la pierdas:** Cuentas Claras no guarda tu frase en ningún servidor y no puede recuperarla si la olvidas.
- Tu frase de paso (y tus credenciales de Git, si usas Git) se guardan únicamente en el **almacén de secretos de tu sistema operativo** (GNOME Keyring/KWallet en Linux, Keychain en macOS, Administrador de Credenciales en Windows).

---

## Opción 1: Sincronizar con una carpeta compartida (Syncthing, Dropbox, Google Drive)

Es la forma más sencilla si ya sincronizas una carpeta entre tus equipos.

### En tu primer dispositivo
1. Crea una carpeta vacía dentro de tu carpeta sincronizada (por ejemplo: `~/Dropbox/cuentas-claras-sync` o `~/Sync/cuentas-claras`).
2. Abre **Cuentas Claras** y entra en **Respaldo y Sync** desde la barra lateral (o la barra inferior en móvil).
3. En la sección **Sincronización entre mis dispositivos**:
   - Selecciona **Carpeta compartida**.
   - Escribe la ruta completa de la carpeta que creaste.
   - Escribe tu **frase de paso** (mínimo 12 caracteres) y guarda la configuración.
4. Pulsa **Sincronizar ahora**. Verás el informe con la cantidad de eventos enviados.

### En tu segundo dispositivo
1. Espera a que Syncthing / Dropbox / Google Drive termine de bajar la carpeta compartida.
2. Abre **Cuentas Claras** → **Respaldo y Sync**.
3. Configura **Carpeta compartida** con la ruta de esa misma carpeta en el segundo equipo y escribe **la misma frase de paso**.
4. Pulsa **Sincronizar ahora**: la aplicación descargará y restaurará todos tus eventos, cuentas, movimientos y tasas automáticamente.

> Cada dispositivo escribe únicamente dentro de su propia subcarpeta (`dispositivos/<id-dispositivo>/`), usando archivos inmutables escritos de forma atómica, por lo que los servicios de nube nunca generan "copias en conflicto".

---

## Opción 2: Sincronizar con un repositorio Git privado

Ideal si prefieres controlar el historial en un repositorio privado de GitHub, GitLab, Codeberg o tu propio servidor Git.

### Requisitos
- Tener **Git** instalado en tus dispositivos (`git --version`).
- Crear un **repositorio privado vacío** (por ejemplo, `https://github.com/tu-usuario/mi-libro-cuentas-claras.git`).
- Si usas HTTPS con GitHub o GitLab, genera un **Personal Access Token (PAT)** con permiso de lectura y escritura únicamente sobre ese repositorio.

### En tu primer dispositivo
1. Abre **Cuentas Claras** → **Respaldo y Sync**.
2. En **Sincronización entre mis dispositivos**, selecciona **Repositorio Git**.
3. Completa los campos:
   - **Dirección del repositorio (URL):** `https://github.com/tu-usuario/mi-libro-cuentas-claras.git` *(nunca escribas el usuario ni el token dentro de la URL)*.
   - **Rama:** `main` (o la rama que prefieras).
   - **Usuario de Git:** tu nombre de usuario (si el repositorio requiere autenticación HTTPS).
   - **Token / Contraseña de Git:** tu PAT (se guarda en el almacén de secretos del SO, nunca en `.git/config` ni en la base de datos).
   - **Frase de paso de cifrado:** mínimo 12 caracteres.
4. Pulsa **Guardar configuración** y luego **Sincronizar ahora**.

### En tu segundo dispositivo
1. Repite la misma configuración apuntando al **mismo repositorio**, la **misma rama** y la **misma frase de paso**.
2. Pulsa **Sincronizar ahora**. Al abrir la aplicación de ahí en adelante, se sincronizará automáticamente si el destino está configurado, y siempre podrás pulsar **Sincronizar ahora** cuando quieras.

---

## Copias de seguridad manuales (Exportar e Importar)

Además de la sincronización continua, desde **Respaldo y Sync** puedes:

- **Exportar copia de seguridad cifrada (`.ccbackup`):** ingresa tu frase de paso dos veces. El archivo resultante no revela ningún dato si se abre en un editor de texto.
- **Exportar sin cifrar (`.ndjson`):** opción avanzada con advertencia de seguridad, pensada para auditar tus datos o reconstruir saldos fuera de la aplicación (ver [formato-exportacion.md](formato-exportacion.md)).
- **Importar copia de seguridad:** selecciona cualquier respaldo previo (cifrado o en claro). Importar el mismo archivo varias veces es seguro e idempotente, e importar un archivo antiguo nunca borra movimientos locales recientes.

---

## Solución de problemas frecuentes

| Mensaje / Situación | Qué significa y cómo resolverlo |
|---|---|
| **“Frase de paso incorrecta o archivo dañado.”** | La frase no coincide con la usada al cifrar o el archivo fue modificado. Verifica mayúsculas/espacios; tu libro local no se modificó. |
| **Versión de formato no soportada** | El archivo fue creado por una versión más nueva de Cuentas Claras. Actualiza la aplicación e inténtalo de nuevo. |
| **No se pudo llegar al repositorio o a la carpeta; se reintentará.** | Estás sin conexión, la carpeta externa está desmontada o la URL es incorrecta. Tus datos locales siguen seguros y se enviarán en el próximo intento. |
| **Las credenciales de Git fueron rechazadas** | Tu token expiró o no tiene permisos de escritura. Genera un token nuevo y actualízalo en la configuración de sincronización. |
| **Evento rechazado por identificador repetido con contenido distinto** | Un evento recibido tenía el mismo ID que uno local pero distinto contenido. Cuentas Claras protege el evento local y aplica normalmente el resto de eventos válidos. |

---

## Demostración con dos instancias en la misma computadora

Si deseas probar la sincronización entre dos instalaciones independientes en el mismo equipo, puedes lanzar cada instancia con su propia carpeta de datos usando la variable de entorno `CUENTAS_CLARAS_DATA_DIR`:

```bash
# Instancia 1 (por ejemplo, "Laptop")
CUENTAS_CLARAS_DATA_DIR=/tmp/cuentas-instancia-1 npm run tauri dev

# Instancia 2 (por ejemplo, "Teléfono")
CUENTAS_CLARAS_DATA_DIR=/tmp/cuentas-instancia-2 npm run tauri dev
```
