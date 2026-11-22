export function claveCacheGrupos(usuarioId: string, grupoId?: string): string {
  const base = `cuentas-claras:grupos:${usuarioId}`;
  return grupoId ? `${base}:detalle:${grupoId}` : base;
}

export function guardarCacheSocial(
  obtenerAlmacen: () => Storage,
  clave: string,
  valor: unknown,
): void {
  try {
    obtenerAlmacen().setItem(clave, JSON.stringify(valor));
  } catch {
    // La funcionalidad en línea sigue disponible si no se puede guardar la copia.
  }
}

export function leerCacheSocial<T>(
  obtenerAlmacen: () => Storage,
  clave: string,
): T | null {
  try {
    return JSON.parse(obtenerAlmacen().getItem(clave) ?? "null") as T | null;
  } catch {
    return null;
  }
}
